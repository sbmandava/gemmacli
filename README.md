# gemma CLI

A small local-LLM CLI built on Google's **Gemma** models via
[`litert-lm`](https://github.com/google-ai-edge/litert-lm). Ask questions,
analyze documents, and query whole directories — all on-device.

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

# Vision / audio
gemma --image photo.jpg
gemma --audio clip.wav
```

Run `gemma --help` for all options plus a live dependency check.

## How it works

- **Model**: `e2b` (fast) or `e4b` (stronger). File modes (`--txt/--doc/--dir`)
  default to `e4b`; override with `--model`.
- **Large inputs**: files over ~3,500 tokens are chunked, embedded with
  `model2vec`, and stored in a **LanceDB** vector cache. Only the chunks most
  relevant to your question are sent to the model — keeping answers accurate and
  within the context window.
- **`--dir`**: recursively ingests all supported files into one LanceDB table,
  re-embedding only files that changed, and retrieves across all of them
  (each excerpt is labeled with its source file).

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
| `GEMMA_RAG_TOPK` | `6` | chunks retrieved per query |
| `GEMMA_CACHE_TTL` | `86400` | evict cached tables idle longer than this (seconds) |
| `HF_HOME` | `~/.cache/huggingface` | model cache root |

Installer-only: `GEMMA_INSTALL_DIR`, `GEMMA_BIN_DIR`, `GEMMA_RAW_BASE`,
`GEMMA_SKIP_MODELS=1`, `GEMMA_SKIP_PREWARM=1`.
