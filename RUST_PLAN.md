# Gemma Genie — Rust rewrite plan

Status: **planning** (no code yet). Target branch: `rust`.

Goal: replace the bash `genie` orchestrator and the two Python helpers
(`genie_rag.py`, `genie_graph.py`) with a **single, self-contained Rust binary**
— no `uvx`, no Python at runtime. Same CLI, same on-disk caches, same behaviour.

## Why this is feasible

Every runtime dependency genie shells out to has a first-class Rust crate, and
several are *already Rust at the core* (the Python packages are just bindings):

| Today (Python via uvx) | Rust crate | Version | Notes |
|---|---|---|---|
| `lancedb` 0.33.0 | **`lancedb`** | 0.26.x (latest 0.x) | Lance core is Rust; Python is a binding. Rust is the first-class API. |
| `model2vec` 0.8.2 | **`model2vec-rs`** | latest 0.x | MinishLab's official Rust port. **Inference/encoding only** — exactly what genie uses. |
| `liteparse` 2.0.6 | **`liteparse`** | 2.0.6 | v2 is a Rust rewrite; the Python `lit` CLI is a binding. Crate has a lib API (`has_lib: true`). |
| `ladybug` 0.17.1 | **`lbug`** | 0.17.1 | Same engine, same version → on-disk graph format compatible. Kuzu-derived, Cypher. |
| `litert-lm` 0.13.1 | *(no Rust crate)* | — | C++ engine with a C API (`c/engine.h`). **This is the one risk — see below.** |

The version match for `lbug` (0.17.1) and `liteparse` (2.0.6) is convenient: the
graph file and parser behaviour should be drop-in compatible.

## The litert-lm question (the only hard part)

LiteRT-LM ships **no Rust inference crate**. The repo's `litert_lm_deps` crate is
build glue (`cxx`, `crate-type = ["staticlib"]`), not an API. The real surface is:

- A full **C API** in `c/engine.h` (`LiteRtLmEngine`, `LiteRtLmSession`,
  conversation/session config, generate, tokenize/detokenize, benchmark).
- The prebuilt **`litert-lm` CLI** (what the bash genie runs today via uvx).

Two strategies:

1. **Subprocess the prebuilt runtime (recommended for v1).** The Rust binary
   spawns the `litert-lm` CLI exactly like the bash version, keeping our noise
   filtering, streaming, GPU verification, and CPU fallback in Rust. Zero native
   build. Ships immediately. Keeps a non-Rust runtime dependency, but it's a
   single prebuilt binary, not a Python stack.
2. **FFI to the C API (v2, optional).** `bindgen` over `c/engine.h`, linking a
   prebuilt/`built` litert-lm static or shared lib. Removes the subprocess and
   gives in-process streaming + token control. Cost: building the native lib
   (Bazel/CMake with TensorFlow, Skia, etc.) and shipping it per-platform.

**Plan: v1 subprocesses litert-lm; revisit FFI once the rest is in Rust.** This
de-risks 80% of the rewrite (RAG + graph + CLI) without taking on the native
build, and the `run_llm`/`verify_gpu` logic ports almost 1:1.

## Crate layout (single binary, Cargo workspace)

```
genie/                      # workspace root (this repo, rust branch)
├── Cargo.toml              # [workspace]
└── crates/
    └── genie/
        ├── Cargo.toml      # the one binary crate
        └── src/
            ├── main.rs     # CLI parse + dispatch (clap)
            ├── cli.rs      # args/subcommands (--ask/--doc/--txt/--dir/--image/--audio, doctor, cache, graph-*)
            ├── config.rs   # env overrides + paths (~/.genie, HF_HOME, thresholds, TTL)
            ├── backend.rs  # resolve/verify GPU, CPU fallback, backend cache
            ├── llm.rs      # litert-lm invocation (subprocess v1), streaming + noise filter
            ├── parse.rs    # liteparse: file/dir extraction + soffice guard
            ├── rag.rs      # model2vec-rs embed + lancedb store/search + chunking + TTL eviction
            ├── graph.rs    # lbug: entity extraction (heuristic + LLM), update/stats/query/correlate
            ├── models.rs   # HF download/verify (hf-hub crate), model-variant selection
            └── doctor.rs   # dependency check + live backend report
```

A workspace (rather than a bare crate) leaves room to split out a `genie-core`
lib later if we want unit tests without the binary, but v1 is one binary crate.

## Behaviour to preserve (parity checklist)

Ported from the current bash + Python, verified against `genie --help`:

- CLI: `--ask`, `--doc` (+`--pages`), `--txt`, `--dir`, `--image`, `--audio`,
  stdin piping; subcommands `doctor`, `cache {info,list,clear}`, `--graph-stats`,
  `--graph-query`, `--verify-models`, `--version`, `--uninstall`.
- RAG: `GENIE_RAG_THRESHOLD` (14000) gate, chunking with overlap
  (`GENIE_CHUNK_SIZE` 1000), top-k (`GENIE_RAG_TOPK` 15), incremental re-embed by
  file signature, single-doc vs dir vs `--search-all` modes, TTL eviction
  (`GENIE_CACHE_TTL` 86400), `Sources:` footer.
- Graph: `(:File)-[:Mentions]->(:Entity)` schema, heuristic + Gemma-LLM entity
  extraction, background updates, TTL reset, correlate on bare `--ask`.
- Backend: env override → cache → probe; **GPU verification** and **runtime CPU
  fallback** (the 0.2.4 work) reimplemented natively.
- Embeddings: `minishlab/potion-retrieval-32M`. **Must match** the Python output
  numerically so existing vectors stay queryable (see risks).
- Caches: read/write the **same** `~/.genie/genie-cache.db` (LanceDB) and
  `~/.genie/genie-graph.lbug` so users migrate with no re-index.

## Milestones (each independently shippable)

1. **M0 — scaffold:** workspace + binary crate, `clap` CLI skeleton that parses
   every existing flag and prints a stub; `cargo check` green. No behaviour.
2. **M1 — LLM passthrough:** `llm.rs` subprocessing litert-lm with noise filter,
   streaming, `backend.rs` (resolve/verify/fallback), `doctor`. `genie --ask`
   reaches parity (no RAG yet). This is the core UX.
3. **M2 — parse + single-doc RAG:** `parse.rs` (liteparse) + `rag.rs`
   (model2vec-rs + lancedb) for `--doc/--txt/--image/stdin`, threshold gating,
   chunking, top-k, `Sources:`. Validate embedding parity vs Python.
4. **M3 — directory RAG:** recursive ingest, incremental re-embed, `--search-all`,
   TTL eviction, `cache` subcommand.
5. **M4 — graph:** `lbug` schema, heuristic extraction, `--graph-stats/-query`,
   correlate-on-bare-`--ask`; then LLM extraction; background updates.
6. **M5 — lifecycle:** `models.rs` (download/verify via `hf-hub`),
   `--verify-models`, `--uninstall`, self-update strategy (or drop it for a
   package-manager/`cargo install` flow).
7. **M6 (optional) — litert FFI:** replace the subprocess with `bindgen` over
   `c/engine.h` for in-process inference.

## Risks & mitigations

- **Embedding parity (high).** `model2vec-rs` must produce vectors close enough to
  Python `model2vec` for the *same* model, or existing caches return wrong
  neighbours. *Mitigation:* numeric diff test on M2; if they diverge, bump the
  cache schema/version key and re-embed rather than silently mixing.
- **LanceDB format compat (medium).** Rust `lancedb` 0.26.x vs Python 0.33.0 wrap
  the same Lance core but version independently. *Mitigation:* test read/write of
  an existing `genie-cache.db`; pin a Rust version with a compatible format, or
  gate on a format-version check.
- **litert-lm native build (medium, deferred).** Only if we pursue M6. *Mitigation:*
  subprocess for v1; treat FFI as opt-in.
- **`lbug`/`liteparse` API maturity (low-med).** Young crates; APIs may shift.
  *Mitigation:* pin exact versions (we already do), thin wrappers in `graph.rs`/
  `parse.rs` so churn is localized.
- **Distribution/self-update (low).** The bash self-update doesn't map to a binary.
  *Mitigation:* ship via `cargo install` / GitHub Releases / Homebrew; replace the
  weekly curl-swap with a "newer release available" notice.

## Open decisions (need a call before/at M0)

1. Keep `litert-lm` as a subprocess for v1? (plan assumes **yes**)
2. Hard requirement to reuse existing on-disk caches, or accept a one-time
   re-index on first Rust run? (affects how strict M2/M3 parity must be)
3. CLI crate: `clap` (derive) — assumed.
4. Repo layout: build the Rust tree on `rust` alongside the existing bash files
   until parity, then swap? (assumed — keeps `main` shippable throughout)
