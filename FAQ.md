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

**Why business users care:**

- **Confidential by design.** Contracts, financials, HR files, board decks —
  ask questions about them without a single byte leaving your laptop.
- **Works on a plane, in a vault, or behind an air-gap.** After the one-time
  setup, there's no network dependency.
- **No per-question cost, no rate limits, no vendor lock-in.** Run it as much as
  you like.
- **It reads your real files.** PDFs, Word, Excel, PowerPoint, folders of
  documents — and answers in plain language, citing the source file.

Think of it as a private analyst that has read all your documents and never
sends them anywhere.

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

Fair. Here's our defense, with apologies to Dr. Seuss — *Green Vectors and
Knowledge Graph*:

> I do not trust this Genie thing.
> I do not trust the claims you bring.
>
> Would you, could you, on your Mac?
> With **Vectors** here — and nothing sent back?
> Would you, could you, doubt the hype,
> then watch a **Knowledge Graph** take shape?
>
> I will not pipe in my domain.
> I will not run it on a plane.
> I will not feed it doc or file.
> I will not trust it for a while!
>
> Try the **Vectors**. Try the **Graph**.
> No cloud, no key, no token math.
> Try them here, try them there —
> your data never leaves your chair.
>
> ...and then I tried, in my own den,
> a folder, screenshot, voice, and pen.
> It read them all, it linked, it found —
> and not one byte had left the ground!
>
> Say! I *like* green **Vectors** and the **Graph**!
> I like them — yes! — I'll run `genie` and laugh.
> I'll trust the Genie on my Mac…
> with **Vectors** out front and a **Graph** at the back.

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
> self-learning, privacy-first AI agents at the edge. Thank you for supporting
> and sponsoring this open project.

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
Compare that to cloud pricing that grows with every employee and every query.

### "Is it private enough for client or regulated data?"

Your files never leave the device — no third-party processor, no external
data-retention policy in the loop. That makes it a natural fit for NDAs, client-
confidential material, legal/financial/health documents, and air-gapped setups.
(You're still responsible for your own device security.)

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

### Will it work without a network — e.g. on an airplane?

**Yes.** Once the one-time install has completed and the models are cached,
`genie` works fully offline — no Wi-Fi required. The 24h update check simply
fails silently when there's no connection and doesn't affect answering.

If you want zero network attempts at all, set `GENIE_NO_UPDATE=1`.

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
| OS | macOS, Linux (Windows via WSL2, untested) | macOS (Apple Silicon) |
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
rm -rf ~/.cache/huggingface/hub/models--minishlab--potion-base-8M
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

---

### Is it really free / private?

Yes. Open-source models, run locally, no API keys, no per-query cost, and your
content stays on your machine. See [README](README.md) for details.
