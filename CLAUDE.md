# CLAUDE.md — Gemma Genie

Private, offline AI assistant CLI built on Gemma 4 (litert-lm) with local RAG
(LanceDB + model2vec) and a correlation graph (LadybugDB). `main` is the shipping
bash + Python implementation; the **`rust` branch** is a single-binary Rust
rewrite — see [RUST_PLAN.md](RUST_PLAN.md).

## Building the Rust version: use LOCAL upstream source first

Upstream source repos are already cloned under **`/opt/projects/unovie/research/`**.
When adding a Rust dependency for the rewrite, **prefer a local path/submodule
dependency to the cloned source over crates.io.** Reasons: build/debug against the
exact upstream we're tracking, patch locally if needed, and stay reproducible
offline. Only fall back to crates.io when the local source isn't a usable Rust
crate (noted below). Always check the repo first.

| Dependency | Local Rust source | crate | Use |
|---|---|---|---|
| **lancedb** | `/opt/projects/unovie/research/lancedb/rust/lancedb` (v0.30.1-beta.2, repo @ `python-v0.33.1-beta.2`) | `lancedb` | **Path dep to local source.** |
| **liteparse** | `/opt/projects/unovie/research/liteparse/crates/liteparse` (v2.0.6) | `liteparse` | **Path dep to local source.** |
| **ladybug / lbug** | `/opt/projects/unovie/research/ladybug` (C++ engine, repo @ `v0.17.1`). Rust API is the **uninitialized submodule** `tools/rust_api` → github.com/ladybugdb/ladybug-rust | `lbug` | **Init the submodule first**, then path dep: `git -C /opt/projects/unovie/research/ladybug submodule update --init tools/rust_api`. Building links the C++ engine (CMake). If that's not viable, fall back to `cargo add lbug`. |
| **model2vec** | `/opt/projects/unovie/research/model2vec` is the **Python** repo (no Rust here). The Rust port `model2vec-rs` (github.com/MinishLab/model2vec-rs) is **not cloned**. | `model2vec-rs` | Clone `model2vec-rs` into `research/` and path-dep it, **or** use crates.io. Inference-only (fine — genie only encodes). |
| **litert-lm** | `/opt/projects/unovie/research/LiteRT-LM` (C++; C API in `c/engine.h`). **No Rust inference crate.** | — | v1 **subprocesses the prebuilt `litert-lm` binary** (per RUST_PLAN.md). FFI via `bindgen` over `c/engine.h` is a later, optional milestone. |

Quick check before depending on any of these:
`find /opt/projects/unovie/research/<repo> -name Cargo.toml -not -path '*/target/*'`

## Behaviour parity & on-disk caches

The Rust binary must preserve today's CLI and read the **same** caches in
`~/.genie/` (`genie-cache.db` LanceDB dir, `genie-graph.lbug`). Two parity risks
to validate early: `model2vec-rs` embeddings must match Python `model2vec` for
`minishlab/potion-retrieval-32M`, and the Rust `lancedb` on-disk format must be
readable. See RUST_PLAN.md → Risks. `lbug` 0.17.1 == ladybug 0.17.1, so the graph
file is format-compatible.

## Conventions

- Shell scripts use `set -euo pipefail` — guard pipelines whose failure is
  expected (e.g. `grep` with no match) with `|| true`, or they abort the script.
- Keep macOS (bash 3.2) compatibility in the bash `genie`: no `${var,,}`, use `tr`.
- Git remote uses an `insteadOf` rewrite that forces SSH→HTTPS; push with the
  explicit scheme form: `git push ssh://git@github.com/sbmandava/gemma-genie.git <branch>`.
