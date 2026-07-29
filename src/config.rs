use serde::{Deserialize, Serialize};
pub use unifi::UnifiConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub mcp: McpConfig,
    pub unifi: UnifiConfig,
}

/// MCP HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    #[serde(default = "default_mcp_host")]
    pub host: String,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_server_name")]
    pub server_name: String,
    /// Disable auth entirely (only legal when bound to loopback)
    pub no_auth: bool,
    /// Static bearer token (UNIFI_MCP_TOKEN)
    pub api_token: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub auth: AuthConfig,
}

impl McpConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// OAuth / auth sub-config (nested under `[mcp.auth]` in config.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub public_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub admin_email: String,
    pub allowed_emails: Vec<String>,
    pub sqlite_path: String,
    pub key_path: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub auth_code_ttl_secs: u64,
    pub register_rpm: u32,
    pub authorize_rpm: u32,
    pub disable_static_token_with_oauth: bool,
    pub allowed_client_redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    #[default]
    Bearer,
    OAuth,
}

// ── defaults ──────────────────────────────────────────────────────────────────

fn default_mcp_host() -> String {
    "0.0.0.0".into()
}
fn default_mcp_port() -> u16 {
    40030
}
fn default_server_name() -> String {
    "unifi-rmcp".into()
}
fn default_auth_sqlite_path() -> String {
    "/data/auth.db".into()
}
fn default_auth_key_path() -> String {
    "/data/auth-jwt.pem".into()
}
fn default_access_token_ttl_secs() -> u64 {
    3600
}
fn default_refresh_token_ttl_secs() -> u64 {
    86400 * 30
}
fn default_auth_code_ttl_secs() -> u64 {
    300
}
fn default_register_rpm() -> u32 {
    10
}
fn default_authorize_rpm() -> u32 {
    60
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: default_mcp_host(),
            port: default_mcp_port(),
            server_name: default_server_name(),
            no_auth: false,
            api_token: None,
            allowed_hosts: Vec::new(),
            allowed_origins: Vec::new(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::default(),
            public_url: None,
            google_client_id: None,
            google_client_secret: None,
            admin_email: String::new(),
            allowed_emails: Vec::new(),
            sqlite_path: default_auth_sqlite_path(),
            key_path: default_auth_key_path(),
            access_token_ttl_secs: default_access_token_ttl_secs(),
            refresh_token_ttl_secs: default_refresh_token_ttl_secs(),
            auth_code_ttl_secs: default_auth_code_ttl_secs(),
            register_rpm: default_register_rpm(),
            authorize_rpm: default_authorize_rpm(),
            disable_static_token_with_oauth: true,
            allowed_client_redirect_uris: Vec::new(),
        }
    }
}

// ── Config loading ────────────────────────────────────────────────────────────

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        match std::fs::read_to_string("config.toml") {
            Ok(contents) => {
                config = toml::from_str(&contents)
                    .map_err(|e| anyhow::anyhow!("Failed to parse config.toml: {e}"))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(anyhow::anyhow!("Failed to read config.toml: {e}")),
        }

        // Env overrides — MCP (UNIFI_MCP_* prefix)
        env_str("UNIFI_MCP_HOST", &mut config.mcp.host);
        env_parse("UNIFI_MCP_PORT", &mut config.mcp.port)?;
        env_bool("UNIFI_MCP_NO_AUTH", &mut config.mcp.no_auth)?;
        env_opt_str("UNIFI_MCP_TOKEN", &mut config.mcp.api_token);
        env_list("UNIFI_MCP_ALLOWED_HOSTS", &mut config.mcp.allowed_hosts);
        env_list("UNIFI_MCP_ALLOWED_ORIGINS", &mut config.mcp.allowed_origins);
        env_opt_str("UNIFI_MCP_PUBLIC_URL", &mut config.mcp.auth.public_url);
        env_str(
            "UNIFI_MCP_AUTH_ADMIN_EMAIL",
            &mut config.mcp.auth.admin_email,
        );
        // Auth mode: 'oauth' enables the full OAuth flow; anything else stays bearer.
        if let Ok(v) = std::env::var("UNIFI_MCP_AUTH_MODE") {
            config.mcp.auth.mode = match v.trim().to_lowercase().as_str() {
                "oauth" => AuthMode::OAuth,
                _ => AuthMode::Bearer,
            };
        }

        // Disable-auth compatibility alias
        if std::env::var("UNIFI_MCP_DISABLE_HTTP_AUTH")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false)
        {
            config.mcp.no_auth = true;
        }

        // Env overrides — UniFi controller (UNIFI_* prefix)
        env_str("UNIFI_URL", &mut config.unifi.url);
        env_str("UNIFI_API_KEY", &mut config.unifi.api_key);
        env_str("UNIFI_SITE", &mut config.unifi.site);
        env_bool("UNIFI_SKIP_TLS_VERIFY", &mut config.unifi.skip_tls_verify)?;
        env_bool("UNIFI_LEGACY", &mut config.unifi.legacy)?;

        Ok(config)
    }
}

// ── env helpers ───────────────────────────────────────────────────────────────

fn env_str(key: &str, target: &mut String) {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        *target = v;
    }
}

fn env_opt_str(key: &str, target: &mut Option<String>) {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        *target = Some(v);
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, target: &mut T) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        *target = v
            .parse()
            .map_err(|_| anyhow::anyhow!("{key}: invalid value {v:?}"))?;
    }
    Ok(())
}

fn env_bool(key: &str, target: &mut bool) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key) {
        match v.to_lowercase().as_str() {
            "1" | "true" | "yes" => *target = true,
            "0" | "false" | "no" => *target = false,
            other => anyhow::bail!("{key}: expected bool, got {other:?}"),
        }
    }
    Ok(())
}

fn env_list(key: &str, target: &mut Vec<String>) {
    if let Ok(v) = std::env::var(key) {
        let items: Vec<String> = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !items.is_empty() {
            *target = items;
        }
    }
}

// ── Appdata directory ─────────────────────────────────────────────────────────

/// Resolve the local data directory for unifi-rmcp.
///
/// - Inside a container (`/.dockerenv` exists or `RUNNING_IN_CONTAINER` set): `/data`
/// - Bare-metal: `~/.unifi-rmcp/`
pub fn default_data_dir() -> std::path::PathBuf {
    if std::path::Path::new("/.dockerenv").exists()
        || std::env::var("RUNNING_IN_CONTAINER").is_ok()
        || std::env::var("container").is_ok()
    {
        return std::path::PathBuf::from("/data");
    }
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".unifi-rmcp")
}

/// Load `~/.unifi-rmcp/.env` (or `/data/.env` in a container) into the process
/// environment if present.
///
/// Best-effort: a missing file is ignored, and existing env vars are NOT
/// overridden — values injected by docker-compose/systemd or the plugin hook's
/// `CLAUDE_PLUGIN_OPTION_*` mapping still take precedence. Lets the binary find
/// its credentials directly from `~/.unifi-rmcp/.env` without a process manager.
/// Call once at startup before `Config::load`. A symlinked `.env` is refused
/// (the dir holds secrets; mirrors axon).
pub fn load_dotenv() {
    let env_path = default_data_dir().join(".env");
    match std::fs::symlink_metadata(&env_path) {
        Ok(md) if md.file_type().is_symlink() => {
            eprintln!(
                "error: refusing to load symlinked .env at {} (potential symlink attack)",
                env_path.display()
            );
            std::process::exit(1);
        }
        Ok(_) => {
            let _ = dotenvy::from_path(&env_path);
        }
        Err(_) => {}
    }
}
