//! Compute-backend selection: resolve (env -> cache -> probe) and verify the GPU
//! with a real litert-lm generation. Ports the bash 0.2.4 logic.

use crate::config::Config;
use crate::llm;
use std::fs;
use std::process::{Command, Stdio};

/// Resolve the backend to use: GENIE_BACKEND -> cached choice -> one-time GPU
/// probe (fall back to CPU), caching the probe result.
pub fn resolve(cfg: &Config) -> String {
    if let Some(b) = &cfg.forced_backend {
        return b.clone();
    }
    if let Ok(s) = fs::read_to_string(&cfg.backend_cache) {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    eprintln!("Detecting compute backend (one-time)...");
    let b = if verify_gpu(cfg) { "gpu" } else { "cpu" };
    if let Some(parent) = cfg.backend_cache.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&cfg.backend_cache, b);
    eprintln!(
        "Using {b} backend (cached in {}).",
        cfg.backend_cache.display()
    );
    b.to_string()
}

/// Run a tiny generation on the GPU and return true only if it genuinely works:
/// the process exits cleanly AND produces non-empty output that isn't litert's
/// "An error occurred" marker (the CLI can exit 0 on a failed generation, and a
/// too-small token budget rejects the prompt — neither is a GPU failure, so we
/// use 64 tokens and check the output, not just the exit code).
pub fn verify_gpu(cfg: &Config) -> bool {
    let mut argv = cfg.litert_base_argv();
    argv.push("--backend=gpu".into());
    argv.push("--max-num-tokens".into());
    argv.push("64".into());
    argv.push("--prompt".into());
    argv.push("ok".into());

    let out = match Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body: String = stdout
        .lines()
        .filter(|l| !llm::is_noise(l) && !llm::is_error_marker(l))
        .collect::<Vec<_>>()
        .join("");
    !body.trim().is_empty()
}
