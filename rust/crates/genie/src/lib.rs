//! Gemma Genie (Rust) — library surface, so integration tests in `tests/` can
//! exercise pure logic. The `genie` binary (src/main.rs) uses these modules too.
#![allow(dead_code)]

pub mod backend;
pub mod cli;
pub mod config;
pub mod doctor;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod graph;
pub mod llm;
pub mod models;
pub mod parse;
pub mod rag;
