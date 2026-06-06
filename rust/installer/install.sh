#!/bin/sh
# Gemma Genie — OS-agnostic install prelude. Picks the bootstrapper for this
# OS/arch, downloads it, and hands off. All real logic lives in the bootstrapper.
#   curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/rust/installer/install.sh | sh
set -eu
BASE="${GENIE_BOOTSTRAP_BASE:-https://github.com/sbmandava/gemma-genie/releases/latest/download}"
os=$(uname -s); arch=$(uname -m)
case "$os" in Linux) o=linux ;; Darwin) o=macos ;; *) echo "unsupported OS: $os"; exit 1 ;; esac
case "$arch" in x86_64|amd64) a=x86_64 ;; arm64|aarch64) a=aarch64 ;; *) echo "unsupported arch: $arch"; exit 1 ;; esac
bin="genie-bootstrap-${a}-${o}"
tmp="$(mktemp)"
echo "Fetching ${bin} ..."
curl -fsSL "${BASE}/${bin}" -o "${tmp}"
chmod +x "${tmp}"
exec "${tmp}" --manifest "${GENIE_MANIFEST:-${BASE}/manifest.json}" --install "$@"
