// Gemma Genie — Rust rewrite (single binary). CLI parse + dispatch.
// Modules live in the library (src/lib.rs) so tests/ can exercise them.
// See ../../RUST_PLAN.md for the plan and ../../CLAUDE.md for dependency wiring.

use anyhow::Result;
use clap::Parser;
use std::io::IsTerminal;

use genie::cli::{Cli, Command};
use genie::config::Config;
use genie::{doctor, graph, llm, models, rag};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load(cli.model.as_deref())?;

    // Subcommands first (doctor / cache).
    if let Some(cmd) = &cli.command {
        return match cmd {
            Command::Doctor => doctor::run(&cfg),
            Command::Cache { action } => rag::cache(action, &cfg),
        };
    }

    // Standalone flags.
    if cli.verify_models {
        return models::verify(&cfg);
    }
    if cli.uninstall {
        return models::uninstall(&cfg, cli.yes);
    }
    if cli.graph_stats {
        return graph::stats(&cfg);
    }
    if let Some(q) = &cli.graph_query {
        return graph::query(&cfg, q);
    }

    // Primary actions.
    if let Some(path) = &cli.image {
        return llm::describe_image(path, &cfg, &cli);
    }
    if let Some(path) = &cli.audio {
        return llm::transcribe_audio(path, &cfg, &cli);
    }
    if let Some(question) = &cli.ask {
        // Document-grounded asks need parsing + RAG (M2/M3).
        if cli.doc.is_some() || cli.txt.is_some() || cli.dir.is_some() {
            return rag::ask(question, &cli, &cfg);
        }
        // Piped input is treated as the document to analyze.
        if let Some(piped) = read_piped_stdin() {
            let prompt = format!("{question}\n\nAnalyze the following input:\n\n{piped}");
            return llm::generate(&cfg, prompt);
        }
        // No input: consult the indexed knowledge base (vectors) first, then
        // fall back to a plain model answer.
        if rag::ask_kb(question, &cfg)? {
            return Ok(());
        }
        return llm::generate(&cfg, question.to_string());
    }

    // No action: print help.
    Cli::parse_from(["genie", "--help"]);
    Ok(())
}

/// Return piped stdin content (None if stdin is a terminal or empty).
fn read_piped_stdin() -> Option<String> {
    use std::io::Read;
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return None;
    }
    let mut buf = String::new();
    if stdin.lock().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
        Some(buf)
    } else {
        None
    }
}
