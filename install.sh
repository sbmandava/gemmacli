#!/usr/bin/env bash
#
# Gemma Genie installer — bootstraps everything on a fresh machine.
#
#   # Default: the Rust single-binary build (downloads the prebuilt genie):
#   curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash
#
#   # Python implementation instead (bash + Python helpers via uvx):
#   curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash -s -- --python
#
# Idempotent: safe to re-run. If you delete ~/.genie (the vector cache) or are on
# a brand-new laptop, re-running this brings everything back, including the
# Gemma model weights.
#
# Flags:
#   (default)           install the prebuilt Rust `genie` binary for your OS/arch
#   --python            install the bash + Python scripts instead
#   --rust              explicit Rust (the default; accepted for clarity)
#
# Environment overrides:
#   GENIE_INSTALL_DIR   where the scripts live   (default: ~/.local/share/genie)
#   GENIE_BIN_DIR       where the `genie` link goes (default: ~/.local/bin, then /usr/local/bin)
#   GENIE_RAW_BASE      raw URL to fetch files from when piped via curl
#   GENIE_RAW_SUBDIR    repo subdir holding the scripts (default: python; "" = root)
#   GENIE_RUST_RELEASE  GitHub release tag for --rust (default: rust-prebuilt-0.2.4)
#   GENIE_RUST_BASE     base URL for --rust binaries (default: that release's assets)
#   HF_HOME             HuggingFace cache root (default: ~/.cache/huggingface) — all models go here
#   GENIE_SKIP_MODELS=1 skip downloading the (large) Gemma weights
#   GENIE_SKIP_PREWARM=1 skip all pre-downloads (deps still install)
#
set -euo pipefail

INSTALL_DIR="${GENIE_INSTALL_DIR:-$HOME/.local/share/genie}"
RAW_BASE="${GENIE_RAW_BASE:-https://raw.githubusercontent.com/sbmandava/gemma-genie/main}"
# Subdirectory in the repo holding the runtime scripts. The bash+Python
# implementation now lives in python/; remote fetches try here first, then the
# repo root (legacy layout). Override with GENIE_RAW_SUBDIR= (empty for root).
RAW_SUBDIR="${GENIE_RAW_SUBDIR-python}"
CACHE_DIR="$HOME/.genie"
# Pin the litert-lm runtime so uvx resolves a known-good version.
LITERT_VERSION="0.13.1"

# All models live in the HuggingFace hub cache.
export HF_HOME="${HF_HOME:-$HOME/.cache/huggingface}"

# Gemma weights to pre-download, as "hf-repo|filename|approx-size" entries.
MODEL_SPECS=(
    "litert-community/gemma-4-E2B-it-litert-lm|gemma-4-E2B-it.litertlm|2.4 GB"
    "litert-community/gemma-4-E4B-it-litert-lm|gemma-4-E4B-it.litertlm|3.4 GB"
)

say()  { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mWARN:\033[0m %s\n' "$*" >&2; }
have() { command -v "$1" >/dev/null 2>&1; }

# Where am I running from? (a local checkout vs. piped through curl) The runtime
# scripts live in python/; this install.sh sits at the repo root. Support both
# the root layout (scripts in ./python/) and running with scripts alongside.
SELF="${BASH_SOURCE[0]:-}"
SRC_DIR=""
if [ -n "$SELF" ] && [ -f "$SELF" ]; then
    self_dir="$(cd "$(dirname "$SELF")" && pwd)"
    if [ -f "$self_dir/genie" ]; then
        SRC_DIR="$self_dir"            # install.sh alongside the scripts
    elif [ -f "$self_dir/python/genie" ]; then
        SRC_DIR="$self_dir/python"     # install.sh at repo root, scripts in python/
    fi
fi

# Flags. Default is the Rust single-binary build; --python installs the bash +
# Python implementation instead. (--rust is accepted as an explicit no-op.)
RUST_MODE=1
for arg in "$@"; do
    case "$arg" in
        --python) RUST_MODE=0 ;;
        --rust) RUST_MODE=1 ;;
        -h|--help) sed -n '2,32p' "$0" 2>/dev/null | sed 's/^#//'; exit 0 ;;
        *) warn "ignoring unknown flag: $arg" ;;
    esac
done

# Rust prebuilt download settings (used unless --python).
RUST_RELEASE="${GENIE_RUST_RELEASE:-rust-prebuilt-0.2.4}"
RUST_BASE="${GENIE_RUST_BASE:-https://github.com/sbmandava/gemma-genie/releases/download/$RUST_RELEASE}"

# Name of the prebuilt Rust `genie` CLI asset for this OS/arch (empty if none).
rust_cli_asset() {
    local os arch
    case "$(uname)" in Darwin) os=macos ;; Linux) os=linux ;; *) return ;; esac
    case "$(uname -m)" in x86_64|amd64) arch=x86_64 ;; arm64|aarch64) arch=aarch64 ;; *) return ;; esac
    if [ "$os" = linux ] && [ "$arch" = x86_64 ]; then echo "genie-x86_64-linux-gnu"
    elif [ "$os" = macos ] && [ "$arch" = aarch64 ]; then echo "genie-aarch64-macos"
    fi   # other targets have no prebuilt CLI yet
}

# ---------------------------------------------------------------------------
# 1. uv / uvx  (runs the model, liteparse, and the RAG helper)
# ---------------------------------------------------------------------------
if ! have uvx; then
    say "Installing uv (provides uvx)..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
fi
have uvx || { warn "uvx still not on PATH; open a new shell and re-run."; exit 1; }
say "uv: $(uv --version 2>/dev/null || echo present)"

# ---------------------------------------------------------------------------
# 2. LibreOffice — needed by liteparse for DOCX/XLSX/PPTX (PDF/images don't need it)
#    We install only the minimal headless components, not the full GUI suite.
# ---------------------------------------------------------------------------
libreoffice_install() {
    case "$(uname)" in
        Darwin)
            if ! have brew; then
                say "Installing Homebrew..."
                NONINTERACTIVE=1 /bin/bash -c \
                  "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || \
                  { warn "Homebrew install failed"; return 1; }
                [ -x /opt/homebrew/bin/brew ] && eval "$(/opt/homebrew/bin/brew shellenv)"
                [ -x /usr/local/bin/brew ]    && eval "$(/usr/local/bin/brew shellenv)"
            fi
            have brew && brew install --cask libreoffice
            ;;
        Linux)
            local SUDO=""
            [ "$(id -u)" -ne 0 ] && have sudo && SUDO="sudo"
            # Minimal component set that still converts docx/xlsx/pptx headlessly.
            if have apt-get; then
                $SUDO apt-get update && \
                $SUDO apt-get install -y --no-install-recommends \
                    libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress
            elif have dnf; then
                $SUDO dnf install -y libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress
            elif have yum; then
                $SUDO yum install -y libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress
            elif have pacman; then
                $SUDO pacman -S --noconfirm libreoffice-still
            elif have zypper; then
                $SUDO zypper install -y libreoffice
            else
                warn "No known package manager; install LibreOffice manually (it provides 'soffice')."
                return 1
            fi
            ;;
        *) warn "Unsupported OS for automatic LibreOffice install."; return 1 ;;
    esac
}

if ! have soffice; then
    say "Installing minimal LibreOffice (for DOCX/XLSX/PPTX parsing; PDF/images work without it)..."
    libreoffice_install || warn "LibreOffice install failed (PDF/images still work; office docs need 'soffice')."
fi

# ---------------------------------------------------------------------------
# 3. Install the scripts into INSTALL_DIR
# ---------------------------------------------------------------------------
say "Installing scripts to $INSTALL_DIR"
if ! mkdir -p "$INSTALL_DIR" 2>/dev/null; then
    sudo mkdir -p "$INSTALL_DIR"
    sudo chown "$(id -u):$(id -g)" "$INSTALL_DIR"
fi

fetch() {  # fetch <filename>
    if [ -n "$SRC_DIR" ] && [ -f "$SRC_DIR/$1" ]; then
        cp "$SRC_DIR/$1" "$INSTALL_DIR/$1"
    else
        # Remote: try the new python/ layout first, then the legacy repo-root
        # path, so this works whether or not the repo has been restructured.
        curl -fsSL "$RAW_BASE/$RAW_SUBDIR/$1" -o "$INSTALL_DIR/$1" \
          || curl -fsSL "$RAW_BASE/$1" -o "$INSTALL_DIR/$1"
    fi
}

if [ "$RUST_MODE" = 1 ]; then
    asset="$(rust_cli_asset)"
    if [ -z "$asset" ]; then
        warn "No prebuilt Rust 'genie' for $(uname)/$(uname -m)."
        warn "Prebuilt CLIs exist only for x86_64-linux and aarch64-macOS."
        warn "For other targets, install the Python version instead: re-run with --python"
        warn "(or build the Rust binary from source in rust/)."
        exit 1
    fi
    say "Installing the Rust genie binary ($asset) from release $RUST_RELEASE..."
    curl -fsSL "$RUST_BASE/$asset" -o "$INSTALL_DIR/genie" \
        || { warn "Failed to download $RUST_BASE/$asset"; exit 1; }
else
    fetch genie
    fetch genie_rag.py
    fetch genie_graph.py
fi
chmod +x "$INSTALL_DIR/genie"

mkdir -p "$CACHE_DIR"   # vector cache lives here (recreated if deleted)

# Pick the default model from system RAM: <6GB -> e2b, >=6GB -> e4b.
if [ ! -s "$CACHE_DIR/model_default" ]; then
    if [ "$(uname)" = "Darwin" ]; then
        mem_bytes="$(sysctl -n hw.memsize 2>/dev/null || echo 0)"
    else
        mem_bytes="$(( $(awk '/MemTotal/{print $2}' /proc/meminfo 2>/dev/null || echo 0) * 1024 ))"
    fi
    mem_gb=$(( mem_bytes / 1024 / 1024 / 1024 ))
    if [ "$mem_gb" -ge 6 ]; then echo e4b > "$CACHE_DIR/model_default"; else echo e2b > "$CACHE_DIR/model_default"; fi
    say "Default model: $(cat "$CACHE_DIR/model_default") (system RAM: ${mem_gb}GB)"
fi

# ---------------------------------------------------------------------------
# 4. Symlink `gemma` onto the PATH
# ---------------------------------------------------------------------------
BIN_DIR="${GENIE_BIN_DIR:-$HOME/.local/bin}"
if { mkdir -p "$BIN_DIR" 2>/dev/null || [ -d "$BIN_DIR" ]; } && \
   ln -sf "$INSTALL_DIR/genie" "$BIN_DIR/genie" 2>/dev/null; then
    say "Linked $BIN_DIR/genie"
elif have sudo && sudo mkdir -p "$BIN_DIR" 2>/dev/null && \
     sudo ln -sf "$INSTALL_DIR/genie" "$BIN_DIR/genie" 2>/dev/null; then
    say "Linked $BIN_DIR/genie (sudo)"
else
    BIN_DIR="$HOME/.local/bin"
    mkdir -p "$BIN_DIR"
    ln -sf "$INSTALL_DIR/genie" "$BIN_DIR/genie"
    say "Linked $BIN_DIR/genie"
    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *) warn "Add $BIN_DIR to your PATH:  export PATH=\"$BIN_DIR:\$PATH\"" ;;
    esac
fi

# ---------------------------------------------------------------------------
# 5. Pre-download everything (so first real run is fast / works offline-ish)
# ---------------------------------------------------------------------------
# True if <repo>'s <file> (or repo snapshot) is already in the HF hub cache.
hf_cached() {  # repo [file]
    local repo="$1" file="${2:-}"
    local snaps="$HF_HOME/hub/models--${1//\//--}/snapshots"
    [ -d "$snaps" ] || return 1
    if [ -n "$file" ]; then
        [ -n "$(find "$snaps" -name "$file" 2>/dev/null | head -1)" ]
    else
        [ -n "$(find "$snaps" -mindepth 1 2>/dev/null | head -1)" ]
    fi
}

if [ "${GENIE_SKIP_PREWARM:-0}" != "1" ]; then
    say "Pre-fetching liteparse..."
    uvx --from liteparse==2.0.6 lit --help >/dev/null 2>&1 || warn "liteparse prefetch failed"

    if hf_cached "minishlab/potion-retrieval-32M"; then
        say "Embedder (model2vec) already cached — skipping."
    else
        say "Pre-fetching lancedb + model2vec embedder..."
        uvx --python 3.12 --with lancedb==0.33.0 --with model2vec==0.8.2 python - <<'PY' >/dev/null 2>&1 || warn "embedder prefetch failed"
from model2vec import StaticModel
StaticModel.from_pretrained("minishlab/potion-retrieval-32M")
import lancedb  # noqa: F401
PY
    fi

    say "Pre-fetching ladybug (graph correlation)..."
    uvx --python 3.12 --with ladybug==0.17.1 python -c "import ladybug" >/dev/null 2>&1 \
        || warn "ladybug prefetch failed"

    if [ "${GENIE_SKIP_MODELS:-0}" != "1" ]; then
        for spec in "${MODEL_SPECS[@]}"; do
            repo="${spec%%|*}"; rest="${spec#*|}"; file="${rest%%|*}"; size="${rest##*|}"
            say "Ensuring $repo (~${size}) is downloaded & checksum-verified..."
            # hf_hub_download is the source of truth: it resumes any partial /
            # aborted download, verifies the file's sha256 before finalizing it
            # in the cache, and returns instantly if it's already complete. This
            # avoids trusting a half-downloaded file just because it exists.
            uvx --with huggingface_hub python - "$repo" "$file" <<'PY' \
              || warn "Could not download/verify $repo (it will download on first use)."
import sys
from huggingface_hub import hf_hub_download
p = hf_hub_download(repo_id=sys.argv[1], filename=sys.argv[2])
print(f"  verified: {p}")
PY
        done
    fi

    # Detect & cache the compute backend (GPU if available, else CPU).
    if [ ! -s "$CACHE_DIR/backend" ]; then
        mp="$(find "$HF_HOME/hub" -name "gemma-4-E2B-it.litertlm" 2>/dev/null | head -1)"
        if [ -n "$mp" ]; then
            if uvx "litert-lm@${LITERT_VERSION}" run "$mp" --backend=gpu --max-num-tokens 64 --prompt "ok" >/dev/null 2>&1; then
                echo gpu > "$CACHE_DIR/backend"; say "Compute backend: gpu"
            else
                echo cpu > "$CACHE_DIR/backend"; say "Compute backend: cpu"
            fi
        fi
    fi
fi

# ---------------------------------------------------------------------------
# 6. Verify
# ---------------------------------------------------------------------------
say "Install complete. Verifying..."
if "$INSTALL_DIR/genie" --help >/dev/null 2>&1; then
    echo
    echo "  Gemma Genie is installed.  Try:"
    echo "    genie --ask \"hello\""
    echo "    genie --ask \"summarize this\" --doc report.pdf"
    echo "    genie --ask \"who owns project X?\" --dir ~/notes"
    echo
    echo "  Run 'genie --help' to see all options and a dependency check."
else
    warn "genie --help did not run cleanly; check the output above."
    exit 1
fi
