//! Document extraction via liteparse (PDF/DOCX/XLSX/PPTX/images), with direct
//! reads for text formats and an early soffice guard for Office/ODF inputs.

use crate::config;
use anyhow::{bail, Result};
use liteparse::config::{LiteParseConfig, OutputFormat};
use liteparse::parser::LiteParse;
use std::path::Path;

const TEXT_EXTS: &[&str] = &[
    "txt", "text", "csv", "tsv", "md", "markdown", "log", "json", "yaml", "yml", "rst",
];
const OFFICE_EXTS: &[&str] = &["docx", "doc", "xlsx", "xls", "pptx", "ppt", "odt", "ods", "odp"];

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Extract text from a file. Text formats are read directly; everything else
/// (PDF/Office/images) goes through liteparse. Office formats need `soffice`.
pub async fn extract(path: &Path, pages: Option<&str>) -> Result<String> {
    let ext = ext_of(path);

    if TEXT_EXTS.contains(&ext.as_str()) {
        return Ok(std::fs::read_to_string(path)?);
    }

    if OFFICE_EXTS.contains(&ext.as_str()) && !config::which("soffice") {
        bail!(
            "parsing '{}' needs LibreOffice, but the 'soffice' command was not found.\n\
             Install the minimal LibreOffice components with:\n  {}",
            path.display(),
            libreoffice_install_cmd()
        );
    }

    let cfg = LiteParseConfig {
        output_format: OutputFormat::Text,
        target_pages: pages.map(|s| s.to_string()),
        quiet: true,
        ..Default::default()
    };
    let lp = LiteParse::new(cfg);
    let result = lp
        .parse(&path.to_string_lossy())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(result.text)
}

/// OS-aware minimal LibreOffice install hint.
fn libreoffice_install_cmd() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install --cask libreoffice"
    } else if config::which("apt-get") {
        "sudo apt-get install -y --no-install-recommends libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress"
    } else if config::which("dnf") {
        "sudo dnf install -y libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress"
    } else {
        "install LibreOffice (it provides the `soffice` command)"
    }
}
