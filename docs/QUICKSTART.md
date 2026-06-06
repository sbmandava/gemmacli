# Quickstart

Get from zero to answering questions about your own files in five steps.
Everything runs **on-device** — no cloud, no API keys.

## A. Install

One command bootstraps everything. **By default it installs the Rust
single-binary build** (no Python at runtime) plus the Gemma model weights, the
embedder, and a `genie` symlink on your `PATH`:

```bash
curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash
```

The installer is idempotent — re-run it any time to repair an install. First run
downloads several GB of model weights (network needed once); after that Genie
works fully offline.

### Prefer the bash + Python build?

Add `--python`:

```bash
curl -fsSL https://raw.githubusercontent.com/sbmandava/gemma-genie/main/install.sh | bash -s -- --python
```

Prebuilt Rust CLIs are published for **x86_64 Linux** and **Apple-Silicon
macOS**; other targets use `--python` (or build from
[rust/README.md](../rust/README.md)). Usage below is identical for both builds.

## B. Verify the install — `genie doctor`

Confirm all dependencies are present and the runtime is ready:

```bash
genie doctor
```

You should see `[ok]` lines for `uvx`, the model weights, and the helpers.
(Optional: `genie --verify-models` checksums the downloaded weights, and
`genie --version` prints the version.)

## C. Ask a general question

No files required — just ask:

```bash
genie --ask "why is the sky blue"
```

The first call warms up the model (a little slower); later calls are faster.

## D. Ingest a folder — `genie --dir`

Point Genie at a directory to recursively ingest every supported file
(PDF, DOCX, XLSX, PPTX, CSV, Markdown, text, images) into the local LanceDB
vector cache. It also builds an entity-correlation graph in the background.

```bash
genie --dir ~/Documents/project --ask "summarize the key points"
```

Supported files are chunked, embedded, and indexed. The cache lives under
`~/.genie` and entries expire after 24h of no use.

## E. Ask about what you ingested

A **bare** `--ask` (no `--dir`/`--doc`/`--txt`) automatically consults whatever
you indexed in the last 24h — the most relevant LanceDB chunks plus graph
correlations — to answer from your own content:

```bash
genie --ask "what deadlines are mentioned in the project files?"
```

Ask follow-ups the same way; everything indexed in the last 24h stays available.

---

### Handy extras

```bash
genie cache info               # inspect the vector cache
genie cache clear              # wipe the vector cache
genie --graph-stats            # entity-correlation graph stats
genie --doc report.pdf --ask "what are the conclusions?"   # one-off document
cat notes.txt | genie --ask "summarize this"               # piped input
```

See [README.md](../README.md) for the full reference and
[VERSIONS.md](VERSIONS.md) for pinned dependency versions.
