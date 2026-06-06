# Changelog

All notable changes to Gemma Genie are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — `rust` branch

### Added
- **Single-binary Rust rewrite** under [`rust/`](../rust/) (milestones M0–M6): same
  CLI/behaviour with no Python/`uvx` at runtime — `lancedb` + `model2vec-rs` +
  `liteparse` + `lbug`, with `litert-lm` subprocessed (or in-process via
  `--features ffi` for text). Builds for Linux x86_64 and macOS arm64.
- **OS-agnostic modular installer** (`rust/installer/` + the `genie-bootstrap`
  crate): a tiny bootstrapper probes the environment (OS/arch/GPU/RAM) and fetches
  only the components a target needs.
- **`genie-bootstrap` published to [crates.io](https://crates.io/crates/genie-bootstrap)**
  (v0.2.4): `cargo install genie-bootstrap && genie-bootstrap --install` fetches the
  matching prebuilt `genie`. (The `genie` CLI stays off crates.io — its local/path
  deps aren't publishable; end users get the prebuilt binary, developers build from
  `rust/`.)

### Changed
- **Repository restructure**: the shipping bash + Python implementation moved to
  [`python/`](../python/) (`genie`, `genie_rag.py`, `genie_graph.py`, `install.sh`).
  `install.sh` and `genie`'s self-update now resolve scripts from `python/` with a
  repo-root fallback (`GENIE_RAW_SUBDIR`), so existing installs keep working.

## [0.2.4]

### Added
- Runtime auto-fallback to CPU: if a GPU run produces no answer (no output, or
  litert's "An error occurred"), genie retries the same prompt on CPU so the
  user still gets a result. A successful GPU run is never re-run, so there's no
  double output, and live streaming on the GPU path is preserved. The fallback
  is remembered for the process and cached for future runs (unless the backend
  is forced via `GENIE_BACKEND`); `genie doctor` re-verifies and can restore GPU.

## [0.2.3]

### Fixed
- `genie --ask` no longer exits non-zero after printing a correct answer — an
  empty "Sources" footer made `grep` fail and tripped `set -euo pipefail`.

### Added
- `--doc` on Office formats (DOCX/XLSX/PPTX and ODF) now exits early with an
  OS-specific install command when LibreOffice's `soffice` is missing, instead
  of failing deep inside the parser.

### Changed
- Suppressed harmless GPU/OpenCL backend log noise from the on-device runtime
  (the `maxDynamic…` warnings and the `Loaded OpenCL library` line), and
  tolerate a non-zero exit from the litert-lm GPU backend after a successful
  generation.
- `genie doctor` now reports the compute backend genie will actually use
  (GPU/CPU) and actively verifies the GPU runs with litert-lm, falling back to
  CPU — and correcting the cached choice — when it doesn't.
- LibreOffice install hints are OS-aware (Homebrew / apt / dnf / yum / pacman /
  zypper) rather than always suggesting Homebrew, and `install.sh` installs only
  the minimal headless components instead of the full GUI suite.

## [0.2.2]

### Changed
- Pinned all runtime dependencies to known-good versions for reproducible
  `uvx` resolution — see [VERSIONS.md](VERSIONS.md):
  litert-lm 0.13.1, lancedb 0.33.0, model2vec 0.8.2, numpy 2.4.6,
  pyarrow 24.0.0, ladybug 0.17.1, liteparse 2.0.6.
- Switched the embedder to `minishlab/potion-retrieval-32M` (retrieval-tuned)
  for better RAG quality, replacing `minishlab/potion-base-8M`.

## [0.2.1]

### Added
- `genie --verify-models` to check model integrity.

### Changed
- Verify model integrity using `hf_hub_download` as the source of truth.
