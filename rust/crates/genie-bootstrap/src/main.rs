//! Gemma Genie — OS-agnostic modular installer (the bootstrapper).
//!
//! A tiny binary the install prelude downloads and runs. It probes the
//! environment (OS, arch, libc, GPU, RAM, existing deps), resolves the minimal
//! set of prebuilt COMPONENTS (platform binary, runtime libs, one model by RAM,
//! GPU backend only if a GPU is present) from a signed manifest, and fetches +
//! sha256-verifies only those. Default run is a dry-run plan; `--install` fetches.
//!
//! Note: this fetches prebuilt *components*, not Rust *crates* (crates are
//! compiled into the binary at build time). See specs/rust-installer.md.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

// ===========================================================================
// Environment probe
// ===========================================================================

#[derive(Debug)]
struct Env {
    os: &'static str,
    arch: &'static str,
    libc: Option<&'static str>,
    triple: String,
    gpu: Option<String>,
    ram_gb: u64,
    model: &'static str,
    soffice: bool,
    genie_dir: PathBuf,
    genie_dir_exists: bool,
}

fn probe() -> Env {
    let os = std::env::consts::OS; // "linux" | "macos" | "windows"
    let arch = std::env::consts::ARCH; // "x86_64" | "aarch64"
    let libc: Option<&'static str> = if os == "linux" {
        Some(if cfg!(target_env = "musl") { "musl" } else { "gnu" })
    } else {
        None
    };
    let triple = match libc {
        Some(l) => format!("{arch}-{os}-{l}"),
        None => format!("{arch}-{os}"),
    };
    let gpu = detect_gpu(os);
    let ram_gb = detect_ram_gb();
    let model = if ram_gb >= 6 { "e4b" } else { "e2b" };
    let home = home_dir();
    let genie_dir = home.join(".genie");
    let genie_dir_exists = genie_dir.exists();
    Env {
        os,
        arch,
        libc,
        triple,
        gpu,
        ram_gb,
        model,
        soffice: which("soffice").is_some(),
        genie_dir_exists,
        genie_dir,
    }
}

/// Best-effort GPU API detection (non-destructive). Real verification happens
/// later via `genie doctor` (with CPU fallback); this just picks the likely one.
fn detect_gpu(os: &str) -> Option<String> {
    match os {
        "macos" => Some("metal".into()),
        "linux" => {
            let nvidia = Path::new("/proc/driver/nvidia").exists()
                || Path::new("/dev/nvidia0").exists()
                || which("nvidia-smi").is_some();
            if nvidia {
                return Some("cuda".into());
            }
            let vulkan = Path::new("/usr/share/vulkan/icd.d").exists()
                || lib_exists("libvulkan.so.1")
                || which("vulkaninfo").is_some();
            if vulkan || Path::new("/dev/dri").exists() {
                return Some("vulkan".into());
            }
            None
        }
        "windows" => Some("directml".into()),
        _ => None,
    }
}

fn detect_ram_gb() -> u64 {
    if let Ok(s) = std::fs::read_to_string("/proc/meminfo") {
        for line in s.lines() {
            if let Some(r) = line.strip_prefix("MemTotal:") {
                if let Some(kb) = r.split_whitespace().next().and_then(|x| x.parse::<u64>().ok()) {
                    return kb / 1024 / 1024;
                }
            }
        }
    }
    if let Ok(out) = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(b) = s.trim().parse::<u64>() {
                return b / 1024 / 1024 / 1024;
            }
        }
    }
    0
}

fn lib_exists(name: &str) -> bool {
    ["/usr/lib", "/usr/lib/x86_64-linux-gnu", "/lib/x86_64-linux-gnu", "/usr/local/lib"]
        .iter()
        .any(|d| Path::new(d).join(name).exists())
}

fn which(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).map(|d| d.join(&exe)).find(|p| p.is_file())
    })
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

// ===========================================================================
// Manifest + plan
// ===========================================================================

#[derive(Deserialize)]
struct Manifest {
    version: String,
    components: Components,
}

/// All component sections are optional, so a manifest can host just the binary
/// (the rest — runtime/model/embedder — may come from elsewhere, e.g. pip/HF).
#[derive(Deserialize, Default)]
#[serde(default)]
struct Components {
    binary: BTreeMap<String, Artifact>,
    runtime: BTreeMap<String, Artifact>,
    gpu: BTreeMap<String, Artifact>,
    model: BTreeMap<String, Artifact>,
    embedder: Option<Artifact>,
}

#[derive(Deserialize, Clone)]
struct Artifact {
    url: String,
    sha256: String,
    #[serde(default)]
    size: u64,
}

struct PlanItem {
    name: String,
    artifact: Option<Artifact>, // None when no manifest given (abstract plan)
    note: String,
}

fn build_plan(env: &Env, manifest: Option<&Manifest>) -> Vec<PlanItem> {
    let mut plan = Vec::new();
    let mut add = |name: String, art: Option<Artifact>, note: String| {
        plan.push(PlanItem { name, artifact: art, note });
    };

    // 1. platform binary
    let bin = manifest.and_then(|m| m.components.binary.get(&env.triple).cloned());
    add(
        format!("genie binary ({})", env.triple),
        bin.clone(),
        if manifest.is_some() && bin.is_none() {
            "no prebuilt for this target — would fall back to a source build".into()
        } else {
            "the CLI".into()
        },
    );

    // 2. runtime lib (litert-lm)
    let rt = manifest.and_then(|m| m.components.runtime.get(&env.triple).cloned());
    add(format!("litert-lm runtime ({})", env.triple), rt, "inference engine".into());

    // 3. GPU backend — only if a GPU was detected
    if let Some(api) = &env.gpu {
        let key = format!("{}-{}", env.triple, api);
        let g = manifest.and_then(|m| m.components.gpu.get(&key).cloned());
        add(format!("GPU backend ({api})"), g, "only because a GPU was detected".into());
    } else {
        add("GPU backend".into(), None, "skipped — no GPU detected (CPU only)".into());
    }

    // 4. model — ONE variant by RAM
    let model = manifest.and_then(|m| m.components.model.get(env.model).cloned());
    add(
        format!("model: {}", env.model),
        model,
        format!("chosen by RAM ({} GB); the other variant is NOT fetched", env.ram_gb),
    );

    // 5. embedder — always
    add(
        "embedder (potion-retrieval-32M)".into(),
        manifest.and_then(|m| m.components.embedder.clone()),
        "small; always needed for RAG".into(),
    );

    // 6. LibreOffice — never bundled; just a hint
    if !env.soffice {
        add("LibreOffice (soffice)".into(), None, "missing — install hint only (DOCX/XLSX/PPTX)".into());
    }

    plan
}

// ===========================================================================
// Fetch + place
// ===========================================================================

fn load_manifest(src: &str) -> Result<Manifest> {
    let text = if src.starts_with("http://") || src.starts_with("https://") {
        ureq::get(src).call().context("fetch manifest")?.into_string()?
    } else {
        std::fs::read_to_string(src).with_context(|| format!("read manifest {src}"))?
    };
    Ok(serde_json::from_str(&text).context("parse manifest")?)
}

fn fetch_verify(art: &Artifact) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ureq::get(&art.url)
        .call()
        .with_context(|| format!("download {}", art.url))?
        .into_reader()
        .read_to_end(&mut buf)?;
    let got = hex(&Sha256::digest(&buf));
    if !art.sha256.is_empty() && got != art.sha256.to_lowercase() {
        bail!("sha256 mismatch for {}: expected {}, got {got}", art.url, art.sha256);
    }
    Ok(buf)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ===========================================================================
// main
// ===========================================================================

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let install = args.iter().any(|a| a == "--install");
    let manifest_arg = arg_value(&args, "--manifest");
    let bin_dir = arg_value(&args, "--bin-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/bin"));

    let env = probe();
    print_env(&env);

    let manifest = match &manifest_arg {
        Some(m) => Some(load_manifest(m)?),
        None => None,
    };
    if let Some(m) = &manifest {
        println!("\nManifest version: {}", m.version);
    }

    let plan = build_plan(&env, manifest.as_ref());
    print_plan(&plan);

    if !install {
        println!("\n(dry run) Pass --install with --manifest <url|file> to download.");
        return Ok(());
    }
    if manifest.is_none() {
        bail!("--install requires --manifest <url|file>");
    }

    // Fetch the components that have a resolved artifact.
    let components_dir = env.genie_dir.join("components");
    std::fs::create_dir_all(&components_dir)?;
    std::fs::create_dir_all(&bin_dir)?;
    for item in &plan {
        let Some(art) = &item.artifact else { continue };
        println!("\nFetching {} ...", item.name);
        let bytes = fetch_verify(art)?;
        let fname = art.url.rsplit('/').next().unwrap_or("component.bin");
        let dest = if item.name.starts_with("genie binary") {
            bin_dir.join(if cfg!(windows) { "genie.exe" } else { "genie" })
        } else {
            components_dir.join(fname)
        };
        std::fs::write(&dest, &bytes)?;
        make_executable_if_binary(&item.name, &dest);
        println!("  -> {} ({} bytes)", dest.display(), bytes.len());
    }
    println!("\nInstalled. Run `genie doctor` to verify the backend.");
    Ok(())
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == key {
            return it.next().cloned();
        }
        if let Some(v) = a.strip_prefix(&format!("{key}=")) {
            return Some(v.to_string());
        }
    }
    None
}

#[cfg(unix)]
fn make_executable_if_binary(name: &str, path: &Path) {
    if name.starts_with("genie binary") {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
    }
}
#[cfg(not(unix))]
fn make_executable_if_binary(_name: &str, _path: &Path) {}

fn print_env(e: &Env) {
    println!("Gemma Genie installer — detected environment:");
    println!("  os/arch : {}/{}{}", e.os, e.arch, e.libc.map(|l| format!(" ({l})")).unwrap_or_default());
    println!("  target  : {}", e.triple);
    println!("  gpu     : {}", e.gpu.clone().unwrap_or_else(|| "none (CPU)".into()));
    println!("  ram     : {} GB  -> model {}", e.ram_gb, e.model);
    println!("  soffice : {}", if e.soffice { "present" } else { "missing" });
    println!("  ~/.genie: {}", if e.genie_dir_exists { "exists" } else { "new" });
}

fn print_plan(plan: &[PlanItem]) {
    println!("\nComponent plan (only what this environment needs):");
    for it in plan {
        let mark = match &it.artifact {
            Some(a) => format!("fetch ({} bytes)", a.size),
            None => "skip".into(),
        };
        println!("  [{mark}] {} — {}", it.name, it.note);
    }
}
