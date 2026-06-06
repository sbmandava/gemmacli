#!/usr/bin/env bash
#
# Gemma Genie installer — bootstraps everything on a fresh machine.
#
#   curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash
#
# Idempotent: safe to re-run. If you delete ~/.genie (the vector cache) or are on
# a brand-new laptop, re-running this brings everything back, including the
# Gemma model weights.
#
# Environment overrides:
#   GENIE_INSTALL_DIR   where the scripts live   (default: ~/.local/share/genie)
#   GENIE_BIN_DIR       where the `genie` link goes (default: ~/.local/bin, then /usr/local/bin)
#   GENIE_RAW_BASE      raw URL to fetch files from when piped via curl
#   HF_HOME             HuggingFace cache root (default: ~/.cache/huggingface) — all models go here
#   GENIE_SKIP_MODELS=1 skip downloading the (large) Gemma weights
#   GENIE_SKIP_PREWARM=1 skip all pre-downloads (deps still install)
#
set -euo pipefail

INSTALL_DIR="${GENIE_INSTALL_DIR:-$HOME/.local/share/genie}"
RAW_BASE="${GENIE_RAW_BASE:-https://raw.githubusercontent.com/sbmandava/gemma-genie/main}"
CACHE_DIR="$HOME/.genie"

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

# Where am I running from? (a local checkout vs. piped through curl)
SELF="${BASH_SOURCE[0]:-}"
SRC_DIR=""
if [ -n "$SELF" ] && [ -f "$SELF" ] && [ -f "$(dirname "$SELF")/genie" ]; then
    SRC_DIR="$(cd "$(dirname "$SELF")" && pwd)"
fi

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
# 2. LibreOffice (macOS) — needed by liteparse for DOCX/XLSX/PPTX
# ---------------------------------------------------------------------------
if [ "$(uname)" = "Darwin" ] && ! have soffice; then
    if ! have brew; then
        say "Installing Homebrew..."
        NONINTERACTIVE=1 /bin/bash -c \
          "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || \
          warn "Homebrew install failed"
        [ -x /opt/homebrew/bin/brew ] && eval "$(/opt/homebrew/bin/brew shellenv)"
        [ -x /usr/local/bin/brew ]    && eval "$(/usr/local/bin/brew shellenv)"
    fi
    if have brew; then
        say "Installing LibreOffice (for DOCX/XLSX/PPTX parsing)..."
        brew install --cask libreoffice || warn "LibreOffice install failed (PDF/images still work)"
    fi
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
        curl -fsSL "$RAW_BASE/$1" -o "$INSTALL_DIR/$1"
    fi
}
fetch genie
fetch genie_rag.py
fetch genie_graph.py
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
    uvx --from liteparse lit --help >/dev/null 2>&1 || warn "liteparse prefetch failed"

    if hf_cached "minishlab/potion-base-8M"; then
        say "Embedder (model2vec) already cached — skipping."
    else
        say "Pre-fetching lancedb + model2vec embedder..."
        uvx --python 3.12 --with lancedb --with model2vec python - <<'PY' >/dev/null 2>&1 || warn "embedder prefetch failed"
from model2vec import StaticModel
StaticModel.from_pretrained("minishlab/potion-base-8M")
import lancedb  # noqa: F401
PY
    fi

    say "Pre-fetching ladybug (graph correlation)..."
    uvx --python 3.12 --with ladybug python -c "import ladybug" >/dev/null 2>&1 \
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
            if uvx litert-lm run "$mp" --backend=gpu --max-num-tokens 64 --prompt "ok" >/dev/null 2>&1; then
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
