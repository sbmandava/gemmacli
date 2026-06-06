# Gemma Genie — FAQ

## Private AI on your own laptop — powered by Google Gemma 4 : E4B

`genie` puts a genuinely capable AI assistant **on your machine**, with **no
cloud, no accounts, and no internet required** once installed. It's built on
two pieces of Google innovation:

- **Gemma 4** — Google's open, efficient models, quantization-aware-trained to
  run fast on everyday laptops
  ([blog](https://blog.google/innovation-and-ai/technology/developers-tools/quantization-aware-training-gemma-4/)).
- **LiteRT-LM** — Google's on-device runtime that runs those models locally,
  using your GPU when available
  ([overview](https://developers.google.com/edge/litert-lm/overview)).

In short: a **private analyst that has read all your documents and never sends
them anywhere.** It reads your real files (PDF, Word, Excel, PowerPoint, whole
folders), answers in plain language **with sources**, runs on a plane or behind
an air-gap, and costs **nothing per question**.

---

## Why Genie — why not Alexa or Copilot?

Because they answer to someone else. **Alexa and Copilot are cloud services** —
they have no real powers of their own; every request travels to their provider's
servers, and they're ultimately built to keep that provider (and its per-token
meter) happy. Your data is what flows through them.

**Genie has exactly one owner: you.** It runs entirely on your laptop and exists
to empower you with the data *already sitting on your machine*. It can:

- **Listen to audio** you give it (`--audio`),
- **Read your files and entire folders** (`--txt` / `--doc` / `--dir`),
- **Look at images and screenshots** and help you decipher them (`--image`),

…and answer — **without sending anything to anyone else.** No cloud overlords,
no account, no telemetry, no meter running. The intelligence works for you, on
your terms, with your data staying yours.

---

## "I'm skeptical of all these Genie claims."

Fair. Here's our rebuttal, with apologies to Dr. Seuss — *Green Vectors and
Knowledge Graph*:

> "I will not trust this Genie elf.
> I'd sooner do the work myself.
> It cannot read, it cannot see —
> and surely it phones home on me?"
>
> Would you, could you, on a Mac?
> The **Vectors** hum, the **Graph** talks back!
> No cloud, no key, no monthly fee,
> no token meter watching thee.
>
> "I will not feed it my report.
> I will not trust some laptop sort.
> I will not pipe my secret stash
> to gremlins gnawing in my cache!"
>
> So try it here! So try it there!
> So try the **Vectors** anywhere!
> It read my folder, watched my screen —
> the sharpest search I've ever seen.
> It heard my voice, it linked the lot…
> then leaked it to the cloud? It did **not!**
>
> Well — look at me! I've changed my tune.
> I hum to **Graphs** beneath the moon.
> I like green **Vectors**! Yes indeed!
> The **Knowledge Graph** is all I need!
> So thank you, Genie, on my Mac —
> my data's mine, and it stayed back.

---

## What is the overarching goal of this project?

Gemma Genie is **sponsored by [Unovie.AI](https://unovie.ai/)** as a public
**testbed for a bigger idea**: that **self-learning AI agents can run on the
edge** — on your own laptop or device — and **get better over time** by learning
from the data and interactions they encounter locally, *without ever sending
anything to the cloud*.

This project demonstrates the core building blocks of such an agent:

- **On-device reasoning** with Google's Gemma 4 models via LiteRT-LM,
- a **local vector memory** (LanceDB) that remembers what it has read, and
- a **local knowledge graph** (LadybugDB) that captures how entities relate,

so the system can **accumulate, correlate, and reuse knowledge privately** and
improve as it sees more of your documents. It's an early, practical step toward
edge agents that **observe, remember, and learn** — entirely on-device.

These goals are laid out in Unovie.AI's **Edge AI whitepaper**:
<https://unovie.ai/resources/edge-ai-whitepaper>

> **Sponsored by [Unovie.AI](https://unovie.ai/)** — building toward
> self-learning, privacy-first AI agents at the edge.

---

## Why the answers are better: built-in Vector search + Knowledge Graph

A language model on its own can only "see" what fits in a single prompt and
tends to guess when documents are large or span many files. `genie` avoids that
by combining **two complementary memories**, both running locally:

- **Vector search (LanceDB).** Every document is split into chunks and embedded,
  so when you ask a question `genie` retrieves the *semantically most relevant*
  passages — even across hundreds of pages and many files — and shows only those
  to the model. This keeps answers grounded in your actual text and within the
  model's context window. Huge thanks to the **[LanceDB](https://lancedb.github.io/lancedb/)**
  team — especially **[Prashant Rao](https://ca.linkedin.com/in/prrao87)**, a
  truly innovative leader from Toronto who teaches the world through his blog
  [The Data Quarry](https://thedataquarry.com/) — for a fast, embeddable vector
  database.

- **Knowledge graph (LadybugDB).** As files are indexed, `genie` also extracts
  the people, projects, organizations, and products and stores how they relate
  in a `(:File)-[:Mentions]->(:Entity)` graph. This lets it answer *relationship*
  questions — who owns what, which vendor built which system, what connects two
  documents — that pure text search misses. Hat tip to the
  **[LadybugDB](https://github.com/LadybugDB/ladybug)** team for an embeddable
  Cypher graph database.

**Together** they make responses noticeably more accurate: the vector store
finds the right evidence, and the graph disambiguates how things are connected
(e.g. distinguishing the *builder* of a system from the *customer* it's for).
Every `genie --ask` automatically consults both — no extra steps.

---

## For startup & small-business owners

### "Don't I need OpenAI or Gemini to do this?"

**No — not for everyday document work.** Summarizing contracts, extracting action
items, answering questions across a folder of files, pulling figures out of
spreadsheets — a modern on-device model like **Gemma 4 E4B** handles these well,
right on your laptop. No cloud account, no API key, no credit card.

### "Wait — AI can run offline, without the cloud?"

Yes. Many people assume "AI = ChatGPT in a browser tab." But capable models now
run **locally on your own hardware**. Gemma Genie downloads the model once, then
every question runs on your machine — on a plane, in a clinic, behind a corporate
firewall — with nothing sent anywhere.

### "What does it cost to run?"

**$0 per question.** No subscription, no per-token API bills, no usage metering.
The only cost is the laptop you already own plus a one-time model download.

And here's the part founders should plan for: **token costs are heading up, not
down.** As AI gets woven into every feature, per-token API spend becomes a
**variable cost that scales with your usage and your users** — great for the
provider, brutal for your margins and your roadmap. If your product's
intelligence lives behind someone's meter, **every new feature is a new bill**
and your unit economics are hostage to their pricing.

**Offline Edge AI flips that.** The cost is fixed (the device), the model is
yours to ship, and you can **build, iterate, and demo features for free** — even
offline. For startups, on-device intelligence isn't just cheaper; it's a
**strategic moat for product innovation**: no rate limits to design around, no
data-sharing to negotiate, and no surprise invoice when a feature goes viral.

### "When would I still want a cloud model (OpenAI / Gemini)?"

Being honest: cloud frontier models still lead for the hardest reasoning,
very large context windows, the latest world knowledge, image generation, and
massive batch jobs. Gemma Genie targets the **~80% of everyday document tasks**
you can do privately and for free — keep the cloud for the heavy, non-sensitive
20% when you truly need it. Plenty of businesses run both.

### "Do I need an AI team or a special server?"

No. One `curl` command installs everything, and a typical MacBook is enough — no
GPUs to provision, no infrastructure to maintain, no fine-tuning required.

---

## Why offline matters: security, governance, privacy & compliance

Running **entirely on-device** removes the single biggest risk in most AI tools:
**your data leaving for someone else's servers.** That changes the conversation
for regulated and security-conscious teams.

- **Data residency & sovereignty.** Nothing leaves the laptop, so data never
  crosses borders or lands in a vendor cloud — simplifying GDPR, data-residency,
  and sovereignty requirements.
- **Confidentiality & NDAs.** Client-confidential, M&A, legal, and board
  material can be analyzed with **no third-party processor or sub-processor** in
  the chain.
- **Regulated industries.** A fit where third-party data sharing is restricted —
  healthcare (PHI/HIPAA), finance (PCI/SOX/MNPI), legal privilege,
  government/defense, and IP-sensitive R&D.
- **No training on your data.** Your documents are never used to train anyone's
  model, and there's no vendor-side prompt/response logging.
- **Air-gapped & field use.** Works with no network at all — secure facilities,
  classified environments, ships, planes, and remote sites.
- **Minimal, auditable footprint.** A small local index that **auto-expires
  after 24h** means less data at rest to govern; you control what's indexed and
  when it's purged (`genie cache clear`, `rm -rf ~/.genie`).
- **No vendor lock-in or outages.** No API keys to rotate, no surprise per-token
  bills, no dependency on a provider's uptime or shifting data policies.

> **Note:** Gemma Genie keeps *your data on-device*, which removes a major
> compliance risk — but it is **not a certification**. You remain responsible for
> device security, access control, and your organization's overall compliance.

---

## Working examples

```bash
# Ask a quick question (no documents needed)
genie --ask "Explain the difference between OPEX and CAPEX in one paragraph"

# Summarize a contract / report (PDF, Word, Excel, PowerPoint)
genie --ask "Summarize the key obligations and renewal terms" --doc MSA_2026.pdf
genie --ask "What are the top 3 risks called out here?" --doc board_deck.pptx
genie --ask "Which line items exceed budget, and by how much?" --doc budget.xlsx

# Turn a whole folder into a searchable, private knowledge base
genie --ask "What's our PTO policy and who approves it?" --dir ~/CompanyDocs

# After indexing, just ask — it remembers what you've shown it (last 24h)
genie --ask "Who owns the Apollo project and which vendor built it?"

# See how concepts/people/projects connect across all your documents
genie --graph-stats
genie --graph-query "MATCH (f:File)-[:Mentions]->(e:Entity) RETURN f.name, e.name LIMIT 20"

# Pipe text in from anything
pbpaste | genie --ask "Rewrite this as a polite customer email"
cat meeting_notes.txt | genie --ask "Extract action items with owners"

# Check what's running on your machine
genie doctor
```

Every one of these runs entirely offline after install. Your files stay yours.

---

### How does this program work?

`genie` runs Google's **Gemma 4** models **on your own machine** via
[`litert-lm`](https://github.com/google-ai-edge/litert-lm). When you analyze a
file or folder it:

1. **Extracts text** — plain text/CSV directly; PDF/DOCX/XLSX/PPTX/images via
   [liteparse](https://pypi.org/project/liteparse/).
2. **Indexes it** into a local **LanceDB** vector store (chunks + embeddings via
   `model2vec`) and a **LadybugDB** entity-correlation graph.
3. **Answers** your question by retrieving the most relevant chunks + graph
   relationships and feeding only those to the local Gemma model.

A bare `genie --ask "..."` (no file) automatically answers from whatever you've
indexed in the last 24h. Everything runs locally.

---

### Are you fine-tuning the model?

**Not right now** — but it's firmly on the roadmap. Gemma Genie is currently
**laying the foundation**: a clean, private, on-device pipeline (model +
vector memory + knowledge graph) that just works. Fine-tuning and personalization
are intended for **upcoming releases**.

With enough user interest on this topic, we plan to add those features while
continuing to optimize the underlying system toward a more **deterministic,
agentic system** — one that learns from your data and behaves predictably, all
on the edge.

---

### Is it built on Ollama or llama.cpp?

**No.** Gemma Genie runs on **Google's LiteRT-LM** — the on-device LLM runtime
built on **LiteRT, the lightweight successor to TensorFlow Lite** for edge
devices. It's a lean, edge-first stack designed to run efficiently on **laptops
and even cell phones**, not just heavyweight servers.

That edge focus is exactly the point: the same kind of engine that powers
on-device AI in phones runs your private assistant here. Huge thanks to
**Google** and **DeepMind** for these innovations — and with their pace, it's
only going to get better.

---

### Why are we planning to move to Rust?

Today's `genie` is a bash orchestrator plus Python helpers, run via `uv`. It
works great — but for a security-first, edge-first agent, we're rewriting it as a
**single, self-contained Rust binary**. Why:

- **Security & safety.** Rust is **memory-safe** by design — it eliminates whole
  classes of bugs (buffer overflows, use-after-free, data races) at compile time.
  For a tool meant to run on confidential, regulated, and air-gapped data, a
  hardened core is the right foundation.
- **Faster, leaner, better.** One native binary means **no Python runtime, no
  `uv`/`pip` at runtime, faster startup, and a smaller footprint** — exactly what
  you want on a laptop or phone. Several of our dependencies (LanceDB, liteparse,
  LadybugDB, model2vec) are *already Rust at the core*, so going native removes
  layers rather than adding them.
- **Safeguard against supply-chain attacks.** A compiled binary with **pinned,
  vendored, auditable dependencies** is a far smaller attack surface than pulling
  dozens of transitive Python packages from the network on each run. Fewer moving
  parts, nothing fetched at runtime, easier to verify end to end.
- **A path to WebAssembly.** The same Rust core can compile to **WASM** — opening
  the door to running Genie's intelligence in a **browser tab or other sandboxed
  environments**, fully on-device, with no install at all. :)
- **Native Windows in the future.** A single Rust binary cross-compiles cleanly,
  so beyond today's macOS focus (with Linux and Windows/WSL2 in alpha) we may
  ship a **true native Windows build** — no WSL2 required — bringing the same
  private, offline assistant to more machines.

We're doing this incrementally: the proven bash + Python version keeps shipping
(now under `python/`) while the Rust rewrite grows alongside it (under `rust/`),
so nothing breaks while we level up the foundation.

---

### Can I install Genie with Cargo?

Yes. The Rust **installer** is published on
[crates.io](https://crates.io/crates/genie-bootstrap) as `genie-bootstrap`. If
you have a Rust toolchain:

```bash
cargo install genie-bootstrap
genie-bootstrap --install
```

`genie-bootstrap` is a tiny, dependency-light binary that probes your
environment (OS, arch, GPU, RAM) and fetches **only** the components your
machine needs — the matching prebuilt `genie` CLI, one model variant by RAM, and
the embedder — then places `genie` on your `PATH`. It's the same prebuilt binary
the `curl | bash` installer uses, just delivered through Cargo.

Note: the `genie` CLI itself is **not** on crates.io. It depends on local/path
crates (LanceDB, Lance, model2vec-rs, liteparse, LadybugDB) that crates.io
doesn't permit in a published crate, so end users get the prebuilt binary (via
`genie-bootstrap`, the `curl` installer, or GitHub Releases) and developers build
it from [`rust/`](../rust/).

---

### Does my data leave my laptop?

**No.** Your documents, questions, embeddings, and the graph never leave the
machine. There is no cloud API, no account, and no telemetry. All processing
(extraction, embedding, retrieval, and the model itself) happens on-device, and
all data stays under `~/.genie/` and the local HuggingFace cache.

The **only** network usage is:
- the **one-time install** (downloading `uv`, Python packages, and the Gemma
  model weights), and
- a **once-per-24h version check** against the public GitHub repo (just to
  auto-upgrade the scripts — your data is never sent). Disable it with
  `GENIE_NO_UPDATE=1`.

---

### Will it work with PDF, Word, Excel — any file?

Supported out of the box:

| Type | Extensions | How |
|------|-----------|-----|
| Text / data | `.txt .csv .tsv .md .log .json .yaml .yml .rst` | read directly |
| PDF | `.pdf` | liteparse (text layer) |
| Word | `.docx .doc` | liteparse + LibreOffice |
| Excel | `.xlsx .xls` | liteparse + LibreOffice |
| PowerPoint | `.pptx .ppt` | liteparse + LibreOffice |
| Images | `.png .jpg .jpeg .tiff .bmp .gif .webp` | liteparse / vision |

Notes:
- **Office formats (Word/Excel/PowerPoint)** need **LibreOffice** installed (the
  installer handles this on macOS). PDF, images, and text files do not.
- **Scanned/image-only PDFs** have no text layer — they need OCR (use `--llm`
  or an image flow); a plain text extract will be empty.
- Not every binary format is supported. If a file type isn't in the table above,
  it's skipped during `--dir` ingestion.

Usage:
```bash
genie --ask "summarize the risks" --doc report.pdf
genie --ask "who owns each project?" --doc plan.xlsx
genie --ask "what changed?" --dir ~/project-docs
```

---

### What are my laptop requirements?

**Preferred hardware:** an **Apple MacBook with an M-series chip (M1/M2/M3/M4)
and 16 GB of RAM** — though **8 GB works just as well** for everyday use. The
unified memory and GPU give the smoothest experience and run the stronger **e4b**
model comfortably.

**No idle footprint.** Gemma Genie is **not a daemon** — there's no resident
process and **zero memory/CPU usage when you're not using it**. It only runs
while a `genie` command is executing: it loads the model, answers, and exits,
freeing all memory. (Indexing kicks off a short background job to update the
graph, but that finishes and exits on its own — nothing stays resident.)

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| OS | macOS (Linux & Windows/WSL2 are alpha) | macOS (Apple Silicon) |
| RAM | ~4 GB (uses the smaller **e2b** model) | 16 GB (runs **e4b** comfortably) |
| Disk | ~6–8 GB free (model weights + caches) | 10 GB+ |
| GPU | none (CPU works) | Apple M-series / supported GPU |

- The default model is chosen automatically from RAM: **< 6 GB → e2b**,
  **≥ 6 GB → e4b**. Override anytime with `--model e2b|e4b` or `GENIE_MODEL`.
- GPU is auto-detected with automatic CPU fallback. Check what you're on with
  `genie doctor`.

---

### How fast is it, and will it get faster?

Today, most of the wait is the model **loading on each command** (and, for
documents, the one-time extraction + embedding). Answers themselves stream
quickly once the model is up.

We're **actively optimizing the backend** — faster model loading/serving,
smarter caching, and leaner retrieval — to make it **significantly quicker**.
And because Gemma Genie **auto-updates** (it checks the repo and upgrades itself,
at most once a day), **you get these speedups automatically** as we ship them —
no reinstall needed. The program is designed to **keep getting better on its own**.

Tips for snappier runs today:
- Use `--model e2b` for quick, lightweight questions.
- Keep `--dir` folders focused; re-indexing only touches changed files.
- On a GPU machine, make sure `genie doctor` shows `compute backend: gpu`.

---

### Why does the knowledge expire after a day?

Both the vector cache (LanceDB) and the knowledge graph (LadybugDB) **auto-expire
after 24 hours of being idle**, then rebuild from your files on demand.

This is **intentional**. Gemma Genie is a **pilot project** exploring self-learning
edge agents, and for that we deliberately want to **keep the ideas fresh without
the baggage of stale, old information**:

- **Freshness over accumulation.** An agent that hoards everything it ever saw
  starts answering from outdated context. A short, rolling memory keeps responses
  anchored to what you're actually working on now.
- **Privacy hygiene.** Indexed content (and the entities extracted from it) doesn't
  linger on disk indefinitely — it clears itself if you stop using it.
- **Clean experiments.** As a testbed, we want each session to reflect current
  data, not artifacts from last week's documents.

Nothing is lost: your **original files are untouched**, and re-asking simply
re-indexes them. You can tune or disable expiry with `GENIE_CACHE_TTL` (seconds),
e.g. `GENIE_CACHE_TTL=604800` for 7 days, or clear it yourself anytime (below).

---

### How big are the indexes / knowledge graph?

**Tiny.** Both the LanceDB vector index and the LadybugDB knowledge graph are
**highly compressed, columnar indexes** — they store compact embeddings and
entity relationships, not copies of your files. For a typical few-document
corpus the whole footprint is **well under a megabyte** (e.g. a 4-document set
here uses ~220 KB for the vector index and ~590 KB for the graph).

So it **barely consumes anything day to day**, and because everything
**auto-cleans after 24h of being idle** (see above), it never grows unbounded —
it stays small and fresh on its own. The only sizeable storage is the one-time
Gemma model download in the HuggingFace cache (a few GB), which is shared and
not part of the per-use index.

---

### How do I flush the cache and start over?

The caches are safe to delete — they rebuild on demand.

```bash
# Clear just the vector cache (indexed document chunks)
genie cache clear

# Or wipe everything genie stores (vector cache, graph, backend/model choice,
# update timestamp) and start completely fresh:
rm -rf ~/.genie

# Re-running any command (or the installer) recreates what's needed.
```

To also remove the downloaded **model weights** (frees several GB):
```bash
rm -rf ~/.cache/huggingface/hub/models--litert-community--gemma-4-*
rm -rf ~/.cache/huggingface/hub/models--minishlab--potion-retrieval-32M
```
They re-download on next use (needs network once).

Both the vector cache and the graph also **auto-expire after 24h idle**, so
stale data clears itself over time.

---

### Where is everything stored?

| Path | What |
|------|------|
| `~/.genie/genie-cache.db/` | LanceDB vector cache |
| `~/.genie/genie-graph.lbug` | LadybugDB correlation graph |
| `~/.genie/backend`, `~/.genie/model_default` | detected GPU/CPU + default model |
| `~/.cache/huggingface/hub/` | model weights (Gemma + embedder) |
| `/opt/projects/unovie/gemmacli/` (or your install dir) | the scripts |

---

### How do I control which model / backend is used?

```bash
genie --model e2b --ask "..."     # force the small, fast model
GENIE_MODEL=e4b genie --ask "..." # force the stronger model
GENIE_BACKEND=cpu genie --ask ... # force CPU
genie doctor                      # show detected backend + default model + RAM
```
