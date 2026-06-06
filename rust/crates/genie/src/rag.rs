//! Retrieval: model2vec-rs embeddings + lancedb store/search, chunking, TTL
//! eviction, single-doc / dir / search-all modes, Sources footer. TODO M2/M3.
use crate::cli::Cli;
use anyhow::Result;

pub fn ask(_question: &str, _cli: &Cli) -> Result<()> {
    // Plain/piped --ask is handled by llm::ask (M1). This entry point is for
    // document-grounded asks (--doc/--txt/--dir), which need parse + retrieval.
    println!("genie (rust): document-grounded --ask (--doc/--txt/--dir) not yet implemented (RUST_PLAN.md M2/M3).");
    Ok(())
}

pub fn cache(action: &str) -> Result<()> {
    println!("genie (rust): cache {action} not yet implemented (RUST_PLAN.md M3).");
    Ok(())
}
