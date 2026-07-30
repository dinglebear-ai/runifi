//! `runifi` command-line and MCP server entry point.

use anyhow::Result;
use std::sync::Arc;

use rmcp::{ServiceExt, transport::stdio};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use unifi::{UnifiClient, UnifiService};
use unifi_rmcp::{
    cli,
    config::{AuthMode, Config},
    mcp::{self, AppState, AuthPolicy},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [f] if matches!(f.as_str(), "--help" | "-h" | "help") => {
            print_usage();
            return Ok(());
        }
        [f] if matches!(f.as_str(), "--version" | "-V" | "version") => {
            println!("runifi {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    // Doctor runs before we build any client — the whole point is it works even
    // when UNIFI_URL / UNIFI_API_KEY are missing.
    let is_doctor = args.iter().any(|a| !a.starts_with('-'))
        && args
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(String::as_str)
            == Some("doctor");
    if is_doctor {
        let json = args.iter().any(|a| a == "--json");
        let config = Config::load().unwrap_or_default();
        return cli::doctor::run_doctor(&config, json).await;
    }
    if let Some((command, json)) = unifi_rmcp::setup::SetupCommand::parse(&args)? {
        return unifi_rmcp::setup::run(command, json);
    }

    // Load ~/.unifi/.env (or /data/.env in a container) before any Config::load
    // so the binary works on bare metal without a process manager injecting env.
    // Non-overriding: explicit process env still wins.
    unifi_rmcp::config::load_dotenv();

    let stdio_mode = matches!(args.as_slice(), [c] if c == "mcp");
    let serve_mode = args.is_empty()
        || matches!(args.as_slice(), [c] if c == "serve")
        || matches!(args.as_slice(), [a, b] if a == "serve" && b == "mcp");

    let log_level = if stdio_mode || !serve_mode {
        "warn"
    } else {
        "info"
    };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    if serve_mode {
        serve_mcp().await
    } else if stdio_mode {
        serve_stdio_mcp().await
    } else {
        run_cli(args).await
    }
}

async fn serve_mcp() -> Result<()> {
    let config = Config::load()?;
    validate_bind_security(&config.mcp)?;
    let state = build_state(config).await?;

    info!(
        bind = %state.config.bind_addr(),
        server_name = %state.config.server_name,
        auth = ?state.auth_policy,
        "runifi starting"
    );

    let bind = state.config.bind_addr();
    let app = mcp::router(state).layer(tower_http::trace::TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(bind = %bind, "MCP HTTP server listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn serve_stdio_mcp() -> Result<()> {
    // Stdio is always LoopbackDev — trusted local pipe, no HTTP auth context.
    let config = Config::load()?;
    let service = UnifiService::new(UnifiClient::new(&config.unifi)?);
    #[allow(clippy::useless_conversion)]
    let state = AppState {
        config: config.mcp,
        auth_policy: AuthPolicy::LoopbackDev,
        service,
    };
    let _ = (); // appease compiler
    let svc = mcp::rmcp_server(state).serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

async fn run_cli(args: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let service = UnifiService::new(UnifiClient::new(&config.unifi)?);
    let (cmd, json) = cli::CliCommand::parse(&args)?;
    cli::run(&service, cmd, json).await
}

async fn build_state(config: Config) -> Result<AppState> {
    let auth_policy = build_auth_policy(&config).await?;
    let service = UnifiService::new(UnifiClient::new(&config.unifi)?);
    Ok(AppState {
        config: config.mcp,
        auth_policy,
        service,
    })
}

async fn build_auth_policy(config: &Config) -> Result<AuthPolicy> {
    if config.mcp.no_auth || config.mcp.host.starts_with("127.") {
        return Ok(AuthPolicy::LoopbackDev);
    }
    if config.mcp.auth.mode == AuthMode::OAuth {
        let auth_cfg = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("UNIFI_MCP")
            .session_cookie_name("unifi_mcp_session")
            .scopes_supported(vec!["unifi:read".into(), "unifi:admin".into()])
            .default_scope("unifi:read")
            .resource_path("/mcp")
            .enable_dynamic_registration(true)
            .build_from_sources(std::env::vars())
            .map_err(|e| anyhow::anyhow!("OAuth config error: {e}"))?;
        let auth_state = lab_auth::state::AuthState::new(auth_cfg)
            .await
            .map_err(|e| anyhow::anyhow!("OAuth state init error: {e}"))?;
        Ok(AuthPolicy::Mounted {
            auth_state: Some(Arc::new(auth_state)),
        })
    } else {
        Ok(AuthPolicy::Mounted { auth_state: None })
    }
}

fn print_usage() {
    eprintln!(
        "Usage:
  runifi [serve]                         Start MCP HTTP server (port 40030)
  runifi mcp                             Start MCP stdio transport
  runifi doctor [--json]                 Pre-flight environment check
  runifi setup check [--json]            Check local plugin setup
  runifi setup repair [--json]           Repair local plugin setup
  runifi setup plugin-hook [--no-repair] [--json]

Network:
  runifi clients [--json]                Connected wireless and wired clients
  runifi devices [--json]                Network devices: APs, switches, gateways
  runifi wlans [--json]                  WiFi network configurations
  runifi health [--json]                 Site health summary
  runifi alarms [--json]                 Active alarms and alerts
  runifi events [--limit N] [--json]     Recent controller events
  runifi sysinfo [--json]                Controller system information
  runifi me [--json]                     Authenticated user info
  runifi <action> [--param k=v] [--body-json JSON] [--json]

Environment:
  UNIFI_URL                     Controller base URL (required), e.g. https://unifi.local
  UNIFI_API_KEY                 API key for X-API-KEY header (required)
  UNIFI_SITE                    Site name (default: default)
  UNIFI_SITE_ID                 Official API site UUID for live tests
  UNIFI_SKIP_TLS_VERIFY         Skip TLS cert check (default: true)
  UNIFI_LEGACY                  Legacy controller mode, no /proxy/network prefix (default: false)
  UNIFI_MCP_HOST                Bind host (default: 0.0.0.0)
  UNIFI_MCP_PORT                Bind port (default: 40030)
  UNIFI_MCP_TOKEN               Static bearer token for MCP auth
  UNIFI_MCP_NO_AUTH             Disable MCP auth (loopback only)
  RUST_LOG                      Log filter"
    );
}

/// Refuse to bind to a non-loopback address without authentication configured,
/// unless the operator explicitly sets UNIFI_NOAUTH=true.
fn validate_bind_security(config: &unifi_rmcp::config::McpConfig) -> anyhow::Result<()> {
    let is_loopback =
        config.host.starts_with("127.") || config.host == "::1" || config.host == "localhost";
    let has_auth = !config.no_auth && config.api_token.is_some();
    let noauth_override = std::env::var("UNIFI_NOAUTH")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);

    if !is_loopback && !has_auth && !noauth_override {
        anyhow::bail!(
            "Refusing to bind MCP server to {} without authentication.\n\
             Set UNIFI_MCP_TOKEN, use UNIFI_MCP_AUTH_MODE=oauth, or set \
             UNIFI_NOAUTH=true if an upstream gateway handles auth.",
            config.host
        );
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "CTRL+C handler failed");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown signal received");
}
