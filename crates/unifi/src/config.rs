use serde::{Deserialize, Serialize};

/// UniFi controller connection config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UnifiConfig {
    /// Controller base URL, e.g. <https://unifi.local> (`UNIFI_URL`).
    pub url: String,
    /// API key for the X-API-KEY header (UNIFI_API_KEY).
    pub api_key: String,
    /// Site name (UNIFI_SITE, default "default").
    pub site: String,
    /// Skip TLS certificate verification; required for self-signed certs.
    pub skip_tls_verify: bool,
    /// Legacy controller mode: no /proxy/network prefix, port 8443.
    pub legacy: bool,
}

impl Default for UnifiConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            api_key: String::new(),
            site: "default".to_string(),
            skip_tls_verify: true,
            legacy: false,
        }
    }
}
