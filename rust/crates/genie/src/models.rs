//! Model lifecycle: download/verify weights via hf-hub, and uninstall.

use crate::config::{Config, EMBED_MODEL};
use anyhow::{anyhow, Result};
use hf_hub::api::sync::ApiBuilder;
use std::io::IsTerminal;
use std::path::PathBuf;

const GEMMA: &[(&str, &str)] = &[
    ("litert-community/gemma-4-E2B-it-litert-lm", "gemma-4-E2B-it.litertlm"),
    ("litert-community/gemma-4-E4B-it-litert-lm", "gemma-4-E4B-it.litertlm"),
];

/// `--verify-models`: ensure each Gemma model is downloaded and integrity-checked
/// (hf-hub verifies the file against the hub's metadata, re-fetching if needed).
pub fn verify(cfg: &Config) -> Result<()> {
    let api = ApiBuilder::new()
        .with_cache_dir(cfg.hf_home.join("hub"))
        .build()
        .map_err(|e| anyhow!("hf-hub init: {e}"))?;

    let mut failed = 0;
    for (repo, file) in GEMMA {
        print!("Verifying {repo} ({file})... ");
        match api.model(repo.to_string()).get(file) {
            Ok(p) => println!("ok\n  {}", p.display()),
            Err(e) => {
                println!("FAILED: {e}");
                failed += 1;
            }
        }
    }
    if failed == 0 {
        println!("All models verified.");
        Ok(())
    } else {
        Err(anyhow!("{failed} model(s) failed verification"))
    }
}

/// `--uninstall`: remove model weights, the embedder, and all genie caches.
pub fn uninstall(cfg: &Config, yes: bool) -> Result<()> {
    let mut targets: Vec<PathBuf> = Vec::new();
    let hub = cfg.hf_home.join("hub");
    for (repo, _) in GEMMA {
        targets.push(hub.join(format!("models--{}", repo.replace('/', "--"))));
    }
    targets.push(hub.join(format!("models--{}", EMBED_MODEL.replace('/', "--"))));
    targets.push(cfg.cache_db.clone());
    targets.push(cfg.cache_db.with_extension("meta"));
    targets.push(cfg.graph_db.clone());
    targets.push(cfg.genie_dir.clone());

    let existing: Vec<&PathBuf> = targets.iter().filter(|p| p.exists()).collect();
    if existing.is_empty() {
        println!("Nothing to remove.");
        return Ok(());
    }

    println!("This will remove:");
    for p in &existing {
        println!("  - {}", p.display());
    }
    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err(anyhow!("re-run with --yes to confirm (non-interactive)"));
        }
        print!("Proceed? [y/N] ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        if !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    for p in &existing {
        let r = if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        match r {
            Ok(_) => println!("removed {}", p.display()),
            Err(e) => eprintln!("warning: could not remove {}: {e}", p.display()),
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        println!("\nThe genie binary itself was left in place: {}", exe.display());
        println!("Delete it manually to fully uninstall.");
    }
    Ok(())
}
