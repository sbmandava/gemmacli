//! End-to-end tests that run the `genie` binary. The model-backed tests are
//! `#[ignore]`d (they need the Gemma weights + GPU/CPU and are slow); run them
//! with `cargo test -- --ignored`. They use the sample corpus under
//! /opt/projects/unovie/dataingest/sample.
use std::path::Path;
use std::process::Command;

const SAMPLE: &str = "/opt/projects/unovie/dataingest/sample";

fn genie() -> Command {
    Command::new(env!("CARGO_BIN_EXE_genie"))
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

/// A scratch path under the OS temp dir (never inside the repo).
fn scratch(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("genie-test-{name}"))
        .to_string_lossy()
        .into_owned()
}

/// Remove a path whether it's a file or a directory (lbug DBs are files).
fn rm(path: &str) {
    let _ = std::fs::remove_dir_all(path);
    let _ = std::fs::remove_file(path);
}

// ---- fast (no model) ----

#[test]
fn version_prints() {
    let out = genie().arg("--version").output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("0.2"), "version output: {s}");
}

#[test]
fn help_lists_flags_and_subcommands() {
    let out = genie().arg("--help").output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("--ask"), "help missing --ask: {s}");
    assert!(s.contains("doctor"), "help missing doctor: {s}");
}

// ---- model-backed (slow; `cargo test -- --ignored`) ----

#[test]
#[ignore = "runs the model (weights + GPU/CPU); slow"]
fn doctor_reports_backend() {
    let out = genie().arg("doctor").output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("compute backend"), "doctor output: {s}");
}

#[test]
#[ignore = "runs the model; slow"]
fn txt_inline_answer_uses_source() {
    let f = fixture("notes.txt");
    let out = genie()
        .args(["--ask", "Who owns Apollo? Reply with just the name.", "--txt", &f])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.to_lowercase().contains("jane"), "expected 'Jane', got: {s}");
    assert!(s.contains("Sources:"), "missing Sources footer: {s}");
}

#[test]
#[ignore = "runs the full RAG path over a large PDF; slow"]
fn pdf_rag_answer_with_sources_no_noise() {
    let pdf = format!("{SAMPLE}/navrules.pdf");
    if !Path::new(&pdf).exists() {
        eprintln!("skipping: sample {pdf} not present");
        return;
    }
    let out = genie()
        .args(["--ask", "What do these rules govern? One sentence.", "--doc", &pdf])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.trim().is_empty(), "empty answer");
    assert!(s.contains("Sources:"), "missing Sources footer");
    assert!(!s.contains("Loaded OpenCL"), "GPU noise leaked to stdout");
}

#[test]
#[ignore = "runs liteparse+soffice over a PPTX then the model; slow"]
fn pptx_answer() {
    let pptx = format!("{SAMPLE}/unovie-country.pptx");
    if !Path::new(&pptx).exists() {
        eprintln!("skipping: sample {pptx} not present");
        return;
    }
    let out = genie()
        .args(["--ask", "Summarize this deck in one sentence.", "--doc", &pptx])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.trim().is_empty(), "empty answer");
}

#[test]
#[ignore = "indexes a dir + runs the model; slow"]
fn dir_kb_incremental_and_cache() {
    // Isolated cache under the OS temp dir so we don't touch ~/.genie or the repo.
    let cache = scratch("m3-cache");
    let dir = scratch("m3-dir");
    rm(&cache);
    rm(&format!("{cache}.meta"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/apollo.txt"), "Project Apollo is owned by Jane Smith.\n").unwrap();
    std::fs::write(format!("{dir}/zeus.md"), "Project Zeus is owned by Bob Jones.\n").unwrap();

    // Index + ask over the directory.
    let out = genie()
        .env("GENIE_CACHE_DB", &cache)
        .args(["--ask", "Who owns Apollo? Name only.", "--dir", &dir])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.to_lowercase().contains("jane"), "expected Jane, got: {s}");

    // cache list shows exactly one (dir) table.
    let out = genie().env("GENIE_CACHE_DB", &cache).args(["cache", "list"]).output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("Tables: 1"), "cache list: {s}");

    // Re-index is incremental (still 1 table, 2 chunks).
    let _ = genie()
        .env("GENIE_CACHE_DB", &cache)
        .args(["--ask", "x", "--dir", &dir])
        .output()
        .unwrap();
    let out = genie().env("GENIE_CACHE_DB", &cache).args(["cache", "list"]).output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("2 chunks"), "expected 2 chunks after re-index, got: {s}");

    // clear empties it.
    let _ = genie().env("GENIE_CACHE_DB", &cache).args(["cache", "clear"]).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "indexes a dir + builds the graph + runs the model; slow"]
fn graph_build_stats_and_correlate() {
    let cache = scratch("m4-cache");
    let graph = scratch("m4-graph");
    let dir = scratch("m4-dir");
    for p in [&cache, &format!("{cache}.meta"), &graph, &dir] {
        rm(p);
    }
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/apollo.txt"), "Project Apollo is owned by Jane Smith and built by Acme Corporation.\n").unwrap();
    std::fs::write(format!("{dir}/zeus.txt"), "Project Zeus is owned by Bob Jones. Acme Corporation is the auditor.\n").unwrap();

    let env = |c: &mut Command| {
        c.env("GENIE_CACHE_DB", &cache).env("GENIE_GRAPH_DB", &graph);
    };

    // Index (also populates the graph).
    let mut c = genie();
    env(&mut c);
    let out = c.args(["--ask", "list", "--dir", &dir]).output().unwrap();
    assert!(out.status.success());

    // graph-stats reports files/entities and Acme as a top hub.
    let mut c = genie();
    env(&mut c);
    let out = c.arg("--graph-stats").output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("Files:"), "stats: {s}");
    assert!(s.contains("Acme Corporation"), "expected Acme entity: {s}");

    // graph-query returns Mentions rows.
    let mut c = genie();
    env(&mut c);
    let out = c
        .args(["--graph-query", "MATCH (f:File)-[:Mentions]->(e:Entity) RETURN f.name, e.name LIMIT 10;"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Acme Corporation"));

    for p in [&cache, &format!("{cache}.meta"), &graph, &dir] {
        rm(p);
    }
}

// ---- M5 lifecycle (fast: no model) ----

#[test]
fn uninstall_guard_refuses_without_yes() {
    use std::process::Stdio;
    let gdir = scratch("u-guard-genie");
    let hf = scratch("u-guard-hf");
    rm(&gdir);
    rm(&hf);
    std::fs::create_dir_all(&gdir).unwrap();
    let out = genie()
        .env("GENIE_DIR", &gdir)
        .env("HF_HOME", &hf)
        .arg("--uninstall")
        .stdin(Stdio::null()) // non-interactive
        .output()
        .unwrap();
    assert!(!out.status.success(), "uninstall must refuse without --yes when non-interactive");
    assert!(Path::new(&gdir).exists(), "must not delete anything when refused");
    rm(&gdir);
}

#[test]
fn uninstall_yes_removes_isolated_data() {
    let gdir = scratch("u-yes-genie");
    let hf = scratch("u-yes-hf");
    rm(&gdir);
    rm(&hf);
    std::fs::create_dir_all(format!("{gdir}/genie-cache.db")).unwrap();
    std::fs::write(format!("{gdir}/backend"), "gpu").unwrap();
    let out = genie()
        .env("GENIE_DIR", &gdir)
        .env("HF_HOME", &hf)
        .args(["--uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(!Path::new(&gdir).exists(), "genie dir should be removed");
}

#[test]
#[ignore = "hits the HF hub (network) to verify model integrity; slow"]
fn verify_models_ok() {
    let out = genie().arg("--verify-models").output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains("verified"));
}

// ---- M6 in-process FFI (only compiled with --features ffi) ----

#[cfg(feature = "ffi")]
#[test]
#[ignore = "links liblitert-lm.so + runs the model in-process; slow"]
fn ffi_in_process_generate() {
    let out = genie()
        .env("GENIE_CACHE_DB", scratch("ffi-none"))
        .env("GENIE_GRAPH_DB", scratch("ffi-none2"))
        .args(["--ask", "why is the sky blue, in one sentence"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(s.contains("scatter") || s.contains("blue"), "unexpected answer: {s}");
    // Must NOT have fallen back to the subprocess.
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("using subprocess"), "FFI fell back to subprocess: {err}");
}
