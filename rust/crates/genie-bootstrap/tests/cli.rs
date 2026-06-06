//! Tests for the bootstrapper CLI (no network: dry-run + manifest-resolve).
use std::process::Command;

fn boot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_genie-bootstrap"))
}

#[test]
fn dry_run_reports_env_and_plan() {
    let out = boot().output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("target  :"), "missing target line:\n{s}");
    assert!(s.contains("Component plan"), "missing plan:\n{s}");
    // Exactly one model variant is ever planned (not both).
    assert_eq!(s.matches("model:").count(), 1, "expected exactly one model line:\n{s}");
}

#[test]
fn manifest_resolves_minimal_set() {
    // A manifest covering this machine's target so the binary/runtime resolve.
    let triple_line = {
        let out = boot().output().unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find(|l| l.trim_start().starts_with("target  :"))
            .unwrap()
            .to_string()
    };
    let triple = triple_line.split(':').nth(1).unwrap().trim().to_string();

    let manifest = format!(
        r#"{{"version":"test","components":{{
          "binary":{{"{t}":{{"url":"https://x/genie","sha256":"","size":1}}}},
          "runtime":{{"{t}":{{"url":"https://x/rt","sha256":"","size":2}}}},
          "model":{{"e2b":{{"url":"https://x/e2b","sha256":"","size":3}},
                    "e4b":{{"url":"https://x/e4b","sha256":"","size":4}}}},
          "embedder":{{"url":"https://x/emb","sha256":"","size":5}}
        }}}}"#,
        t = triple
    );
    let mf = std::env::temp_dir().join("genie-bootstrap-test-manifest.json");
    std::fs::write(&mf, manifest).unwrap();

    let out = boot().args(["--manifest", mf.to_str().unwrap()]).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("Manifest version: test"));
    // The binary + runtime for this target resolve to a fetch.
    assert!(s.contains("genie binary"));
    assert!(s.matches("fetch (").count() >= 2, "expected resolved fetches:\n{s}");
    // Still only one model.
    assert_eq!(s.matches("model:").count(), 1);
    let _ = std::fs::remove_file(&mf);
}
