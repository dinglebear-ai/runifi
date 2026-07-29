//! `unifi doctor` — pre-flight environment validation (§48)
//!
//! Checks every precondition before the user starts the server.
//! Runs even when UNIFI_URL / UNIFI_API_KEY are not set — that is the point.

use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::config::{Config, default_data_dir};

// ── check record ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DoctorCheck {
    pub category: &'static str,
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl DoctorCheck {
    fn pass(category: &'static str, name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: true,
            value: Some(value.into()),
            hint: None,
            latency_ms: None,
        }
    }

    fn fail(category: &'static str, name: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: false,
            value: None,
            hint: Some(hint.into()),
            latency_ms: None,
        }
    }

    fn warn(
        category: &'static str,
        name: impl Into<String>,
        value: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            category,
            name: name.into(),
            // Warnings still count as "ok" — they don't fail the exit code.
            // Callers that want to surface them can check hint.is_some() && ok.
            ok: true,
            value: Some(value.into()),
            hint: Some(hint.into()),
            latency_ms: None,
        }
    }
}

// ── individual checks ─────────────────────────────────────────────────────────

fn check_config_file(data_dir: &Path) -> DoctorCheck {
    let path = data_dir.join("config.toml");
    let display = path
        .to_str()
        .map(|s| s.replace(&std::env::var("HOME").unwrap_or_default(), "~"))
        .unwrap_or_else(|| path.display().to_string());
    if path.exists() {
        DoctorCheck::pass("config", "Config file", display)
    } else {
        DoctorCheck::warn(
            "config",
            "Config file",
            "not present",
            format!("{display} is optional; env vars override everything"),
        )
    }
}

fn check_dir_writable(label: &'static str, category: &'static str, dir: &Path) -> DoctorCheck {
    let display = dir
        .to_str()
        .map(|s| s.replace(&std::env::var("HOME").unwrap_or_default(), "~"))
        .unwrap_or_else(|| dir.display().to_string());

    if let Err(e) = std::fs::create_dir_all(dir) {
        return DoctorCheck::fail(category, label, format!("Cannot create {display}: {e}"));
    }

    // Probe writability
    let probe = dir.join(".doctor_write_probe");
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            // Report size for log dir
            let extra = if label.contains("Log") {
                let mb = dir_size_mb(dir);
                if mb > 0 {
                    format!("{display} (writable, {mb} MB)")
                } else {
                    format!("{display} (writable)")
                }
            } else {
                format!("{display} (writable)")
            };
            DoctorCheck::pass(category, label, extra)
        }
        Err(e) => DoctorCheck::fail(category, label, format!("{display} is not writable: {e}")),
    }
}

fn dir_size_mb(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let bytes: u64 = entries
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    bytes / (1024 * 1024)
}

fn check_binary_in_path(binary: &str) -> DoctorCheck {
    let found = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(PathBuf::from)
        .find(|dir| dir.join(binary).is_file())
        .map(|dir| dir.join(binary).display().to_string());

    match found {
        Some(path) => DoctorCheck::pass("config", "Binary in PATH".to_string(), path),
        None => DoctorCheck::fail(
            "config",
            "Binary in PATH",
            format!(
                "'{binary}' not found in $PATH — run install.sh or copy the binary to \
                 ~/.local/bin/{binary}"
            ),
        ),
    }
}

fn check_required_url(env_key: &'static str, value: &str) -> DoctorCheck {
    if value.is_empty() {
        return DoctorCheck::fail(
            "credentials",
            env_key,
            format!("Set {env_key} in ~/.unifi-rmcp/.env or your shell environment"),
        );
    }
    // Warn if http:// — most UniFi controllers serve HTTPS (even self-signed)
    if value.starts_with("http://") {
        DoctorCheck::warn(
            "credentials",
            env_key,
            format!("{value} (set)"),
            "URL uses http:// — UniFi controllers normally use https://. \
             Set UNIFI_SKIP_TLS_VERIFY=true if the cert is self-signed."
                .to_string(),
        )
    } else {
        DoctorCheck::pass("credentials", env_key, format!("{value} (set)"))
    }
}

fn check_required_var(env_key: &'static str, value: &str) -> DoctorCheck {
    if value.is_empty() {
        DoctorCheck::fail(
            "credentials",
            env_key,
            format!("Set {env_key} in ~/.unifi-rmcp/.env or your shell environment"),
        )
    } else {
        DoctorCheck::pass("credentials", env_key, "set".to_string())
    }
}

fn check_optional_var(env_key: &'static str, value: &str, default_note: &str) -> DoctorCheck {
    if value.is_empty() || value == "default" {
        DoctorCheck::pass(
            "credentials",
            env_key,
            format!("using default ({default_note})"),
        )
    } else {
        DoctorCheck::pass("credentials", env_key, format!("{value} (set)"))
    }
}

fn check_tls_note(skip_tls_verify: bool) -> DoctorCheck {
    if skip_tls_verify {
        DoctorCheck::pass(
            "credentials",
            "UNIFI_SKIP_TLS_VERIFY",
            "true — self-signed certs accepted (normal for UniFi)",
        )
    } else {
        DoctorCheck::warn(
            "credentials",
            "UNIFI_SKIP_TLS_VERIFY",
            "false (set)",
            "Most UniFi controllers use self-signed certs. \
             If connection fails, set UNIFI_SKIP_TLS_VERIFY=true",
        )
    }
}

async fn check_upstream(url: &str, api_key: &str, skip_tls: bool, legacy: bool) -> DoctorCheck {
    let self_path = if legacy {
        "/api/self"
    } else {
        "/proxy/network/api/self"
    };
    let endpoint = format!("{}{}", url.trim_end_matches('/'), self_path);
    let client = match reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(skip_tls)
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck::fail(
                "connectivity",
                "Upstream reachable",
                format!("Failed to build HTTP client: {e}"),
            );
        }
    };

    let start = Instant::now();
    let result = client
        .get(&endpoint)
        .header("X-API-KEY", api_key)
        .send()
        .await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            let display = format!("{endpoint} → {status} ({elapsed_ms} ms)");
            if status.is_success() || status.as_u16() == 401 {
                // 401 means we reached the controller (key may be wrong)
                let mut check = DoctorCheck::pass("connectivity", "Upstream reachable", display);
                check.latency_ms = Some(elapsed_ms);
                check
            } else {
                let mut check = DoctorCheck {
                    category: "connectivity",
                    name: "Upstream reachable".to_string(),
                    ok: false,
                    value: Some(display.clone()),
                    hint: Some(format!(
                        "Got HTTP {status} from {endpoint}. \
                         Check UNIFI_URL and API key permissions."
                    )),
                    latency_ms: Some(elapsed_ms),
                };
                // 401 is still "reachable" — key issue, not connectivity
                if status.as_u16() == 401 {
                    check.ok = true;
                    check.hint = Some(
                        "401 Unauthorized — controller reached but API key may be wrong. \
                         Regenerate in UniFi Settings > API."
                            .to_string(),
                    );
                }
                check
            }
        }
        Err(e) => {
            let hint = if e.is_connect() || e.is_timeout() {
                format!(
                    "Cannot reach {endpoint}: {e}. \
                     Check UNIFI_URL and that the controller is running. \
                     If using self-signed certs, set UNIFI_SKIP_TLS_VERIFY=true"
                )
            } else {
                format!("Request to {endpoint} failed: {e}")
            };
            DoctorCheck::fail("connectivity", "Upstream reachable", hint)
        }
    }
}

fn check_port_available(host: &str, port: u16) -> DoctorCheck {
    let name = format!("MCP port {port}");
    let probe_host = if host == "127.0.0.1" || host == "localhost" {
        host
    } else {
        "0.0.0.0"
    };
    let probe_addr = format!("{probe_host}:{port}");
    match TcpListener::bind(&probe_addr) {
        Ok(_) => DoctorCheck::pass("mcp", name, "available"),
        Err(_) => DoctorCheck::warn(
            "mcp",
            name,
            "in use",
            format!(
                "Port {port} is already in use. \
                 Change UNIFI_MCP_PORT if you need to run a second instance."
            ),
        ),
    }
}

// ── report rendering ──────────────────────────────────────────────────────────

fn print_doctor_report(checks: &[DoctorCheck], version: &str) {
    println!();
    println!("runifi {version} — environment check");
    println!();

    let categories = [
        ("config", "Config"),
        ("credentials", "Service credentials"),
        ("connectivity", "Connectivity"),
        ("mcp", "MCP server"),
    ];

    for (cat_key, cat_label) in &categories {
        let cat_checks: Vec<&DoctorCheck> =
            checks.iter().filter(|c| c.category == *cat_key).collect();
        if cat_checks.is_empty() {
            continue;
        }
        println!("  {cat_label}");
        println!("  {}", "─".repeat(44));
        for c in &cat_checks {
            let icon = if c.ok { "✓" } else { "✗" };
            let value_part = c
                .value
                .as_deref()
                .map(|v| format!(": {v}"))
                .unwrap_or_default();
            println!("  {icon} {:<26}{}", c.name, value_part);
            if let Some(hint) = &c.hint {
                if c.ok {
                    // Warning hint
                    println!("    ⚠  {hint}");
                } else {
                    println!("    → {hint}");
                }
            }
        }
        println!();
    }

    let issues = checks.iter().filter(|c| !c.ok).count();
    println!("  {}", "━".repeat(44));
    if issues == 0 {
        println!("  All checks passed. Ready to run: unifi serve");
    } else {
        println!(
            "  {issues} issue{} found. Fix {} before running: unifi serve",
            if issues == 1 { "" } else { "s" },
            if issues == 1 { "it" } else { "them" }
        );
    }
    println!();
}

// ── entry point ───────────────────────────────────────────────────────────────

/// Run the doctor pre-flight check.
///
/// This MUST work even when `UNIFI_URL` / `UNIFI_API_KEY` are not set — that is
/// the whole point. Do NOT construct a `UnifiClient` or `UnifiService` here.
pub async fn run_doctor(config: &Config, json: bool) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let data_dir = default_data_dir();
    let mut checks: Vec<DoctorCheck> = vec![
        // ── 1. Config / filesystem ────────────────────────────────────────────
        check_config_file(&data_dir),
        check_dir_writable("Data directory", "config", &data_dir),
        check_dir_writable("Log directory", "config", &data_dir.join("logs")),
        check_binary_in_path("runifi"),
        // ── 2. Required env vars / config fields ──────────────────────────────
        check_required_url("UNIFI_URL", &config.unifi.url),
        check_required_var("UNIFI_API_KEY", &config.unifi.api_key),
        check_optional_var("UNIFI_SITE", &config.unifi.site, "default"),
        check_tls_note(config.unifi.skip_tls_verify),
    ];

    // ── 3. Upstream connectivity (skip if URL is empty) ───────────────────────
    if !config.unifi.url.is_empty() {
        checks.push(
            check_upstream(
                &config.unifi.url,
                &config.unifi.api_key,
                config.unifi.skip_tls_verify,
                config.unifi.legacy,
            )
            .await,
        );
    }

    // ── 4. MCP port ───────────────────────────────────────────────────────────
    checks.push(check_port_available(&config.mcp.host, config.mcp.port));

    // ── 5. Output ─────────────────────────────────────────────────────────────
    let issues = checks.iter().filter(|c| !c.ok).count();

    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        print_doctor_report(&checks, version);
    }

    if issues > 0 {
        std::process::exit(1);
    }
    Ok(())
}
