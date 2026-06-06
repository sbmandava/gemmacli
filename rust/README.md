# Gemma Genie — Rust rewrite

Single-binary Rust port of the bash + Python implementation in [`../python/`](../python/).
Design and milestones: [`../RUST_PLAN.md`](../RUST_PLAN.md). Dependency wiring and
build prerequisites: [`../CLAUDE.md`](../CLAUDE.md).

**Status: M0–M6 implemented and tested.** The `genie` binary reaches behaviour
parity with the bash CLI — `--ask` (plain/stdin/`--doc`/`--txt`/`--dir`),
`--image`/`--audio`, `doctor`, `cache`, the correlation graph
(`--graph-stats`/`--graph-query`), and model lifecycle
(`--verify-models`/`--uninstall`). Default build subprocesses the `litert-lm`
runtime; `--features ffi` runs text generation **in-process** via litert-lm's
C API.

| Milestone | Scope | State |
|---|---|---|
| M0 | scaffold + clap CLI | ✅ |
| M1 | `llm`/`backend`/`doctor`/`config` (subprocess + GPU verify + CPU fallback) | ✅ |
| M2 | `parse` (liteparse) + single-doc RAG (model2vec-rs + lancedb) | ✅ |
| M3 | directory KB (incremental), search-all, TTL, `cache` | ✅ |
| M4 | correlation graph via `lbug` (heuristic extraction) | ✅ |
| M5 | model lifecycle (`--verify-models`, `--uninstall`) via `hf-hub` | ✅ |
| M6 | in-process FFI (text); vision/audio + streaming stay subprocess | ✅ (text) |

## Build & run

Prereqs: `protoc`, **GCC ≥ 13** on Linux (lbug needs C++20 `<format>`), CMake, a
C++ toolchain. Linux builds use `CC=gcc-13 CXX=g++-13` (set in `.cargo/config.toml`).
See [`../CLAUDE.md`](../CLAUDE.md) for the full list and the **macOS build host**.

```sh
cd rust
cargo build --release -p genie           # the CLI
cargo run -- --ask "why is the sky blue" # subprocess runtime
cargo run -- doctor

# Optional: in-process inference via the litert-lm C API
cargo build --release -p genie --features ffi
```

Tests: `cargo test` (fast unit + CLI); model-backed + ffi tests are `#[ignore]`d
(`cargo test -- --ignored`, ffi with `--features ffi`). The sample corpus used by
the integration tests is `/opt/projects/unovie/dataingest/sample`.

## Installer (OS-agnostic, modular)

[`installer/`](installer/) holds the `genie-bootstrap` preludes
(`install.sh`/`install.ps1`) and an example manifest; the bootstrapper crate is
[`crates/genie-bootstrap`](crates/genie-bootstrap) — it probes the environment
and fetches only the components a target needs. See
[`../specs/rust-installer.md`](../specs/rust-installer.md) (local).

## Layout

```
rust/
├── Cargo.toml            # workspace (+ [patch] redirecting the lance core to local source)
├── .cargo/config.toml    # CC/CXX=gcc-13 + linker flag for the lbug/lancedb zstd clash
├── installer/            # install.sh / install.ps1 / manifest.example.json
└── crates/
    ├── genie/            # the CLI
    │   ├── src/          # main, cli, config, backend, llm, parse, rag, graph, models, doctor, ffi
    │   └── tests/        # unit.rs + integration.rs (run the binary on the sample corpus)
    └── genie-bootstrap/  # the modular installer bootstrapper
```

## Dependencies

Built from **local** `research/` sources (see `../CLAUDE.md`): `lancedb` (+`lance`
via `[patch]`), `model2vec-rs`, `liteparse`, `lbug`. The `litert-lm` runtime is
subprocessed from the prebuilt binary (FFI links `liblitert-lm.so` under
`--features ffi`).
