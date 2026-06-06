# /// script
# requires-python = ">=3.11"
# dependencies = ["ladybug"]
# ///
"""
Graph-correlation helper for the `gemma` CLI, backed by LadybugDB.

Builds a property graph from a directory of files:

    (:File {path})  -[:Mentions {count}]->  (:Entity {name})

Entities are extracted heuristically by default (capitalized phrases, acronyms,
frequent terms); with --llm they are refined per file by the Gemma model.

Then it answers correlation queries: which entities span multiple files, which
files are linked through shared entities, entity "hubs", or raw Cypher.

Modes: build | related | hubs | stats | query
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
from gemma_rag import extract_file, walk_files, TEXT_EXTS, DOC_EXTS  # noqa: E402

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

# Capitalized word/phrase within a single line (inter-word gap is spaces/tabs
# only — never a newline, so entities don't merge across lines).
CAP_PHRASE = re.compile(r"\b[A-Z][a-zA-Z0-9&'-]*(?:[ \t]+[A-Z][a-zA-Z0-9&'-]*)*\b")
ACRONYM = re.compile(r"\b[A-Z]{2,}[0-9]*\b")


def _clean(tok):
    return tok.strip(" \t.,;:!?'\"()[]-")


def extract_entities_heuristic(text, max_entities=40):
    """Return {entity: count} of likely named entities in text."""
    counts = {}
    for line in text.splitlines():
        for m in CAP_PHRASE.findall(line):
            phrase = _clean(m)
            # Drop leading stopword tokens, e.g. "The Apollo" -> "Apollo".
            words = phrase.split()
            while words and words[0] in STOPWORDS:
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
    # Keep multi-word names, acronyms, or single capitalized tokens that aren't
    # stopwords (length >= 3) — this keeps "DirecTV"/"Apollo" while dropping
    # sentence-initial filler like "The"/"And".
    keep = {
        k: v for k, v in counts.items()
        if " " in k or k.isupper() or len(k) >= 3 or v > 1
    }
    return dict(sorted(keep.items(), key=lambda kv: -kv[1])[:max_entities])


def extract_entities_llm(text, model_path, max_entities=40):
    """Use the Gemma model (via litert-lm) to extract entities for one file."""
    uvx = shutil.which("uvx") or "uvx"
    prompt = (
        "Extract the key named entities (people, organizations, projects, "
        "products, places) from the text. Reply with ONLY a comma-separated "
        "list of entity names, nothing else.\n\n" + text[:6000]
    )
    try:
        r = subprocess.run(
            [uvx, "litert-lm", "run", model_path, "--backend=gpu", "--prompt", prompt],
            capture_output=True, text=True,
        )
        out = r.stdout if r.returncode == 0 else ""
    except Exception:
        out = ""
    names = [n.strip(" .\t\"'") for n in re.split(r"[,\n]", out) if n.strip()]
    seen, ents = set(), {}
    for n in names:
        if 2 <= len(n) <= 60 and n.lower() not in seen:
            seen.add(n.lower())
            ents[n] = 1
        if len(ents) >= max_entities:
            break
    return ents


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
    """True if the graph hasn't been used within `ttl` seconds."""
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


def open_conn(db_path):
    import ladybug as lb
    return lb.Connection(lb.Database(db_path))


def build(db_path, root, recursive, use_llm, model_path):
    root = os.path.abspath(root)
    files = [
        os.path.relpath(p, root)
        for p in walk_files(root, recursive)
        if os.path.splitext(p)[1].lower() in (TEXT_EXTS | DOC_EXTS)
    ]
    if not files:
        sys.stderr.write("No supported files found in directory.\n")
        sys.exit(2)

    reset_db(db_path)
    conn = open_conn(db_path)
    conn.execute("CREATE NODE TABLE File(path STRING PRIMARY KEY)")
    conn.execute("CREATE NODE TABLE Entity(name STRING PRIMARY KEY)")
    conn.execute("CREATE REL TABLE Mentions(FROM File TO Entity, count INT64)")

    n_ent_links = 0
    for rel in files:
        abspath = os.path.join(root, rel)
        text = extract_file(abspath) or ""
        if not text.strip():
            continue
        ents = (extract_entities_llm(text, model_path) if use_llm
                else extract_entities_heuristic(text))
        conn.execute("MERGE (f:File {path: $p})", {"p": rel})
        for name, cnt in ents.items():
            conn.execute("MERGE (e:Entity {name: $n})", {"n": name})
            conn.execute(
                "MATCH (f:File {path: $p}), (e:Entity {name: $n}) "
                "MERGE (f)-[m:Mentions]->(e) SET m.count = $c",
                {"p": rel, "n": name, "c": int(cnt)},
            )
            n_ent_links += 1
        sys.stderr.write(f"  {rel}: {len(ents)} entities\n")

    sys.stderr.write(
        f"Built graph: {len(files)} files, {n_ent_links} mentions "
        f"({'LLM' if use_llm else 'heuristic'} extraction).\n"
    )


def _rows(conn, cypher, params=None):
    res = conn.execute(cypher, params or {})
    return [list(r) for r in res]


def related(db_path, name, limit):
    conn = open_conn(db_path)
    is_file = _rows(conn, "MATCH (f:File {path: $n}) RETURN f.path", {"n": name})
    is_entity = _rows(conn, "MATCH (e:Entity {name: $n}) RETURN e.name", {"n": name})

    if is_file:
        print(f"Files sharing entities with '{name}':")
        rows = _rows(conn,
            "MATCH (f1:File {path: $n})-[:Mentions]->(e:Entity)<-[:Mentions]-(f2:File) "
            "WHERE f1.path <> f2.path "
            "RETURN f2.path AS other, collect(e.name) AS shared, count(e) AS n "
            "ORDER BY n DESC LIMIT $k", {"n": name, "k": limit})
        for other, shared, n in rows:
            print(f"  {other}  ({n} shared)  {', '.join(shared[:8])}")
        if not rows:
            print("  (none)")
    elif is_entity:
        print(f"Files mentioning '{name}':")
        for (p,) in _rows(conn,
            "MATCH (e:Entity {name: $n})<-[:Mentions]-(f:File) RETURN f.path ORDER BY f.path",
            {"n": name}):
            print(f"  {p}")
        print(f"\nEntities co-occurring with '{name}':")
        rows = _rows(conn,
            "MATCH (e:Entity {name: $n})<-[:Mentions]-(f:File)-[:Mentions]->(e2:Entity) "
            "WHERE e2.name <> e.name "
            "RETURN e2.name AS name, count(f) AS files ORDER BY files DESC LIMIT $k",
            {"n": name, "k": limit})
        for nm, files in rows:
            print(f"  {nm}  ({files} files)")
        if not rows:
            print("  (none)")
    else:
        print(f"'{name}' not found as a File path or Entity name. "
              f"Try `gemma graph hubs` to see known entities.")


def hubs(db_path, limit):
    conn = open_conn(db_path)
    print("Top entities by number of files (correlation hubs):")
    rows = _rows(conn,
        "MATCH (f:File)-[:Mentions]->(e:Entity) "
        "RETURN e.name AS name, count(DISTINCT f) AS files "
        "ORDER BY files DESC, name LIMIT $k", {"k": limit})
    for name, files in rows:
        print(f"  {files:>3}  {name}")
    if not rows:
        print("  (graph is empty — run `gemma graph build --dir ...` first)")


def stats(db_path):
    conn = open_conn(db_path)
    nf = _rows(conn, "MATCH (f:File) RETURN count(f)")[0][0]
    ne = _rows(conn, "MATCH (e:Entity) RETURN count(e)")[0][0]
    nm = _rows(conn, "MATCH (:File)-[m:Mentions]->(:Entity) RETURN count(m)")[0][0]
    print(f"Graph: {nf} files, {ne} entities, {nm} mentions")
    print(f"DB:    {db_path}")


def query(db_path, cypher):
    conn = open_conn(db_path)
    for row in conn.execute(cypher):
        print(row)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--mode", required=True,
                    choices=["build", "related", "hubs", "stats", "query"])
    ap.add_argument("--dir")
    ap.add_argument("--recursive", action="store_true")
    ap.add_argument("--llm", action="store_true")
    ap.add_argument("--model", help="model path for --llm extraction")
    ap.add_argument("--name")
    ap.add_argument("--cypher")
    ap.add_argument("--limit", type=int, default=15)
    ap.add_argument("--ttl", type=int, default=86400,
                    help="clear the graph if idle longer than this many seconds")
    args = ap.parse_args()

    if args.mode == "build":
        if not args.dir:
            ap.error("build requires --dir")
        build(args.db, args.dir, args.recursive, args.llm, args.model)
        touch_usage(args.db)
        return

    if not os.path.exists(args.db):
        sys.stderr.write("No graph yet — run `gemma graph build --dir ...` first.\n")
        sys.exit(2)

    # Evict a stale graph (idle > ttl), mirroring the LanceDB cache behavior.
    if is_expired(args.db, args.ttl):
        reset_db(args.db)
        sys.stderr.write(
            "Graph expired (idle > TTL) and was cleared — "
            "rebuild with `gemma graph build --dir ...`.\n")
        sys.exit(2)
    touch_usage(args.db)

    if args.mode == "related":
        if not args.name:
            ap.error("related requires a name")
        related(args.db, args.name, args.limit)
    elif args.mode == "hubs":
        hubs(args.db, args.limit)
    elif args.mode == "stats":
        stats(args.db)
    elif args.mode == "query":
        if not args.cypher:
            ap.error("query requires a Cypher string")
        query(args.db, args.cypher)


if __name__ == "__main__":
    main()
