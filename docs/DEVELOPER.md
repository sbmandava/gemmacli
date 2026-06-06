# Developer Guide — building Gemma Genie from source

This guide is for **developers who want to build the Rust `genie` CLI from
source** on **Linux** or **macOS**. If you just want to *use* Genie, you don't
need any of this — install a prebuilt binary instead:

```bash
# end users: one-line install (no toolchain needed)
curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash
# …or, with a Rust toolchain, the published installer:
cargo install genie-bootstrap && genie-bootstrap --install
```

Building from source is only needed to **hack on the code** or to **produce a
binary for a target we don't ship prebuilt**.

> Heads-up: the `genie` CLI links several **C/C++ native** libraries (LanceDB,
> Lance, LadybugDB, liteparse/PDFium/Tesseract, optionally litert-lm). The build
> is heavier than a pure-Rust crate — budget ~10 min for a clean release build and
> follow the prerequisites below exactly.

---

## Step 1 — Install Rust

We use the official toolchain installer, [`rustup`](https://rustup.rs). It
installs `rustc`, `cargo`, and keeps them updatable.

### Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# accept the defaults (option 1), then load it into your current shell:
. "$HOME/.cargo/env"
```

### macOS

```bash
# If you don't already have the Xcode command-line tools (provides clang, make…):
xcode-select --install

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
```

### Verify (both platforms)

```bash
rustc --version    # expect 1.8x or newer
cargo --version
```

(If you ever need to update later: `rustup update`.)

---

## Step 2 — Install native build prerequisites

These are **system** tools/libraries the native dependencies need at build time —
they are *not* Rust crates.

### Linux (Debian/Ubuntu)

| Tool | Why | Install |
|---|---|---|
| **protobuf-compiler** (`protoc`) | LanceDB's `lance-encoding` build script | `sudo apt-get install -y protobuf-compiler` |
| **GCC ≥ 13** (`g++-13`, `gcc-13`) | LadybugDB's FFI shim `#include <format>` (C++20) — libstdc++ only ships `<format>` from GCC 13. GCC 11/12 fail with `fatal error: format: No such file or directory` | see below |
| **CMake** + a **C++ toolchain** | LadybugDB build shim; general native builds | `sudo apt-get install -y cmake build-essential pkg-config` |

Install GCC 13 from the toolchain PPA, then point the build at it:

```bash
sudo add-apt-repository -y ppa:ubuntu-toolchain-r/test
sudo apt-get update
sudo apt-get install -y gcc-13 g++-13
```

The workspace's [`rust/.cargo/config.toml`](../rust/.cargo/config.toml) already
sets `CC=gcc-13 CXX=g++-13` and the `-Wl,--allow-multiple-definition` linker flag
(LadybugDB and LanceDB both bundle zstd), so **you don't need to export anything**
— just have `gcc-13`/`g++-13` on `PATH`.

Two more native deps are fetched/built automatically on the **first** build (so
the first build needs network):
- **PDFium** — liteparse's `pdfium-sys` downloads a prebuilt PDFium.
- **Tesseract + Leptonica** — liteparse builds them from source (the `tesseract`
  feature is on by default; disable it to slim the build / skip OCR).

### macOS (Apple Silicon or Intel)

No Homebrew/sudo required for the Rust toolchain itself. You need `clang` (from
Xcode CLT, Step 1), plus `cmake` and `protoc`:

```bash
# with Homebrew:
brew install cmake protobuf

# …or without brew/sudo (downloads official prebuilts into ~/.local):
#   see the project's installpre.sh approach — install cmake + protoc into
#   ~/.local/bin and put it on PATH:  export PATH="$HOME/.local/bin:$PATH"
```

On macOS the build uses **Apple clang**, not gcc-13. Override the Linux defaults
from `.cargo/config.toml` when you build (see Step 4):

```bash
export CC=clang CXX=clang++
```

(The zstd duplicate-symbol issue does **not** occur with Apple's `ld64`, so the
`--allow-multiple-definition` flag is harmless on macOS.)

---

## Step 3 — Get the upstream source dependencies

The `genie` CLI depends on five upstream Rust projects via **local path
dependencies** (so we build/debug against the exact upstream we track, and stay
reproducible offline). They are referenced by **absolute path** in
[`rust/crates/genie/Cargo.toml`](../rust/crates/genie/Cargo.toml) and the
`[patch]` in [`rust/Cargo.toml`](../rust/Cargo.toml), all rooted at
`/opt/projects/unovie/research/`.

The simplest way to build is to **clone them to those same paths** at the pinned
revisions:

```bash
mkdir -p /opt/projects/unovie/research
cd /opt/projects/unovie/research

git clone https://github.com/lancedb/lancedb.git
git -C lancedb checkout python-v0.33.1-beta.2

git clone https://github.com/lance-format/lance.git
git -C lance checkout v8.0.0-beta.6

git clone https://github.com/run-llama/liteparse.git
git -C liteparse checkout crates-v2.0.6

git clone https://github.com/MinishLab/model2vec-rs.git
git -C model2vec-rs checkout v0.2.1

git clone https://github.com/LadybugDB/ladybug-rust.git    # crate `lbug` v0.17.0
# (ladybug-rust's build.rs downloads a prebuilt engine; the C++ source clone
#  below is optional — only needed if you want it to build the engine locally)
git clone https://github.com/LadybugDB/ladybug.git
git -C ladybug checkout v0.17.1
```

| Path dep | Repo | Pin |
|---|---|---|
| `liteparse` | `run-llama/liteparse` | `crates-v2.0.6` |
| `model2vec-rs` | `MinishLab/model2vec-rs` | `v0.2.1` |
| `lancedb` | `lancedb/lancedb` | `python-v0.33.1-beta.2` (rust crate `0.30.1-beta.2`) |
| `lance` (core, via `[patch]`) | `lance-format/lance` | `v8.0.0-beta.6` |
| `lbug` | `LadybugDB/ladybug-rust` | crate `0.17.0` |

> **Different base directory?** If you can't use `/opt/projects/unovie/research/`,
> edit the `path = "…"` entries in `rust/crates/genie/Cargo.toml` and the
> `[patch."https://github.com/lance-format/lance.git"]` block in `rust/Cargo.toml`
> to point at wherever you cloned them. (A relative-path or env-based scheme is on
> the backlog; for now the paths are absolute.)

---

## Step 4 — Build & run

```bash
cd /opt/projects/unovie/gemma-genie/rust

# Linux: gcc-13 is picked up from .cargo/config.toml automatically.
# macOS: override the compiler first ->  export CC=clang CXX=clang++

cargo build --release -p genie

# run it straight from the workspace:
cargo run --release -p genie -- --ask "why is the sky blue"
cargo run --release -p genie -- doctor
```

The release binary lands at `rust/target/release/genie`. Put it on your `PATH`
(e.g. symlink into `~/.local/bin/`) to use it as `genie`.

### Optional: in-process inference (FFI)

By default `genie` **subprocesses** the prebuilt `litert-lm` runtime. To link the
litert-lm C API and generate **text** in-process instead:

```bash
cargo build --release -p genie --features ffi
```

This needs `liblitert-lm.so` discoverable at build/run time (set
`LITERT_LM_LIB_DIR` if it's not on the default search path). Vision/audio and
streaming still use the subprocess path.

---

## Step 5 — Run the tests

```bash
cd /opt/projects/unovie/gemma-genie/rust

cargo test                       # fast unit + CLI tests (always run)
cargo test -- --ignored          # model-backed integration tests (need GPU + model)
cargo test --features ffi -- --ignored   # also exercise the FFI path
```

The integration tests run the built binary against the sample corpus at
`/opt/projects/unovie/dataingest/sample`.

---

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `Could not find protoc` | Install `protobuf-compiler` (Step 2). |
| `fatal error: format: No such file or directory` | GCC < 13. Install `g++-13` and build on Linux (config.toml sets `CXX=g++-13`). |
| `duplicate symbol` / zstd link errors (Linux) | Ensure you're building inside `rust/` so `.cargo/config.toml` (with `-Wl,--allow-multiple-definition`) applies. |
| `Relocations in generic ELF EM:62` (cross builds) | The `CC=gcc-13` default leaked into a cross target; set per-target `CC_<target>`/`AR_<target>` for `ring`. |
| `error: failed to load source for dependency … path …` | An upstream path dep isn't cloned to the expected location — see Step 3. |
| macOS picks up `gcc-13` and fails | `export CC=clang CXX=clang++` before building (Step 2/4). |

---

## See also

- [`../rust/README.md`](../rust/README.md) — crate layout and build summary.
- [`QUICKSTART.md`](QUICKSTART.md) — the end-user install/usage path.
- [`RUST_PLAN.md`](RUST_PLAN.md) — the rewrite plan and milestones.
- [`VERSIONS.md`](VERSIONS.md) — pinned dependency versions.
