// Gemma Genie — Rust rewrite (single binary). SCAFFOLD ONLY.
//
// This parses the full CLI surface of the bash `genie` and dispatches to module
// stubs. No behaviour yet — each path prints which milestone implements it.
// See ../../RUST_PLAN.md for the plan and ../../CLAUDE.md for dependency wiring.
#![allow(dead_code)]

mod backend;
mod cli;
mod config;
mod doctor;
mod graph;
mod llm;
mod models;
mod parse;
mod rag;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load(cli.model.as_deref())?;

    // Subcommands first (doctor / cache).
    if let Some(cmd) = &cli.command {
        return match cmd {
            Command::Doctor => doctor::run(&cfg),
            Command::Cache { action } => rag::cache(action),
        };
    }

    // Standalone flags.
    if cli.verify_models {
        return models::verify();
    }
    if cli.uninstall {
        return models::uninstall();
    }
    if cli.graph_stats {
        return graph::stats();
    }
    if let Some(q) = &cli.graph_query {
        return graph::query(q);
    }

    // Primary actions.
    if let Some(path) = &cli.image {
        return llm::describe_image(path, &cfg, &cli);
    }
    if let Some(path) = &cli.audio {
        return llm::transcribe_audio(path, &cfg, &cli);
    }
    if let Some(question) = &cli.ask {
        // Document-grounded asks need parsing + RAG (M2/M3); plain or piped
        // questions are answered directly by the model (M1).
        if cli.doc.is_some() || cli.txt.is_some() || cli.dir.is_some() {
            return rag::ask(question, &cli);
        }
        return llm::ask(question, &cfg, &cli);
    }

    // No action: print help.
    Cli::parse_from(["genie", "--help"]);
    Ok(())
}
