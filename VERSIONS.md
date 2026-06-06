# Pinned Versions

The shipping **Python implementation** (in [`python/`](python/)) resolves its
runtime dependencies on demand via `uvx`. To keep resolution reproducible — so a
cache miss can't silently pull a newer, possibly breaking release — every
dependency is pinned to a known-good version. (The **Rust rewrite** in
[`rust/`](rust/) pins its equivalents in Cargo from local source — see the
"Rust rewrite" section below.)

This file is the single source of truth. When bumping a version, update the
pinned value **here and at every invocation site listed below**, then re-test.

## Versions

| Package        | Version  | Role                                              |
|----------------|----------|---------------------------------------------------|
| litert-lm      | 0.13.1   | On-device LLM runtime (Gemma inference)           |
| lancedb        | 0.33.0   | Vector store for RAG retrieval                    |
| model2vec      | 0.8.2    | Static embedder (minishlab/potion-retrieval-32M)  |
| numpy          | 2.4.6    | Numerics (RAG helper)                             |
| pyarrow        | 24.0.0   | LanceDB data layer                                |
| ladybug        | 0.17.1   | Embedded Cypher graph DB (entity correlation)     |
| liteparse      | 2.0.6    | Document extraction (PDF/DOCX/XLSX/PPTX/images)   |

## Where each pin lives (Python implementation, `python/`)

- **litert-lm** — `LITERT_VERSION="0.13.1"` in `python/genie` and
  `python/install.sh`; used as `litert-lm@${LITERT_VERSION}`. Hardcoded as
  `litert-lm@0.13.1` in `python/genie_graph.py`. Not env-overridable.
- **lancedb** — `--with lancedb==0.33.0` in `python/genie` and
  `python/install.sh`; `dependencies` block in `python/genie_rag.py`.
- **model2vec** — `--with model2vec==0.8.2` in `python/genie` and
  `python/install.sh`; `dependencies` block in `python/genie_rag.py`.
- **numpy** / **pyarrow** — `dependencies` block in `python/genie_rag.py`.
- **ladybug** — `--with ladybug==0.17.1` in `python/genie` and
  `python/install.sh`; `dependencies` block in `python/genie_graph.py`.
- **liteparse** — `--from liteparse==2.0.6` in `python/genie`,
  `python/genie_rag.py`, and `python/install.sh`.

## Rust rewrite (`rust/`)

The Rust binary pins equivalents in Cargo, built from local upstream source
under `/opt/projects/unovie/research/` (see the build notes; dependency wiring is
tracked there). The embedder model and litert-lm runtime version match the
Python side.

| Crate          | Version        | Role                                      |
|----------------|----------------|-------------------------------------------|
| lancedb        | 0.30.1-beta.2  | vector store (Rust-native)                |
| lance (core)   | v8.0.0-beta.6  | lancedb's storage engine (via `[patch]`)  |
| model2vec-rs   | 0.2.1          | static embedder (potion-retrieval-32M)    |
| liteparse      | 2.0.6          | document extraction                       |
| lbug           | 0.17.0         | embedded Cypher graph (ladybug engine)    |
| hf-hub         | 0.4            | model download / verify                   |
| litert-lm      | 0.13.1         | inference runtime (subprocess; FFI optional) |

Build prerequisites (Linux): `protoc`, **GCC ≥ 13** (lbug needs C++20 `<format>`),
CMake, a C++ toolchain. The embedder is `minishlab/potion-retrieval-32M` and the
models are the same `litert-community/gemma-4-E{2,4}B-it-litert-lm` weights.
