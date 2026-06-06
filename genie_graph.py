# /// script
# requires-python = ">=3.11"
# dependencies = ["ladybug"]
# ///
"""
Graph-correlation helper for the `gemma` CLI, backed by LadybugDB.

The graph is updated automatically as a side effect of analyzing files with
`gemma --txt/--doc/--dir` (just like the LanceDB vector cache). It stores:

    (:File {path, sig, name})  -[:Mentions {count}]->  (:Entity {name})

Entities are extracted heuristically (capitalized phrases, acronyms, frequent
terms). The graph is a single local .lbug file and is cleared after the TTL.

Modes:
  update  --dir PATH [--recursive]        incrementally index a directory
  update  --file-name NAME --sig SIG      index one file (text on stdin)
  stats                                   node/edge counts
  query   --cypher "..."                  run a raw Cypher query
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time

# Reuse the file walker / extractor from the RAG helper (same directory).
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from genie_rag import extract_file, file_sig, walk_files, TEXT_EXTS, DOC_EXTS  # noqa: E402

STOPWORDS = {
    "The", "This", "That", "These", "Those", "There", "Here", "And", "But",
    "For", "With", "From", "Into", "Over", "After", "Before", "When", "While",
    "Where", "What", "Which", "Who", "Whom", "Whose", "Why", "How", "All",
    "Any", "Some", "Each", "Every", "Page", "Note", "Notes", "Total", "Section",
    "Table", "Figure", "Sl", "No", "Yes", "Date", "Name", "Description",
    "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
    "January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December",
}

CAP_PHRASE = re.compile(r"\b[A-Z][a-zA-Z0-9&'-]*(?:[ \t]+[A-Z][a-zA-Z0-9&'-]*)*\b")
ACRONYM = re.compile(r"\b[A-Z]{2,}[0-9]*\b")


def _clean(tok):
    return tok.strip(" \t.,;:!?'\"()[]-")


def extract_entities(text, max_entities=40):
    """Return {entity: count} of likely named entities in text."""
    counts = {}
    for line in text.splitlines():
        for m in CAP_PHRASE.findall(line):
            words = _clean(m).split()
            while words and words[0] in STOPWORDS:   # "The Apollo" -> "Apollo"
                words.pop(0)
            phrase = " ".join(words)
            if len(phrase) < 2:
                continue
            if " " not in phrase and phrase in STOPWORDS:
                continue
            counts[phrase] = counts.get(phrase, 0) + 1
        for a in ACRONYM.findall(line):
            if len(a) >= 2:
                counts[a] = counts.get(a, 0) + 1
    keep = {
        k: v for k, v in counts.items()
        if " " in k or k.isupper() or len(k) >= 3 or v > 1
    }
    return dict(sorted(keep.items(), key=lambda kv: -kv[1])[:max_entities])


def extract_entities_llm(text, model_path, max_entities=40, backend="gpu"):
    """Higher-quality entity extraction via the Gemma model (one call per file)."""
    uvx = shutil.which("uvx") or "uvx"
    prompt = (
        "Extract the key named entities (people, organizations, projects, "
        "products, places) from the text. Reply with ONLY a comma-separated "
        "list of entity names, nothing else.\n\n" + text[:6000]
    )
    try:
        r = subprocess.run(
            [uvx, "litert-lm", "run", model_path, f"--backend={backend}", "--prompt", prompt],
            capture_output=True, text=True,
        )
        out = r.stdout if r.returncode == 0 else ""
    except Exception:
        out = ""
    if not out.strip():
        return extract_entities(text)  # fall back to heuristic on failure
    seen, ents = set(), {}
    for n in re.split(r"[,\n]", out):
        n = _clean(n)
        if 2 <= len(n) <= 60 and n.lower() not in seen:
            seen.add(n.lower())
            ents[n] = 1
        if len(ents) >= max_entities:
            break
    return ents or extract_entities(text)


# --- persistence / TTL ------------------------------------------------------

def reset_db(db_path):
    for p in (db_path, db_path + ".wal", db_path + ".meta"):
        if os.path.isdir(p):
            shutil.rmtree(p, ignore_errors=True)
        elif os.path.exists(p):
            try:
                os.remove(p)
            except OSError:
                pass


def touch_usage(db_path):
    try:
        with open(db_path + ".meta", "w") as fh:
            json.dump({"last": time.time()}, fh)
    except OSError:
        pass


def is_expired(db_path, ttl):
    meta = db_path + ".meta"
    last = None
    if os.path.exists(meta):
        try:
            with open(meta) as fh:
                last = json.load(fh).get("last")
        except (OSError, ValueError):
            last = None
    if last is None:
        try:
            last = os.path.getmtime(db_path)
        except OSError:
            return False
    return (time.time() - last) > ttl


def connect(db_path):
    import ladybug as lb
    return lb.Connection(lb.Database(db_path))


def ensure_schema(conn):
    stmts = [
        "CREATE NODE TABLE IF NOT EXISTS File(path STRING PRIMARY KEY, sig STRING, name STRING)",
        "CREATE NODE TABLE IF NOT EXISTS Entity(name STRING PRIMARY KEY)",
        "CREATE REL TABLE IF NOT EXISTS Mentions(FROM File TO Entity, count INT64)",
    ]
    for s in stmts:
        try:
            conn.execute(s)
        except Exception:
            pass  # already exists (older engines without IF NOT EXISTS)


def _rows(conn, cypher, params=None):
    return [list(r) for r in conn.execute(cypher, params or {})]


def upsert_file(conn, path, name, sig, text, extractor=extract_entities):
    """Insert/refresh one file's entities; skip if signature is unchanged."""
    cur = _rows(conn, "MATCH (f:File {path:$p}) RETURN f.sig", {"p": path})
    if cur and cur[0][0] == sig:
        return False  # unchanged
    if cur:
        conn.execute("MATCH (f:File {path:$p})-[m:Mentions]->() DELETE m", {"p": path})
        conn.execute("MATCH (f:File {path:$p}) SET f.sig=$s, f.name=$n",
                     {"p": path, "s": sig, "n": name})
    else:
        conn.execute("CREATE (f:File {path:$p, sig:$s, name:$n})",
                     {"p": path, "s": sig, "n": name})
    for ename, cnt in extractor(text).items():
        conn.execute("MERGE (e:Entity {name:$n})", {"n": ename})
        conn.execute(
            "MATCH (f:File {path:$p}), (e:Entity {name:$n}) "
            "MERGE (f)-[m:Mentions]->(e) SET m.count=$c",
            {"p": path, "n": ename, "c": int(cnt)},
        )
    return True


def update_dir(db_path, root, recursive, extractor=extract_entities):
    root = os.path.abspath(root)
    conn = connect(db_path)
    ensure_schema(conn)
    current = {}
    for p in walk_files(root, recursive):
        if os.path.splitext(p)[1].lower() in (TEXT_EXTS | DOC_EXTS):
            current[os.path.abspath(p)] = file_sig(p)

    changed = 0
    for path, sig in current.items():
        text = extract_file(path) or ""
        if text.strip() and upsert_file(conn, path, os.path.relpath(path, root), sig, text, extractor):
            changed += 1

    # Drop files that disappeared from this directory tree.
    removed = 0
    for (p,) in _rows(conn, "MATCH (f:File) RETURN f.path"):
        if p.startswith(root + os.sep) and p not in current:
            conn.execute("MATCH (f:File {path:$p}) DETACH DELETE f", {"p": p})
            removed += 1

    touch_usage(db_path)
    sys.stderr.write(
        f"Graph updated: {len(current)} files ({changed} new/changed, {removed} removed).\n")


def update_file_stdin(db_path, name, sig, extractor=extract_entities):
    text = sys.stdin.read()
    if not text.strip():
        return
    conn = connect(db_path)
    ensure_schema(conn)
    key = os.path.abspath(name) if os.path.exists(name) else name
    upsert_file(conn, key, os.path.basename(name), sig, text, extractor)
    touch_usage(db_path)


def stats(db_path):
    conn = connect(db_path)
    ensure_schema(conn)
    nf = _rows(conn, "MATCH (f:File) RETURN count(f)")[0][0]
    ne = _rows(conn, "MATCH (e:Entity) RETURN count(e)")[0][0]
    nm = _rows(conn, "MATCH (:File)-[m:Mentions]->(:Entity) RETURN count(m)")[0][0]
    print(f"Graph: {nf} files, {ne} entities, {nm} mentions")
    print(f"DB:    {db_path}")
    if nf:
        print("\nTop correlation hubs (entities across the most files):")
        for name, files in _rows(conn,
            "MATCH (f:File)-[:Mentions]->(e:Entity) "
            "RETURN e.name, count(DISTINCT f) AS c ORDER BY c DESC, e.name LIMIT 10"):
            print(f"  {files:>3}  {name}")


def query(db_path, cypher):
    conn = connect(db_path)
    ensure_schema(conn)
    for row in conn.execute(cypher):
        print(row)


def correlate(db_path, query_text):
    """Print known correlations for entities mentioned in the query text."""
    conn = connect(db_path)
    ensure_schema(conn)
    lines = []
    for ent in extract_entities(query_text):
        files = [r[0] for r in _rows(conn,
            "MATCH (e:Entity {name:$n})<-[:Mentions]-(f:File) RETURN f.name LIMIT 20",
            {"n": ent})]
        if not files:
            continue
        co = [r[0] for r in _rows(conn,
            "MATCH (e:Entity {name:$n})<-[:Mentions]-(:File)-[:Mentions]->(e2:Entity) "
            "WHERE e2.name <> e.name "
            "RETURN e2.name, count(*) AS c ORDER BY c DESC LIMIT 8", {"n": ent})]
        line = f"- {ent}: appears in {', '.join(files)}"
        if co:
            line += f"; related entities: {', '.join(co)}"
        lines.append(line)
    if lines:
        print("Known correlations from your indexed data:")
        print("\n".join(lines))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--mode", required=True,
                    choices=["update", "stats", "query", "correlate"])
    ap.add_argument("--dir")
    ap.add_argument("--recursive", action="store_true")
    ap.add_argument("--file-name", dest="file_name")
    ap.add_argument("--sig")
    ap.add_argument("--cypher")
    ap.add_argument("--query", help="text to extract entities from (correlate mode)")
    ap.add_argument("--llm", action="store_true",
                    help="extract entities with the Gemma model instead of heuristics")
    ap.add_argument("--model", help="model path for --llm extraction")
    ap.add_argument("--backend", default="gpu", help="litert-lm backend for --llm")
    ap.add_argument("--ttl", type=int, default=86400)
    args = ap.parse_args()

    # Evict a stale graph before any read; updates refresh the timestamp.
    if args.mode in ("stats", "query") and os.path.exists(args.db) and is_expired(args.db, args.ttl):
        reset_db(args.db)

    if args.llm and args.model:
        extractor = lambda t: extract_entities_llm(t, args.model, backend=args.backend)  # noqa: E731
    else:
        extractor = extract_entities

    if args.mode == "update":
        if args.dir:
            update_dir(args.db, args.dir, args.recursive, extractor)
        elif args.file_name and args.sig is not None:
            update_file_stdin(args.db, args.file_name, args.sig, extractor)
        else:
            ap.error("update requires --dir, or --file-name and --sig")
    elif args.mode == "stats":
        stats(args.db)
    elif args.mode == "query":
        if not args.cypher:
            ap.error("query requires --cypher")
        query(args.db, args.cypher)
    elif args.mode == "correlate":
        if not os.path.exists(args.db):
            return
        correlate(args.db, args.query or args.cypher or "")


if __name__ == "__main__":
    main()
