# /// script
# requires-python = ">=3.11"
# dependencies = ["lancedb", "model2vec", "numpy", "pyarrow"]
# ///
"""
RAG helper for the `gemma` script.

Two modes:
  * single-doc: reads document text on stdin (keyed by --cache-key).
  * directory : --dir PATH walks a directory (optionally --recursive),
                extracts every supported file, and ingests them into one
                LanceDB table, re-embedding only files that changed.

In both modes it chunks text, embeds with a static model2vec embedder, stores
the vectors in a persistent LanceDB table, then retrieves the top-k chunks most
relevant to --query and prints them to stdout (directory hits are labeled with
their source file).
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

EMBED_MODEL = "minishlab/potion-base-8M"
USAGE_FILE = ".usage.json"

# Files read directly as text.
TEXT_EXTS = {
    ".txt", ".text", ".csv", ".tsv", ".md", ".markdown",
    ".log", ".json", ".yaml", ".yml", ".rst",
}
# Files extracted via liteparse (needs LibreOffice for office formats).
DOC_EXTS = {
    ".pdf", ".docx", ".doc", ".xlsx", ".xls", ".pptx", ".ppt",
    ".png", ".jpg", ".jpeg", ".tiff", ".tif", ".bmp", ".gif", ".webp",
}


def extract_file(path):
    """Return text for a supported file, or None if the type is unsupported."""
    ext = os.path.splitext(path)[1].lower()
    if ext in TEXT_EXTS:
        try:
            with open(path, encoding="utf-8", errors="ignore") as fh:
                return fh.read()
        except OSError:
            return ""
    if ext in DOC_EXTS:
        uvx = shutil.which("uvx") or "uvx"
        try:
            r = subprocess.run(
                [uvx, "--from", "liteparse", "lit", "parse", "--quiet", path],
                capture_output=True, text=True,
            )
            return r.stdout if r.returncode == 0 else ""
        except Exception:
            return ""
    return None


def file_sig(path):
    st = os.stat(path)
    return f"{int(st.st_mtime)}-{st.st_size}"


def walk_files(root, recursive):
    """Yield non-hidden file paths under root."""
    if recursive:
        for dirpath, dirs, files in os.walk(root):
            dirs[:] = [d for d in dirs if not d.startswith(".")]
            for f in sorted(files):
                if not f.startswith("."):
                    yield os.path.join(dirpath, f)
    else:
        for f in sorted(os.listdir(root)):
            p = os.path.join(root, f)
            if os.path.isfile(p) and not f.startswith("."):
                yield p


def prune_expired(db, db_path, existing, ttl, now):
    """Drop tables not used within `ttl` seconds; return (usage, live tables)."""
    usage_path = os.path.join(db_path, USAGE_FILE)
    usage = {}
    if os.path.exists(usage_path):
        try:
            with open(usage_path) as fh:
                usage = json.load(fh)
        except (ValueError, OSError):
            usage = {}

    # Seed any tables that predate usage tracking with their on-disk mtime.
    for t in existing:
        if t not in usage:
            try:
                usage[t] = os.path.getmtime(os.path.join(db_path, t + ".lance"))
            except OSError:
                usage[t] = now

    live = list(existing)
    for t, last in list(usage.items()):
        if now - last > ttl:
            if t in live:
                try:
                    db.drop_table(t)
                except Exception:
                    pass
                live.remove(t)
            usage.pop(t, None)
    return usage, live, usage_path


def save_usage(usage, usage_path):
    try:
        with open(usage_path, "w") as fh:
            json.dump(usage, fh)
    except OSError:
        pass


def chunk_text(text, max_chars=1000, overlap=150):
    """Split text into overlapping chunks on paragraph/line boundaries."""
    # Prefer splitting on blank lines, fall back to single newlines.
    paras = [p.strip() for p in re.split(r"\n\s*\n", text) if p.strip()]
    if not paras:
        paras = [ln for ln in text.splitlines() if ln.strip()]
    chunks = []
    buf = ""
    for p in paras:
        if len(buf) + len(p) + 1 <= max_chars:
            buf = f"{buf}\n{p}" if buf else p
        else:
            if buf:
                chunks.append(buf)
            if len(p) <= max_chars:
                buf = p
            else:
                # Hard-split an oversized paragraph (e.g. a wide table row).
                for i in range(0, len(p), max_chars - overlap):
                    chunks.append(p[i : i + max_chars])
                buf = ""
    if buf:
        chunks.append(buf)
    return chunks or [text[:max_chars]]


def table_name(cache_key):
    return "doc_" + hashlib.sha256(cache_key.encode()).hexdigest()[:16]


def dir_table_name(root):
    return "dir_" + hashlib.sha256(os.path.abspath(root).encode()).hexdigest()[:16]


def _sql_quote(s):
    return s.replace("'", "''")


def ingest_dir(db, name, root, recursive, model, existing):
    """Build or incrementally update a directory's table; return the table.

    Re-embeds only files that are new or whose size+mtime changed, and drops
    rows for files that were deleted or modified. Returns None if no supported
    files are present.
    """
    root = os.path.abspath(root)

    # Map of relative path -> (abs path, signature) for supported files only.
    current = {}
    for p in walk_files(root, recursive):
        ext = os.path.splitext(p)[1].lower()
        if ext in TEXT_EXTS or ext in DOC_EXTS:
            current[os.path.relpath(p, root)] = (p, file_sig(p))

    def chunks_for(rel, abspath, sig):
        rows = []
        for c in chunk_text(extract_file(abspath) or ""):
            rows.append({"source": rel, "sig": sig, "text": c})
        return rows

    if name not in existing:
        rows = []
        for rel, (p, sig) in current.items():
            rows.extend(chunks_for(rel, p, sig))
        if not rows:
            return None, (0, 0, 0)
        vecs = model.encode([r["text"] for r in rows])
        data = [{**rows[i], "vector": vecs[i].tolist()} for i in range(len(rows))]
        table = db.create_table(name, data=data)
        return table, (len(current), len(current), 0)

    table = db.open_table(name)
    arrow = table.to_arrow()
    existing_sig = {}
    for s, g in zip(arrow.column("source").to_pylist(), arrow.column("sig").to_pylist()):
        existing_sig[s] = g

    changed = [rel for rel, (_, sig) in current.items() if existing_sig.get(rel) != sig]
    removed = [s for s in existing_sig if s not in current]

    for s in set(changed) | set(removed):
        table.delete(f"source = '{_sql_quote(s)}'")

    add_rows = []
    for rel in changed:
        p, sig = current[rel]
        add_rows.extend(chunks_for(rel, p, sig))
    if add_rows:
        vecs = model.encode([r["text"] for r in add_rows])
        table.add([{**add_rows[i], "vector": vecs[i].tolist()} for i in range(len(add_rows))])

    return table, (len(current), len(changed), len(removed))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--cache-key", help="required for single-doc (stdin) mode")
    ap.add_argument("--dir", help="ingest all supported files in this directory")
    ap.add_argument("--recursive", action="store_true",
                    help="with --dir, descend into subdirectories")
    ap.add_argument("--query", required=True)
    ap.add_argument("--top-k", type=int, default=6)
    ap.add_argument("--ttl", type=int, default=86400,
                    help="evict cached tables idle longer than this many seconds")
    args = ap.parse_args()

    if not args.dir and not args.cache_key:
        ap.error("either --dir or --cache-key is required")

    import lancedb
    from model2vec import StaticModel

    model = StaticModel.from_pretrained(EMBED_MODEL)
    db = lancedb.connect(args.db)
    name = dir_table_name(args.dir) if args.dir else table_name(args.cache_key)

    # table_names() returns a plain list of names and works in both 0.30.x and
    # 0.33.x. (list_tables() in 0.33 returns a paginated object, not a list.)
    import warnings
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        existing = db.table_names()

    now = time.time()
    usage, existing, usage_path = prune_expired(db, args.db, existing, args.ttl, now)

    if args.dir:
        table, (total, added, removed) = ingest_dir(
            db, name, args.dir, args.recursive, model, existing)
        if table is None:
            sys.stderr.write("No supported files found in directory.\n")
            sys.exit(2)
        sys.stderr.write(
            f"Indexed {total} file(s) in directory "
            f"({added} new/changed, {removed} removed).\n")
    elif name in existing:
        table = db.open_table(name)
    else:
        text = sys.stdin.read()
        chunks = chunk_text(text)
        vectors = model.encode(chunks)
        rows = [
            {"id": i, "text": c, "vector": vectors[i].tolist()}
            for i, c in enumerate(chunks)
        ]
        table = db.create_table(name, data=rows)

    usage[name] = now  # refresh last-use time for the table we touched
    save_usage(usage, usage_path)

    qvec = model.encode([args.query])[0].tolist()
    hits = table.search(qvec).limit(args.top_k).to_list()
    parts = []
    for h in hits:
        src = h.get("source")
        parts.append(f"[source: {src}]\n{h['text']}" if src else h["text"])
    sys.stdout.write("\n\n---\n\n".join(parts))


if __name__ == "__main__":
    main()
