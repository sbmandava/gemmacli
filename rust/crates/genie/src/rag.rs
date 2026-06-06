//! Retrieval: model2vec-rs embeddings + lancedb vector store.
//! M2: single-doc (--doc/--txt) with threshold gating + Sources footer.
//! M3: directory mode (incremental), search-all (bare --ask), TTL eviction,
//!     and the `cache` subcommand.

use crate::cli::Cli;
use crate::config::{Config, EMBED_MODEL};
use crate::llm;
use crate::parse;
use anyhow::{bail, Result};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use arrow_array::cast::AsArray;
use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use model2vec_rs::model::StaticModel;

const TEXT_EXTS: &[&str] = &[
    "txt", "text", "csv", "tsv", "md", "markdown", "log", "json", "yaml", "yml", "rst",
];
const DOC_EXTS: &[&str] = &[
    "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "png", "jpg", "jpeg", "tiff", "tif", "bmp",
    "gif", "webp",
];

// ===========================================================================
// Entry points
// ===========================================================================

/// Document-grounded ask: --doc/--txt (single doc) or --dir (knowledge base).
pub fn ask(question: &str, cli: &Cli, cfg: &Config) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let (prompt, sources) = if let Some(dir) = &cli.dir {
        rt.block_on(prepare_dir(question, dir, cfg))?
    } else {
        rt.block_on(prepare_doc(question, cli, cfg))?
    };
    drop(rt);
    llm::generate(cfg, prompt)?;
    print_sources(&sources);
    Ok(())
}

/// Bare `--ask` with no inputs: consult the vector KB indexed in the last TTL.
/// Returns Ok(true) if it answered from the KB, Ok(false) if there was nothing
/// indexed (caller should fall back to a plain model answer).
pub fn ask_kb(question: &str, cfg: &Config) -> Result<bool> {
    let excerpts = if cfg.cache_db.exists() {
        let rt = tokio::runtime::Runtime::new()?;
        let e = rt.block_on(search_all(cfg, question))?;
        drop(rt);
        e
    } else {
        Vec::new()
    };
    let graph_ctx = crate::graph::correlate(cfg, question);

    if excerpts.is_empty() && graph_ctx.is_none() {
        return Ok(false);
    }

    let mut ctx = String::new();
    let mut sources = Vec::new();
    for (source, chunk) in &excerpts {
        ctx.push_str(&format!("[source: {source}]\n{chunk}\n\n"));
        if !sources.contains(source) {
            sources.push(source.clone());
        }
    }
    let graph_block = match &graph_ctx {
        Some(g) => format!("\nKnowledge-graph relationships:\n{g}\n"),
        None => String::new(),
    };
    let prompt = format!(
        "{question}\n\nAnswer using the knowledge-graph relationships and document \
         excerpts below (retrieved from the user's indexed data). Ground every claim \
         in this evidence, pay attention to source file names, and use the graph \
         relationships to disambiguate how entities relate.\n{graph_block}\nDocument excerpts:\n{ctx}"
    );
    llm::generate(cfg, prompt)?;
    print_sources(&sources);
    Ok(true)
}

/// `cache [info|list|clear]`.
pub fn cache(action: &str, cfg: &Config) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(cache_impl(action, cfg))
}

// ===========================================================================
// Single document (M2)
// ===========================================================================

async fn prepare_doc(question: &str, cli: &Cli, cfg: &Config) -> Result<(String, Vec<String>)> {
    let path = cli
        .doc
        .as_ref()
        .or(cli.txt.as_ref())
        .ok_or_else(|| anyhow::anyhow!("no --doc/--txt path"))?;
    let src_disp = abspath(path);
    let text = parse::extract(path, cli.pages.as_deref()).await?;

    // Update the correlation graph from this file's text (best-effort).
    let _ = crate::graph::update_file_text(cfg, &src_disp, &file_sig(path), &text);

    if text.len() <= cfg.rag_threshold {
        let prompt =
            format!("{question}\n\nAnalyze the following document (\"{src_disp}\"):\n\n{text}");
        return Ok((prompt, vec![src_disp]));
    }

    eprintln!(
        "Large input ({} chars) — retrieving relevant chunks via LanceDB...",
        text.len()
    );
    let model = load_model()?;
    let db = connect(cfg).await?;
    prune_expired(&db, cfg).await;

    let cache_key = format!("{src_disp}|{}|cs={}", file_sig(path), cfg.chunk_size);
    let name = table_name("doc", &cache_key);
    let chunks = chunk_text(&text, cfg.chunk_size, 150);
    if !db.table_names().execute().await?.contains(&name) {
        let sig = file_sig(path);
        add_chunks(&db, &name, &chunks, &src_disp, &sig, &model, true).await?;
    }
    touch(cfg, &name);

    let excerpts = query_table(&db, &name, question, cfg.rag_topk, &model).await?;
    Ok(build_prompt(question, &src_disp, excerpts))
}

// ===========================================================================
// Directory knowledge base (M3, incremental)
// ===========================================================================

async fn prepare_dir(question: &str, dir: &Path, cfg: &Config) -> Result<(String, Vec<String>)> {
    let dir_abs = abspath(dir);
    eprintln!("Indexing \"{dir_abs}\" into LanceDB (incremental)...");
    let model = load_model()?;
    let db = connect(cfg).await?;
    prune_expired(&db, cfg).await;

    let name = table_name("dir", &dir_abs);
    ingest_dir(&db, &name, dir, cfg, &model).await?;
    touch(cfg, &name);

    let excerpts = query_table(&db, &name, question, cfg.rag_topk, &model).await?;
    let prompt = format!(
        "{question}\n\nUse the following excerpts from files in \"{dir_abs}\" to answer \
         (each excerpt is labeled with its source file):\n\n{}",
        excerpts
            .iter()
            .map(|(s, c)| format!("[source: {s}]\n{c}\n"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let mut sources = Vec::new();
    for (s, _) in &excerpts {
        if !sources.contains(s) {
            sources.push(s.clone());
        }
    }
    Ok((prompt, sources))
}

/// Incrementally sync a directory into its LanceDB table using a sidecar
/// source->sig map (avoids scanning the table). Re-embeds only changed/new
/// files and drops rows for deleted ones.
async fn ingest_dir(
    db: &lancedb::Connection,
    name: &str,
    dir: &Path,
    cfg: &Config,
    model: &StaticModel,
) -> Result<()> {
    let files = walk_files(dir);
    let mut sigs = load_sigs(cfg, name);

    let exists = db.table_names().execute().await?.contains(&name.to_string());
    let tbl = if exists {
        Some(db.open_table(name).execute().await?)
    } else {
        None
    };

    let current: HashMap<String, String> = files
        .iter()
        .map(|f| (abspath(f), file_sig(f)))
        .collect();

    // Removed files: drop their rows.
    let removed: Vec<String> = sigs.keys().filter(|s| !current.contains_key(*s)).cloned().collect();
    if let Some(t) = &tbl {
        for src in &removed {
            let _ = t.delete(&format!("source = '{}'", sql_escape(src))).await;
            sigs.remove(src);
        }
    } else {
        for src in &removed {
            sigs.remove(src);
        }
    }

    // New/changed files: (re)embed.
    let mut created = exists;
    for f in &files {
        let src = abspath(f);
        let sig = file_sig(f);
        if sigs.get(&src) == Some(&sig) {
            continue;
        }
        let text = match parse::extract(f, None).await {
            Ok(t) if !t.trim().is_empty() => t,
            _ => continue,
        };
        let chunks = chunk_text(&text, cfg.chunk_size, 150);
        if chunks.is_empty() {
            continue;
        }
        if created {
            // delete any stale rows for this source, then add
            if let Some(t) = &tbl {
                let _ = t.delete(&format!("source = '{}'", sql_escape(&src))).await;
            }
            add_chunks(db, name, &chunks, &src, &sig, model, false).await?;
        } else {
            add_chunks(db, name, &chunks, &src, &sig, model, true).await?;
            created = true;
        }
        // Update the correlation graph for this (re)embedded file (best-effort).
        let _ = crate::graph::update_file_text(cfg, &src, &sig, &text);
        sigs.insert(src, sig);
    }
    save_sigs(cfg, name, &sigs);
    Ok(())
}

// ===========================================================================
// LanceDB helpers
// ===========================================================================

async fn connect(cfg: &Config) -> Result<lancedb::Connection> {
    Ok(lancedb::connect(&cfg.cache_db.to_string_lossy()).execute().await?)
}

fn schema(dim: usize) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("text", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("sig", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), dim as i32),
            true,
        ),
    ]))
}

fn batch(chunks: &[String], src: &str, sig: &str, embs: &[Vec<f32>], dim: usize) -> Result<RecordBatch> {
    let texts = StringArray::from(chunks.to_vec());
    let srcs = StringArray::from(vec![src.to_string(); chunks.len()]);
    let sigs = StringArray::from(vec![sig.to_string(); chunks.len()]);
    let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        embs.iter().map(|e| Some(e.iter().map(|&x| Some(x)).collect::<Vec<_>>())),
        dim as i32,
    );
    Ok(RecordBatch::try_new(
        schema(dim),
        vec![Arc::new(texts), Arc::new(srcs), Arc::new(sigs), Arc::new(vectors)],
    )?)
}

/// Embed `chunks` and either create the table (first batch) or add to it.
async fn add_chunks(
    db: &lancedb::Connection,
    name: &str,
    chunks: &[String],
    src: &str,
    sig: &str,
    model: &StaticModel,
    create: bool,
) -> Result<()> {
    let embs = model.encode(&chunks.to_vec());
    let dim = embs.first().map(|v| v.len()).unwrap_or(0);
    if dim == 0 {
        bail!("embedder returned no vectors");
    }
    let rb = batch(chunks, src, sig, &embs, dim)?;
    if create {
        db.create_table(name, rb).execute().await?;
    } else {
        let tbl = db.open_table(name).execute().await?;
        tbl.add(rb).execute().await?;
    }
    Ok(())
}

async fn query_table(
    db: &lancedb::Connection,
    name: &str,
    query: &str,
    topk: usize,
    model: &StaticModel,
) -> Result<Vec<(String, String)>> {
    if !db.table_names().execute().await?.contains(&name.to_string()) {
        return Ok(vec![]);
    }
    let tbl = db.open_table(name).execute().await?;
    let qv = model.encode_single(query);
    let results: Vec<RecordBatch> = tbl
        .query()
        .limit(topk)
        .nearest_to(qv.as_slice())?
        .execute()
        .await?
        .try_collect()
        .await?;
    Ok(rows_to_excerpts(&results))
}

/// Search every table in the cache and return the globally top-k chunks.
async fn search_all(cfg: &Config, query: &str) -> Result<Vec<(String, String)>> {
    let db = connect(cfg).await?;
    prune_expired(&db, cfg).await;
    let model = load_model()?;
    let qv = model.encode_single(query);
    let mut scored: Vec<(f32, String, String)> = Vec::new();
    for name in db.table_names().execute().await? {
        let tbl = match db.open_table(&name).execute().await {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Skip any table we can't query — e.g. a foreign/older cache whose
        // vector dimension differs from this embedder — instead of failing the
        // whole search.
        let query = match tbl.query().limit(cfg.rag_topk).nearest_to(qv.as_slice()) {
            Ok(q) => q,
            Err(_) => continue,
        };
        let stream = match query.execute().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let results: Vec<RecordBatch> = match stream.try_collect().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        for rb in &results {
            let texts = match rb.column_by_name("text") {
                Some(c) => c.as_string::<i32>(),
                None => continue,
            };
            let srcs = match rb.column_by_name("source") {
                Some(c) => c.as_string::<i32>(),
                None => continue,
            };
            let dist = rb
                .column_by_name("_distance")
                .map(|c| c.as_primitive::<Float32Type>().clone());
            for i in 0..rb.num_rows() {
                let d = dist.as_ref().map(|a| a.value(i)).unwrap_or(0.0);
                scored.push((d, srcs.value(i).to_string(), texts.value(i).to_string()));
            }
        }
        touch(cfg, &name);
    }
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(cfg.rag_topk);
    Ok(scored.into_iter().map(|(_, s, t)| (s, t)).collect())
}

fn rows_to_excerpts(results: &[RecordBatch]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for rb in results {
        let (texts, srcs) = match (rb.column_by_name("text"), rb.column_by_name("source")) {
            (Some(t), Some(s)) => (t.as_string::<i32>(), s.as_string::<i32>()),
            _ => continue,
        };
        for i in 0..rb.num_rows() {
            out.push((srcs.value(i).to_string(), texts.value(i).to_string()));
        }
    }
    out
}

// ===========================================================================
// cache subcommand + TTL
// ===========================================================================

async fn cache_impl(action: &str, cfg: &Config) -> Result<()> {
    match action {
        "clear" => {
            if cfg.cache_db.exists() {
                std::fs::remove_dir_all(&cfg.cache_db).ok();
            }
            std::fs::remove_dir_all(meta_dir(cfg)).ok();
            println!("Cleared vector cache at {}", cfg.cache_db.display());
        }
        "list" | "info" => {
            if !cfg.cache_db.exists() {
                println!("No cache at {} (nothing indexed yet).", cfg.cache_db.display());
                return Ok(());
            }
            let db = connect(cfg).await?;
            let names = db.table_names().execute().await?;
            println!("Vector cache: {}", cfg.cache_db.display());
            println!("Tables: {}", names.len());
            if action == "list" {
                for n in &names {
                    let tbl = db.open_table(n).execute().await?;
                    let rows = tbl.count_rows(None).await.unwrap_or(0);
                    println!("  - {n}  ({rows} chunks)");
                }
            }
            println!("TTL: {}s (idle tables auto-expire)", cfg.rag_ttl);
        }
        other => bail!("unknown cache action '{other}' (use info|list|clear)"),
    }
    Ok(())
}

/// Drop tables idle longer than the TTL.
async fn prune_expired(db: &lancedb::Connection, cfg: &Config) {
    let usage = load_usage(cfg);
    let now = now_secs();
    let mut changed = false;
    let mut usage = usage;
    if let Ok(names) = db.table_names().execute().await {
        for n in names {
            if let Some(&last) = usage.get(&n) {
                if now.saturating_sub(last) > cfg.rag_ttl {
                    let _ = db.drop_table(&n, &[]).await;
                    let _ = std::fs::remove_file(sigs_path(cfg, &n));
                    usage.remove(&n);
                    changed = true;
                }
            }
        }
    }
    if changed {
        save_usage(cfg, &usage);
    }
}

fn touch(cfg: &Config, name: &str) {
    let mut usage = load_usage(cfg);
    usage.insert(name.to_string(), now_secs());
    save_usage(cfg, &usage);
}

// ===========================================================================
// Small helpers
// ===========================================================================

fn build_prompt(question: &str, src_disp: &str, excerpts: Vec<(String, String)>) -> (String, Vec<String>) {
    let mut ctx = String::new();
    let mut sources = Vec::new();
    for (source, chunk) in &excerpts {
        ctx.push_str(&format!("[source: {source}]\n{chunk}\n\n"));
        if !sources.contains(source) {
            sources.push(source.clone());
        }
    }
    let prompt =
        format!("{question}\n\nUse the following excerpts from \"{src_disp}\" to answer:\n\n{ctx}");
    (prompt, sources)
}

fn load_model() -> Result<StaticModel> {
    StaticModel::from_pretrained(EMBED_MODEL, None, None, None)
        .map_err(|e| anyhow::anyhow!("failed to load embedder: {e}"))
}

/// Overlapping char-window chunks (trimmed); paragraph-aware enough for retrieval.
pub fn chunk_text(text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        let t = text.trim();
        return if t.is_empty() { vec![] } else { vec![t.to_string()] };
    }
    let overlap = overlap.min(max_chars / 4);
    let step = max_chars.saturating_sub(overlap).max(1);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + max_chars).min(chars.len());
        let s: String = chars[i..end].iter().collect();
        let t = s.trim();
        if !t.is_empty() {
            out.push(t.to_string());
        }
        if end == chars.len() {
            break;
        }
        i += step;
    }
    out
}

fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = match std::fs::read_dir(&d) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(ext) = p.extension().and_then(|x| x.to_str()) {
                let ext = ext.to_lowercase();
                if TEXT_EXTS.contains(&ext.as_str()) || DOC_EXTS.contains(&ext.as_str()) {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn abspath(p: &Path) -> String {
    std::fs::canonicalize(p)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string_lossy().into_owned())
}

fn table_name(prefix: &str, key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("{prefix}_{:016x}", h.finish())
}

fn file_sig(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(m) => {
            let mtime = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("{}-{}", m.len(), mtime)
        }
        Err(_) => "0".to_string(),
    }
}

fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn print_sources(sources: &[String]) {
    if sources.is_empty() {
        return;
    }
    println!("\nSources:");
    for s in sources {
        println!("  - {s}");
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// --- tiny line-based persistence (avoids a serde dep) ---
// Co-located with the cache DB (not genie_dir) so they reset together even when
// GENIE_CACHE_DB points elsewhere.

fn meta_dir(cfg: &Config) -> PathBuf {
    cfg.cache_db.with_extension("meta")
}

fn usage_path(cfg: &Config) -> PathBuf {
    meta_dir(cfg).join("usage.txt")
}

fn load_usage(cfg: &Config) -> HashMap<String, u64> {
    let mut m = HashMap::new();
    if let Ok(s) = std::fs::read_to_string(usage_path(cfg)) {
        for line in s.lines() {
            if let Some((n, t)) = line.split_once('\t') {
                if let Ok(t) = t.parse::<u64>() {
                    m.insert(n.to_string(), t);
                }
            }
        }
    }
    m
}

fn save_usage(cfg: &Config, m: &HashMap<String, u64>) {
    let _ = std::fs::create_dir_all(meta_dir(cfg));
    let body: String = m.iter().map(|(n, t)| format!("{n}\t{t}\n")).collect();
    let _ = std::fs::write(usage_path(cfg), body);
}

fn sigs_path(cfg: &Config, name: &str) -> PathBuf {
    meta_dir(cfg).join(format!("{name}.sigs"))
}

fn load_sigs(cfg: &Config, name: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Ok(s) = std::fs::read_to_string(sigs_path(cfg, name)) {
        for line in s.lines() {
            if let Some((sig, src)) = line.split_once('\t') {
                m.insert(src.to_string(), sig.to_string());
            }
        }
    }
    m
}

fn save_sigs(cfg: &Config, name: &str, m: &HashMap<String, String>) {
    let _ = std::fs::create_dir_all(meta_dir(cfg));
    let body: String = m.iter().map(|(src, sig)| format!("{sig}\t{src}\n")).collect();
    let _ = std::fs::write(sigs_path(cfg, name), body);
}
