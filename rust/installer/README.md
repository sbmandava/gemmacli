# Gemma Genie — installer

OS-agnostic, modular installer. A tiny per-OS prelude downloads the
`genie-bootstrap` binary, which probes the environment (OS, arch, libc, GPU,
RAM, existing deps) and fetches **only the components that environment needs**
from a signed `manifest.json` (one model variant by RAM, GPU backend only if a
GPU is present, the right platform binary + runtime libs).

- `install.sh`  — Linux/macOS prelude (`curl … | sh`)
- `install.ps1` — Windows prelude (`iwr … | iex`)
- `manifest.example.json` — example component manifest
- the bootstrapper itself: `../crates/genie-bootstrap`

Design: [`../../specs/rust-installer.md`](../../specs/rust-installer.md) (local).

Try it without installing anything (dry-run plan):
```
cargo run -p genie-bootstrap                                   # probe + abstract plan
cargo run -p genie-bootstrap -- --manifest installer/manifest.example.json
```
