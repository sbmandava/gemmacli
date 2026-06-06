//! CLI surface — mirrors the bash `genie`'s flags and subcommands.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "genie",
    version,
    about = "Private, offline AI assistant — Gemma 4 + local RAG + correlation graph",
    long_about = None,
)]
pub struct Cli {
    /// Ask a question (optionally grounded by --doc/--txt/--dir or piped stdin).
    #[arg(long)]
    pub ask: Option<String>,

    /// Analyze a document (PDF/DOCX/XLSX/PPTX/image) via liteparse.
    #[arg(long)]
    pub doc: Option<PathBuf>,

    /// Analyze a plain-text file.
    #[arg(long)]
    pub txt: Option<PathBuf>,

    /// Index/query a directory as a knowledge base.
    #[arg(long)]
    pub dir: Option<PathBuf>,

    /// Describe an image.
    #[arg(long)]
    pub image: Option<PathBuf>,

    /// Transcribe an audio file.
    #[arg(long)]
    pub audio: Option<PathBuf>,

    /// Page range for --doc (e.g. "1-5").
    #[arg(long)]
    pub pages: Option<String>,

    /// Number of chunks to retrieve.
    #[arg(long = "top-k")]
    pub top_k: Option<usize>,

    /// Characters per chunk when embedding.
    #[arg(long = "chunk-size")]
    pub chunk_size: Option<usize>,

    /// Model variant: e2b or e4b.
    #[arg(long)]
    pub model: Option<String>,

    /// Print correlation-graph stats and exit.
    #[arg(long = "graph-stats")]
    pub graph_stats: bool,

    /// Run a raw Cypher query against the correlation graph.
    #[arg(long = "graph-query")]
    pub graph_query: Option<String>,

    /// Verify model weights integrity (sha256) and exit.
    #[arg(long = "verify-models")]
    pub verify_models: bool,

    /// Remove models, caches, and scripts.
    #[arg(long)]
    pub uninstall: bool,

    /// Assume "yes" to confirmations (e.g. --uninstall).
    #[arg(short = 'y', long)]
    pub yes: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Dependency check + live backend/model report.
    Doctor,
    /// Inspect or clear the LanceDB vector cache.
    Cache {
        /// info | list | clear
        #[arg(default_value = "info")]
        action: String,
    },
}
