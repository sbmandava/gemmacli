# gemma CLI

A small local-LLM CLI built on Google's **Gemma 4** models — **E2B** (fast) and
**E4B** (stronger) — run on-device via
[`litert-lm`](https://github.com/google-ai-edge/litert-lm). Ask questions,
analyze documents, and query whole directories — all locally.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/sbmandava/gemmacli/main/install.sh | bash
```

The installer is idempotent and bootstraps **everything** on a fresh machine:

- `uv` / `uvx` (auto-installed if missing)
- LibreOffice (macOS, for DOCX/XLSX/PPTX parsing)
- `liteparse`, `lancedb`, `model2vec` (fetched via `uvx`)
- the Gemma model weights (downloaded into the HuggingFace hub cache)
- a `gemma` symlink on your `PATH`

Re-run it any time to repair an install or after deleting `~/.gemma`.

## Usage

```bash
gemma --ask "Explain TCP slow start in two sentences"

# Analyze a single document (PDF/DOCX/XLSX/PPTX/image)
gemma --ask "Summarize the key risks" --doc report.pdf
gemma --ask "Which sheet has the budget?" --doc plan.xlsx --pages "1-3"

# Plain text / CSV
gemma --ask "Who is blocked?" --txt tasks.csv

# Query an entire folder (recursive knowledge base)
gemma --ask "What's our vacation policy and who owns project X?" --dir ~/notes

# Graph correlation is automatic: analyzing files also builds a LadybugDB
# entity graph. A bare --ask then answers from everything indexed in the last 24h.
gemma --ask "summarize the Q2 risks" --doc q2.pdf   # indexes + answers
gemma --ask "who owns the Apollo project?"          # uses indexed data, no file
gemma --graph-stats                                 # graph stats + top hubs
gemma --graph-query "MATCH (f:File)-[:Mentions]->(e:Entity) RETURN f.name,e.name LIMIT 10"

# Vision / audio
gemma --image photo.jpg
gemma --audio clip.wav

# Pipe input from stdin
cat notes.txt | gemma --ask "summarize this"

# Tune retrieval for large inputs
gemma --ask "key risks?" --doc big.pdf --top-k 10 --chunk-size 1500
```

Run `gemma --help` for all options plus a live dependency check.

### Utility commands

```bash
gemma --version            # print version
gemma doctor               # dependency check
gemma cache info           # show vector-cache path, size, table count
gemma cache list           # list indexed tables
gemma cache clear          # wipe the vector cache
```

## How it works

- **Model**: **Gemma 4 E2B** (`--model e2b`, fast) or **Gemma 4 E4B**
  (`--model e4b`, stronger). File modes (`--txt/--doc/--dir`) default to **E4B**;
  override with `--model`. Weights are pulled from the `litert-community`
  HuggingFace repos (`gemma-4-E2B-it-litert-lm`, `gemma-4-E4B-it-litert-lm`).
- **Large inputs**: files over ~3,500 tokens are chunked, embedded with
  `model2vec`, and stored in a **LanceDB** vector cache. Only the chunks most
  relevant to your question are sent to the model — keeping answers accurate and
  within the context window.
- **`--dir`**: recursively ingests all supported files into one LanceDB table,
  re-embedding only files that changed, and retrieves across all of them
  (each excerpt is labeled with its source file).
- **`graph`**: builds a `(:File)-[:Mentions]->(:Entity)` property graph in
  **LadybugDB** from a folder (entities extracted heuristically, or with
  `--llm` via Gemma), then answers correlation queries — shared entities,
  files linked through them, entity hubs, or raw Cypher. The graph is a local
  `.lbug` file at `~/.gemma/gemma-graph.lbug` and is auto-cleared after 24h idle.

## Layout

| Path | What |
|------|------|
| `/opt/projects/unovie/gemmacli/` | the scripts (`gemma`, `gemma_rag.py`, `install.sh`) |
| `~/.gemma/gemma-cache.db/` | LanceDB vector cache (safe to delete; rebuilds on demand) |
| `~/.cache/huggingface/hub/` | all model weights (Gemma + embedder) |

## Environment overrides

| Var | Default | Purpose |
|-----|---------|---------|
| `GEMMA_CACHE_DB` | `~/.gemma/gemma-cache.db` | vector cache location |
| `GEMMA_RAG_THRESHOLD` | `14000` | char threshold before RAG kicks in |
| `GEMMA_RAG_TOPK` | `6` | chunks retrieved per query (`--top-k`) |
| `GEMMA_CHUNK_SIZE` | `1000` | characters per chunk (`--chunk-size`) |
| `GEMMA_CACHE_TTL` | `86400` | evict cached tables/graph idle longer than this (seconds) |
| `GEMMA_GRAPH_DB` | `~/.gemma/gemma-graph.lbug` | LadybugDB correlation-graph file |
| `GEMMA_BACKEND` | auto | force `gpu` or `cpu` (otherwise auto-detected and cached) |
| `HF_HOME` | `~/.cache/huggingface` | model cache root |

Installer-only: `GEMMA_INSTALL_DIR`, `GEMMA_BIN_DIR`, `GEMMA_RAW_BASE`,
`GEMMA_SKIP_MODELS=1`, `GEMMA_SKIP_PREWARM=1`.

## References

- [Quantization-aware training for Gemma 4](https://blog.google/innovation-and-ai/technology/developers-tools/quantization-aware-training-gemma-4/)
  — the QAT-tuned Gemma 4 models this CLI runs.
- [LiteRT-LM overview](https://developers.google.com/edge/litert-lm/overview)
  — the on-device runtime (`litert-lm`) used to run the models.

Also built on [LanceDB](https://lancedb.github.io/lancedb/),
[LadybugDB](https://github.com/LadybugDB/ladybug) (embedded Cypher graph DB),
[model2vec](https://github.com/MinishLab/model2vec), and
[liteparse](https://pypi.org/project/liteparse/).
