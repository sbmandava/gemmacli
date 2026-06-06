# Gemma Genie 🧞

<p align="center">
  <img src="genie.png" alt="Gemma Genie" width="220">
</p>

**Gemma Genie** is a private, offline AI assistant for your laptop — a CLI
(`genie`) built on Google's **Gemma 4** models — **E2B** (fast) and **E4B**
(stronger) — run on-device via
[`litert-lm`](https://github.com/google-ai-edge/litert-lm). Ask questions,
analyze documents, and query whole folders — all locally, no cloud.

## Project goals

- **Privacy first** — your documents never leave the device; no cloud, no API keys.
- **Truly offline** — network is needed only for the one-time install.
- **Useful on real files** — ask across PDFs, Office docs, folders, images, and audio.
- **Reproducible** — runtime dependencies are pinned to known-good versions.

It also serves as an open testbed for **self-learning AI agents that run on the
edge** — see [Sponsor & vision](#sponsor--vision).

**Documentation**

- [QUICKSTART.md](docs/QUICKSTART.md) — install and first queries in five steps
- [FAQ.md](docs/FAQ.md) — privacy, offline use, file formats, requirements, cache reset
- [VERSIONS.md](docs/VERSIONS.md) — pinned dependency versions
- [CHANGELOG.md](docs/CHANGELOG.md) — version history
- [rust/README.md](rust/README.md) — the single-binary **Rust rewrite** (in progress)

## Repository layout

The shipping implementation and a from-scratch rewrite live side by side:

| Path | What |
|------|------|
| [`install.sh`](install.sh) | the installer (repo root). **Default installs the Rust binary**; `--python` installs the bash + Python implementation. |
| [`rust/`](rust/) | the **single-binary Rust build** (the default) — same CLI/behaviour, no Python/`uvx` at runtime (`lancedb` + `model2vec-rs` + `liteparse` + `lbug`, with litert-lm subprocessed). Prebuilt for Linux/macOS; see [rust/README.md](rust/README.md). |
| [`python/`](python/) | the **bash + Python** implementation — `genie` + helpers (`genie_rag.py`, `genie_graph.py`) run via `uvx`. Installed with `install.sh --python`. |

## Supported platforms

| OS | Status |
|----|--------|
| macOS (Apple Silicon / Intel) | ✅ supported |
| Linux (x86_64 / arm64) | 🧪 alpha |
| Windows (WSL2) | 🧪 alpha |

GPU acceleration is used when available, with automatic fallback to CPU.

## Install

By default this installs the **Rust single-binary** build (no Python at runtime):

```bash
curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash
```

Prefer the **bash + Python** implementation? Add `--python`:

```bash
curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash -s -- --python
```

The installer is idempotent and bootstraps **everything** on a fresh machine:

- the prebuilt Rust `genie` for your OS/arch (default), or the bash + Python
  scripts with `--python`
- `uv` / `uvx` (auto-installed; runs the litert-lm runtime, and the Python deps
  in `--python` mode)
- LibreOffice (for DOCX/XLSX/PPTX parsing)
- the Gemma model weights (downloaded into the HuggingFace hub cache)
- a `genie` symlink on your `PATH`

Prebuilt Rust CLIs are published for **x86_64 Linux** and **Apple-Silicon
macOS**; other targets fall back to `--python` (or build from
[`rust/`](rust/)). Re-run any time to repair an install or after deleting `~/.genie`.

> **Runs offline.** Network is only needed for the one-time install (downloading
> `uv`, the Python deps, and the Gemma model weights). Once those are cached,
> `genie` runs entirely on-device from the command line — no cloud, no API keys,
> no internet required. Your documents never leave the machine.

## Usage

```bash
genie --ask "Explain TCP slow start in two sentences"

# Analyze a single document (PDF/DOCX/XLSX/PPTX/image)
genie --ask "Summarize the key risks" --doc report.pdf
genie --ask "Which sheet has the budget?" --doc plan.xlsx --pages "1-3"

# Plain text / CSV
genie --ask "Who is blocked?" --txt tasks.csv

# Query an entire folder (recursive knowledge base)
genie --ask "What's our vacation policy and who owns project X?" --dir ~/notes

# Graph correlation is automatic: analyzing files also builds a LadybugDB
# entity graph. A bare --ask then answers from everything indexed in the last 24h.
genie --ask "summarize the Q2 risks" --doc q2.pdf   # indexes + answers
genie --ask "who owns the Apollo project?"          # uses indexed data, no file
genie --graph-stats                                 # graph stats + top hubs
genie --graph-query "MATCH (f:File)-[:Mentions]->(e:Entity) RETURN f.name,e.name LIMIT 10"

# Vision / audio
genie --image photo.jpg
genie --audio clip.wav

# Pipe input from stdin
cat notes.txt | genie --ask "summarize this"

# Tune retrieval for large inputs
genie --ask "key risks?" --doc big.pdf --top-k 10 --chunk-size 1500
```

Run `genie --help` for all options plus a live dependency check.

### Utility commands

```bash
genie --version            # print version
genie doctor               # dependency check
genie cache info           # show vector-cache path, size, table count
genie cache list           # list indexed tables
genie cache clear          # wipe the vector cache
genie --graph-stats        # correlation-graph counts + top entity hubs
genie --graph-query "MATCH (f:File)-[:Mentions]->(e:Entity) RETURN f.name,e.name LIMIT 10"
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
  (each excerpt is labeled with its source file). It **also** builds a
  `(:File)-[:Mentions]->(:Entity)` correlation graph in **LadybugDB** in the
  same step (entities extracted via Gemma for directories, heuristically for
  single `--txt`/`--doc`) — so analyzing files populates both the vector cache
  and the graph automatically.
- **Graph correlation (LadybugDB)**: the graph is updated automatically by the
  steps above (no separate build command) and stored as a local `.lbug` file at
  `~/.genie/genie-graph.lbug`, auto-cleared after 24h idle. Inspect it with
  `genie --graph-stats` (counts + top hubs) or `genie --graph-query "<cypher>"`.
- **Auto-consult**: a bare `genie --ask` (no file given) automatically answers
  from whatever you indexed in the last 24h — relevant LanceDB chunks plus
  LadybugDB entity correlations for the entities in your question.

## Layout

| Path | What |
|------|------|
| `~/.local/share/genie/` | the scripts (`genie`, `genie_rag.py`, `install.sh`) |
| `~/.genie/genie-cache.db/` | LanceDB vector cache (safe to delete; rebuilds on demand) |
| `~/.cache/huggingface/hub/` | all model weights (Gemma + embedder) |

## Environment overrides

| Var | Default | Purpose |
|-----|---------|---------|
| `GENIE_CACHE_DB` | `~/.genie/genie-cache.db` | vector cache location |
| `GENIE_RAG_THRESHOLD` | `14000` | char threshold before RAG kicks in |
| `GENIE_RAG_TOPK` | `15` | chunks retrieved per query (`--top-k`) |
| `GENIE_CHUNK_SIZE` | `1000` | characters per chunk (`--chunk-size`) |
| `GENIE_CACHE_TTL` | `86400` | evict cached tables/graph idle longer than this (seconds) |
| `GENIE_GRAPH_DB` | `~/.genie/genie-graph.lbug` | LadybugDB correlation-graph file |
| `GENIE_BACKEND` | auto | force `gpu` or `cpu` (otherwise auto-detected and cached) |
| `GENIE_MODEL` | auto | force `e2b`/`e4b` (default set at install from RAM: <6GB→e2b, ≥6GB→e4b) |
| `HF_HOME` | `~/.cache/huggingface` | model cache root |

Installer-only: `GENIE_INSTALL_DIR`, `GENIE_BIN_DIR`, `GENIE_RAW_BASE`,
`GENIE_SKIP_MODELS=1`, `GENIE_SKIP_PREWARM=1`.

## References

- [Quantization-aware training for Gemma 4](https://blog.google/innovation-and-ai/technology/developers-tools/quantization-aware-training-gemma-4/)
  — the QAT-tuned Gemma 4 models this CLI runs.
- [LiteRT-LM overview](https://developers.google.com/edge/litert-lm/overview)
  — the on-device runtime (`litert-lm`) used to run the models.

Also built on [LanceDB](https://lancedb.github.io/lancedb/),
[LadybugDB](https://github.com/LadybugDB/ladybug) (embedded Cypher graph DB),
[model2vec](https://github.com/MinishLab/model2vec) (the
`minishlab/potion-retrieval-32M` embedder), and
[liteparse](https://pypi.org/project/liteparse/).

Huge thanks to **[Google DeepMind](https://deepmind.google/)** for their amazing
innovation — **TensorFlow**, **LiteRT/LiteRT-LM**, and the open, offline-capable
**Gemma** models — that make on-device AI like this possible.

Special thanks to **[Prashant Rao](https://ca.linkedin.com/in/prrao87)** at
LanceDB — a truly innovative leader from Toronto who teaches the world through
his blog, [The Data Quarry](https://thedataquarry.com/).

## Sponsor & vision

**Gemma Genie is sponsored by [Unovie.AI](https://unovie.ai/)** as a testbed for
**self-learning AI agents that run on the edge** — learning and improving locally,
without the cloud. See Unovie.AI's
[Edge AI whitepaper](https://unovie.ai/resources/edge-ai-whitepaper) for the
overarching goals.

## About the author

Created by **[Suresh Mandava](https://www.linkedin.com/in/mandavasuresh)**.
