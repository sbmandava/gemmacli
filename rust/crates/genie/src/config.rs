//! Paths, env overrides, model resolution, and the litert-lm command builder.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pinned litert-lm runtime version (used only for the `uvx` fallback).
pub const LITERT_VERSION: &str = "0.13.1";

/// Static embedder used for RAG (must match the Python implementation).
pub const EMBED_MODEL: &str = "minishlab/potion-retrieval-32M";

pub struct Config {
    pub home: PathBuf,
    pub genie_dir: PathBuf,
    pub hf_home: PathBuf,
    pub backend_cache: PathBuf,
    pub model_default_file: PathBuf,
    /// GENIE_BACKEND override ("gpu"/"cpu"), if set.
    pub forced_backend: Option<String>,
    /// Resolved model variant: "e2b" or "e4b".
    pub model_variant: String,
    // --- RAG (M2/M3) ---
    pub cache_db: PathBuf,
    pub rag_threshold: usize,
    pub rag_topk: usize,
    pub chunk_size: usize,
    pub rag_ttl: u64,
    // --- graph (M4) ---
    pub graph_db: PathBuf,
}

impl Config {
    pub fn load(model_override: Option<&str>) -> Result<Config> {
        let home = PathBuf::from(std::env::var("HOME").context("HOME is not set")?);
        let hf_home = env_path("HF_HOME", home.join(".cache/huggingface"));
        let genie_dir = env_path("GENIE_DIR", home.join(".genie"));
        let backend_cache = env_path("GENIE_BACKEND_CACHE", genie_dir.join("backend"));
        let model_default_file =
            env_path("GENIE_MODEL_DEFAULT_FILE", genie_dir.join("model_default"));
        let forced_backend = nonempty_env("GENIE_BACKEND");
        let model_variant = resolve_model(model_override, &model_default_file);
        let cache_db = env_path("GENIE_CACHE_DB", genie_dir.join("genie-cache.db"));
        let rag_threshold = env_usize("GENIE_RAG_THRESHOLD", 14000);
        let rag_topk = env_usize("GENIE_RAG_TOPK", 15);
        let chunk_size = env_usize("GENIE_CHUNK_SIZE", 1000);
        let rag_ttl = env_usize("GENIE_CACHE_TTL", 86400) as u64;
        let graph_db = env_path("GENIE_GRAPH_DB", genie_dir.join("genie-graph.lbug"));
        Ok(Config {
            home,
            genie_dir,
            hf_home,
            backend_cache,
            model_default_file,
            forced_backend,
            model_variant,
            cache_db,
            rag_threshold,
            rag_topk,
            chunk_size,
            rag_ttl,
            graph_db,
        })
    }

    /// HF repo id + filename for the resolved model variant.
    pub fn model_repo_file(&self) -> (&'static str, &'static str) {
        match self.model_variant.as_str() {
            "e2b" => (
                "litert-community/gemma-4-E2B-it-litert-lm",
                "gemma-4-E2B-it.litertlm",
            ),
            _ => (
                "litert-community/gemma-4-E4B-it-litert-lm",
                "gemma-4-E4B-it.litertlm",
            ),
        }
    }

    /// Path to the model weights in the HF hub cache, if already downloaded.
    pub fn model_path(&self) -> Option<PathBuf> {
        let (repo, file) = self.model_repo_file();
        let snaps = self
            .hf_home
            .join("hub")
            .join(format!("models--{}", repo.replace('/', "--")))
            .join("snapshots");
        find_file(&snaps, file, 3)
    }

    /// Base argv for invoking litert-lm: prefer the `litert-lm` binary on PATH,
    /// else fall back to `uvx litert-lm@<ver> run`. Includes the model reference.
    pub fn litert_base_argv(&self) -> Vec<String> {
        let mut v = Vec::new();
        if which("litert-lm") {
            v.push("litert-lm".into());
            v.push("run".into());
        } else {
            v.push("uvx".into());
            v.push(format!("litert-lm@{LITERT_VERSION}"));
            v.push("run".into());
        }
        match self.model_path() {
            Some(p) => v.push(p.to_string_lossy().into_owned()),
            None => {
                // Not downloaded yet — let the runtime fetch it from HF.
                let (repo, file) = self.model_repo_file();
                v.push("--from-huggingface-repo".into());
                v.push(repo.into());
                v.push(file.into());
            }
        }
        v
    }
}

fn env_path(var: &str, default: PathBuf) -> PathBuf {
    match std::env::var(var) {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => default,
    }
}

fn nonempty_env(var: &str) -> Option<String> {
    match std::env::var(var) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

fn env_usize(var: &str, default: usize) -> usize {
    nonempty_env(var)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn resolve_model(model_override: Option<&str>, model_default_file: &Path) -> String {
    let norm = |s: &str| -> Option<String> {
        match s.trim().to_lowercase().as_str() {
            "e2b" => Some("e2b".into()),
            "e4b" => Some("e4b".into()),
            _ => None,
        }
    };
    if let Some(m) = model_override.and_then(norm) {
        return m;
    }
    if let Some(m) = nonempty_env("GENIE_MODEL").as_deref().and_then(norm) {
        return m;
    }
    if let Ok(s) = std::fs::read_to_string(model_default_file) {
        if let Some(m) = norm(&s) {
            return m;
        }
    }
    if detect_total_gb() >= 6 {
        "e4b".into()
    } else {
        "e2b".into()
    }
}

/// Total system RAM in whole GB (Linux /proc/meminfo, else macOS sysctl).
pub fn detect_total_gb() -> u64 {
    if let Ok(s) = std::fs::read_to_string("/proc/meminfo") {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                if let Some(kb) = rest.split_whitespace().next().and_then(|x| x.parse::<u64>().ok())
                {
                    return kb / 1024 / 1024;
                }
            }
        }
    }
    if let Ok(out) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return bytes / 1024 / 1024 / 1024;
            }
        }
    }
    0
}

/// Recursively look for `name` under `dir` up to `depth` levels deep.
fn find_file(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().map(|n| n == name).unwrap_or(false) {
            return Some(path);
        }
        if path.is_dir() {
            subdirs.push(path);
        }
    }
    if depth > 0 {
        for sub in subdirs {
            if let Some(found) = find_file(&sub, name, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Is `name` an executable found on PATH?
pub fn which(name: &str) -> bool {
    std::env::var("PATH")
        .map(|paths| paths.split(':').any(|d| Path::new(d).join(name).is_file()))
        .unwrap_or(false)
}
