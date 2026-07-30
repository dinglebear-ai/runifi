//! Repository maintenance and API contract verification tasks.

mod endpoint_probe;
mod forbidden_strings;
mod internal_reference;
mod official_api;
mod verify_endpoints;
mod verify_policy;

use anyhow::{Context, Result, bail};
use std::process::Command;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("refresh-official-api") => official_api::refresh(),
        Some("validate-internal-reference") => internal_reference::refresh(),
        Some("refresh-internal-reference") => internal_reference::refresh(),
        Some("verify-api-endpoints") => verify_endpoints::verify(),
        Some("check-forbidden-strings") => forbidden_strings::check(),
        Some("dist") => dist(),
        Some("ci") => ci(),
        Some("symlink-docs") => symlink_docs(),
        Some("check-env") => check_env(),
        Some(cmd) => bail!(
            "Unknown xtask command: {cmd}\n\nAvailable: refresh-official-api, validate-internal-reference, refresh-internal-reference, verify-api-endpoints, check-forbidden-strings, dist, ci, symlink-docs, check-env"
        ),
        None => bail!(
            "Usage: cargo run -p xtask -- <command>\n\nAvailable: refresh-official-api, validate-internal-reference, refresh-internal-reference, verify-api-endpoints, check-forbidden-strings, dist, ci, symlink-docs, check-env"
        ),
    }
}

fn dist() -> Result<()> {
    println!("xtask dist: building release binary...");
    run("cargo", &["build", "--release", "--locked"])?;

    std::fs::create_dir_all("bin").context("create bin/")?;
    let src = "target/release/runifi";
    let dst = "bin/runifi";
    std::fs::copy(src, dst).with_context(|| format!("copy {src} to {dst}"))?;
    println!("xtask dist: copied {src} to {dst}");

    if Command::new("git")
        .args(["lfs", "version"])
        .output()
        .is_ok()
    {
        println!("xtask dist: tracking bin/ via Git LFS...");
        run("git", &["lfs", "track", "bin/*"])?;
        run("git", &["add", ".gitattributes", dst])?;
        println!("xtask dist: staged {dst} for LFS commit");
    } else {
        eprintln!("WARNING: git-lfs not found; bin/runifi will not be LFS-tracked");
    }
    Ok(())
}

fn ci() -> Result<()> {
    println!("xtask ci: cargo fmt --check");
    run("cargo", &["fmt", "--", "--check"])?;

    println!("xtask ci: cargo clippy");
    run("cargo", &["clippy", "--", "-D", "warnings"])?;

    println!("xtask ci: cargo nextest run --profile ci");
    run("cargo", &["nextest", "run", "--profile", "ci"])?;

    println!("xtask ci: taplo check");
    run("taplo", &["check"])?;

    println!("xtask ci: cargo audit");
    run("cargo", &["audit"])?;

    println!("xtask ci: all checks passed");
    Ok(())
}

fn symlink_docs() -> Result<()> {
    let root = std::env::current_dir().context("get cwd")?;

    for entry in walkdir(&root)? {
        let path = entry?;
        if path.file_name() != Some(std::ffi::OsStr::new("CLAUDE.md")) {
            continue;
        }
        let dir = path.parent().unwrap_or(&root);
        for link_name in ["AGENTS.md", "GEMINI.md"] {
            let link_path = dir.join(link_name);
            if link_path.exists() || link_path.symlink_metadata().is_ok() {
                std::fs::remove_file(&link_path).ok();
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink("CLAUDE.md", &link_path)
                .with_context(|| format!("symlink CLAUDE.md to {}", link_path.display()))?;
            #[cfg(not(unix))]
            eprintln!(
                "WARNING: symlinks not supported on this platform; skipping {}",
                link_path.display()
            );
            println!("symlink-docs: {} -> CLAUDE.md", link_path.display());
        }
    }
    Ok(())
}

fn check_env() -> Result<()> {
    let required = [
        (
            "UNIFI_URL",
            "UniFi controller base URL, e.g. https://unifi.local",
        ),
        (
            "UNIFI_API_KEY",
            "API key from UniFi OS Settings > Admins & Users > API Keys",
        ),
    ];

    let mut missing = false;
    for (var, hint) in &required {
        match std::env::var(var) {
            Ok(v) if !v.is_empty() => println!("  ok {var}"),
            _ => {
                eprintln!("  missing {var}: {hint}");
                missing = true;
            }
        }
    }

    if missing {
        bail!(
            "Missing required environment variables; copy .env.example to .env and fill in values"
        );
    }
    println!("check-env: all required variables set");
    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run: {program} {}", args.join(" ")))?;
    if !status.success() {
        bail!(
            "{program} {} failed with exit code {:?}",
            args.join(" "),
            status.code()
        );
    }
    Ok(())
}

fn walkdir(root: &std::path::Path) -> Result<impl Iterator<Item = Result<std::path::PathBuf>>> {
    let mut entries = Vec::new();
    collect_files(root, &mut entries)?;
    Ok(entries.into_iter().map(Ok))
}

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}
