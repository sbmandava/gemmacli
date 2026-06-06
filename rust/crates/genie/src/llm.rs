//! litert-lm invocation. v1 subprocesses the prebuilt `litert-lm` binary with
//! noise filtering, line-streamed output, and runtime CPU fallback. FFI is M6.

use crate::backend;
use crate::cli::Cli;
use crate::config::Config;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

/// Harmless GPU/OpenCL backend chatter to drop from the runtime's output.
const NOISE: [&str; 3] = [
    "maxDynamicUniformBuffersPerPipelineLayout",
    "maxDynamicStorageBuffersPerPipelineLayout",
    "Loaded OpenCL library with dlopen",
];

pub fn is_noise(line: &str) -> bool {
    NOISE.iter().any(|p| line.contains(p))
}

/// litert prints this (and can still exit 0) when a generation fails.
pub fn is_error_marker(line: &str) -> bool {
    line.trim_start().starts_with("An error occurred")
}

/// A single model action and its litert-lm flags.
enum Action {
    Ask(String),
    Image(String),
    Audio(String),
}

/// Run the model on an already-built prompt (plain ask or RAG-built prompt).
pub fn generate(cfg: &Config, prompt: String) -> Result<()> {
    #[cfg(feature = "ffi")]
    {
        let msg = serde_json::json!({ "role": "user", "content": prompt }).to_string();
        if try_ffi(cfg, &msg) {
            return Ok(());
        }
    }
    run(cfg, Action::Ask(prompt))
}

// NOTE: vision (--image) and audio (--audio) stay on the subprocess path. The
// in-process FFI multimodal message path returns no output (the engine accepts
// the image/audio message but produces nothing — protocol detail still TBD), so
// routing them through FFI would just fail and fall back, adding a wasted engine
// init. Text generation is the part that works in-process.
pub fn describe_image(path: &Path, cfg: &Config, _cli: &Cli) -> Result<()> {
    run(cfg, Action::Image(path.to_string_lossy().into_owned()))
}

pub fn transcribe_audio(path: &Path, cfg: &Config, _cli: &Cli) -> Result<()> {
    run(cfg, Action::Audio(path.to_string_lossy().into_owned()))
}

/// Attempt the in-process FFI text path; returns true if it handled the request.
/// Falls back (returns false) on any error so the caller uses the subprocess.
#[cfg(feature = "ffi")]
fn try_ffi(cfg: &Config, message_json: &str) -> bool {
    let Some(mp) = cfg.model_path() else {
        return false;
    };
    let backend = backend::resolve(cfg);
    match crate::ffi::run(
        &mp.to_string_lossy(),
        &backend,
        None,
        None,
        message_json,
    ) {
        Ok(text) => {
            println!("{}", text.trim_end());
            true
        }
        Err(e) => {
            eprintln!("genie: in-process FFI failed ({e}); using subprocess");
            false
        }
    }
}

/// Run the model with automatic CPU fallback: if a GPU run yields no real
/// answer, retry once on CPU (a successful GPU run is never re-run).
fn run(cfg: &Config, action: Action) -> Result<()> {
    let backend = backend::resolve(cfg);
    let produced = run_once(&build_argv(cfg, &backend, &action))?;
    if !produced && backend == "gpu" {
        eprintln!("genie: GPU backend produced no answer — retrying on CPU...");
        if cfg.forced_backend.is_none() {
            let _ = std::fs::write(&cfg.backend_cache, "cpu");
        }
        run_once(&build_argv(cfg, "cpu", &action))?;
    }
    Ok(())
}

fn build_argv(cfg: &Config, backend: &str, action: &Action) -> Vec<String> {
    let mut v = cfg.litert_base_argv();
    match action {
        Action::Ask(prompt) => {
            v.push(format!("--backend={backend}"));
            v.push("--prompt".into());
            v.push(prompt.clone());
        }
        Action::Image(path) => {
            v.push(format!("--backend={backend}"));
            v.push(format!("--vision-backend={backend}"));
            v.push("--attachment".into());
            v.push(path.clone());
            v.push("--prompt".into());
            v.push("describe".into());
        }
        Action::Audio(path) => {
            v.push(format!("--backend={backend}"));
            v.push("--audio-backend=cpu".into());
            v.push("--attachment".into());
            v.push(path.clone());
            v.push("--prompt".into());
            v.push("transcribe".into());
        }
    }
    v
}

/// Run one invocation: stream filtered output live, return true if a real answer
/// was produced (non-empty, non-error, non-noise stdout).
fn run_once(argv: &[String]) -> Result<bool> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to launch `{}`", argv[0]))?;

    // Filter noise from stderr on a separate thread.
    let stderr = child.stderr.take().expect("piped stderr");
    let err_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if !is_noise(&line) {
                eprintln!("{line}");
            }
        }
    });

    // Stream stdout (filtered), tracking whether a real answer appeared.
    let stdout = child.stdout.take().expect("piped stdout");
    let reader = BufReader::new(stdout);
    let out = std::io::stdout();
    let mut lock = out.lock();
    let mut produced = false;
    for line in reader.lines().map_while(Result::ok) {
        if is_noise(&line) {
            continue;
        }
        if !line.trim().is_empty() && !is_error_marker(&line) {
            produced = true;
        }
        let _ = writeln!(lock, "{line}");
        let _ = lock.flush();
    }

    let _ = err_thread.join();
    let _ = child.wait();
    Ok(produced)
}
