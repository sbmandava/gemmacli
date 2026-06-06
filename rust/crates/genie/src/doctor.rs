//! Dependency check + live backend/model report (ports `genie doctor`).

use crate::backend;
use crate::config::{self, Config};
use anyhow::Result;
use std::fs;

pub fn run(cfg: &Config) -> Result<()> {
    println!("Gemma Genie {} (rust)", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Dependency check:");

    // litert-lm runtime (subprocessed).
    if config::which("litert-lm") {
        line("[ok]", "litert-lm runtime (on PATH)");
    } else if config::which("uvx") {
        line("[ok]", "litert-lm via uvx (binary not on PATH)");
    } else {
        line("[MISSING]", "litert-lm runtime — install litert-lm or uv (uvx)");
    }

    // LibreOffice (for Office formats via liteparse; relevant from M2 on).
    if config::which("soffice") {
        line("[ok]", "LibreOffice (soffice) — enables DOCX/XLSX/PPTX");
    } else {
        line(
            "[warn]",
            "LibreOffice (soffice) not found — DOCX/XLSX/PPTX need it (PDF/images are fine)",
        );
    }

    // Model weights present?
    match cfg.model_path() {
        Some(p) => line("[ok]", &format!("model weights: {}", p.display())),
        None => line(
            "[warn]",
            &format!(
                "model weights for {} not downloaded yet (will fetch on first run)",
                cfg.model_variant
            ),
        ),
    }

    // Live compute backend — actually verify a cached/forced GPU.
    let (be, src) = report_backend(cfg);
    line("[info]", &format!("compute backend: {} ({src})", be.to_uppercase()));
    line(
        "[info]",
        &format!(
            "default model: {} (system RAM: {}GB; override with --model / GENIE_MODEL)",
            cfg.model_variant,
            config::detect_total_gb()
        ),
    );

    println!();
    println!("Caches: {} (genie data), {} (HF models)", cfg.genie_dir.display(), cfg.hf_home.display());
    Ok(())
}

/// Determine and verify the backend genie would actually use.
fn report_backend(cfg: &Config) -> (String, String) {
    let (mut raw, mut src, forced) = if let Some(b) = &cfg.forced_backend {
        (b.clone(), "forced via GENIE_BACKEND".to_string(), true)
    } else if let Ok(s) = fs::read_to_string(&cfg.backend_cache) {
        (s.trim().to_string(), "cached".to_string(), false)
    } else {
        (String::new(), String::new(), false)
    };

    let can_probe = cfg.model_path().is_some()
        && (config::which("litert-lm") || config::which("uvx"));

    if raw.is_empty() {
        if can_probe {
            raw = backend::resolve(cfg);
            src = "auto-detected just now".to_string();
        } else {
            raw = "unknown".to_string();
            src = "will auto-detect on first query".to_string();
        }
    } else if raw == "gpu" && can_probe {
        if backend::verify_gpu(cfg) {
            src = format!("{src}, verified working with litert-lm");
        } else if forced {
            src = "forced via GENIE_BACKEND — WARNING: GPU did not run with litert-lm".to_string();
        } else {
            let _ = fs::write(&cfg.backend_cache, "cpu");
            raw = "cpu".to_string();
            src = "GPU did not run with litert-lm — fell back to CPU".to_string();
        }
    } else if raw == "gpu" {
        src = format!("{src} (model not downloaded yet — not verified)");
    }
    (raw, src)
}

fn line(tag: &str, msg: &str) {
    println!("  {tag:<9} {msg}");
}
