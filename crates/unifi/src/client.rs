use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::{UnifiConfig, http};

/// HTTP REST client for UniFi controllers.
///
/// Supports both modern UniFi OS with `/proxy/network` and legacy controllers
/// without that prefix. Authentication uses the `X-API-KEY` header.
#[derive(Clone)]
pub struct UnifiClient {
    client: Client,
    /// Base URL, e.g. `https://unifi.local`.
    pub url: String,
    api_key: String,
    site: String,
    skip_tls_verify: bool,
    legacy: bool,
}

impl UnifiClient {
    pub fn new(cfg: &UnifiConfig) -> Result<Self> {
        if cfg.url.is_empty() {
            anyhow::bail!(
                "UNIFI_URL is not set - set it to your controller's base URL, \
                 e.g. UNIFI_URL=https://unifi.local"
            );
        }
        if cfg.api_key.is_empty() {
            anyhow::bail!(
                "UNIFI_API_KEY is not set - generate an API key in \
                 UniFi Settings > API"
            );
        }
        let client = http::client(cfg)?;
        Ok(Self {
            client,
            url: cfg.url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
            site: cfg.site.clone(),
            skip_tls_verify: cfg.skip_tls_verify,
            legacy: cfg.legacy,
        })
    }

    pub fn config(&self) -> UnifiConfig {
        UnifiConfig {
            url: self.url.clone(),
            api_key: self.api_key.clone(),
            site: self.site.clone(),
            skip_tls_verify: self.skip_tls_verify,
            legacy: self.legacy,
        }
    }

    fn site_path(&self, suffix: &str) -> String {
        let prefix = if self.legacy { "" } else { "/proxy/network" };
        format!("{prefix}/api/s/{site}/{suffix}", site = self.site)
    }

    fn self_path(&self) -> &'static str {
        if self.legacy {
            "/api/self"
        } else {
            "/proxy/network/api/self"
        }
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{path}", self.url);
        let resp = self
            .client
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!(
                        "UniFi controller at {url} timed out after 30s - \
                         check UNIFI_SKIP_TLS_VERIFY=true for self-signed certs, \
                         or verify the controller is reachable"
                    )
                } else if e.is_connect() {
                    anyhow::anyhow!(
                        "UniFi controller at {url} unreachable - \
                         check UNIFI_URL is correct and the controller is running. \
                         For self-signed certs set UNIFI_SKIP_TLS_VERIFY=true"
                    )
                } else {
                    anyhow::anyhow!("GET {url} failed: {e}")
                }
            })?;

        let status = resp.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!(
                "UNIFI_API_KEY rejected by {url} (HTTP 401) - \
                 generate a new API key in UniFi Settings > API"
            );
        }

        let body: Value = resp
            .json()
            .await
            .with_context(|| format!("failed to parse JSON response from {url}"))?;

        if !status.is_success() {
            anyhow::bail!("UniFi HTTP {status} from {url}: {body}");
        }

        Ok(body)
    }

    /// Connected clients (wireless and wired).
    pub async fn clients(&self) -> Result<Value> {
        let path = self.site_path("stat/sta");
        let span = tracing::info_span!("upstream.clients", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi clients API");
        let result = self.get(&path).await;
        self.log_result(&result, "clients");
        result
    }

    /// Network devices: APs, switches, gateways.
    pub async fn devices(&self) -> Result<Value> {
        let path = self.site_path("stat/device");
        let span = tracing::info_span!("upstream.devices", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi devices API");
        let result = self.get(&path).await;
        self.log_result(&result, "devices");
        result
    }

    /// WLAN (WiFi network) configurations.
    pub async fn wlans(&self) -> Result<Value> {
        let path = self.site_path("rest/wlanconf");
        let span = tracing::info_span!("upstream.wlans", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi wlans API");
        let result = self.get(&path).await;
        self.log_result(&result, "wlans");
        result
    }

    /// Site health summary.
    pub async fn health(&self) -> Result<Value> {
        let path = self.site_path("stat/health");
        let span = tracing::info_span!("upstream.health", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi health API");
        let result = self.get(&path).await;
        self.log_result(&result, "health");
        result
    }

    /// Active alarms / alerts.
    pub async fn alarms(&self) -> Result<Value> {
        let path = self.site_path("rest/alarm");
        let span = tracing::info_span!("upstream.alarms", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi alarms API");
        let result = self.get(&path).await;
        self.log_result(&result, "alarms");
        result
    }

    /// Recent events.
    pub async fn events(&self) -> Result<Value> {
        let path = self.site_path("rest/event");
        let span = tracing::info_span!("upstream.events", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi events API");
        let result = self.get(&path).await;
        self.log_result(&result, "events");
        result
    }

    /// Controller system info.
    pub async fn sysinfo(&self) -> Result<Value> {
        let path = self.site_path("stat/sysinfo");
        let span = tracing::info_span!("upstream.sysinfo", site = %self.site);
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi sysinfo API");
        let result = self.get(&path).await;
        self.log_result(&result, "sysinfo");
        result
    }

    /// Authenticated user info.
    pub async fn me(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.me");
        let _guard = span.enter();
        tracing::debug!(url = %self.url, "calling UniFi me API");
        let result = self.get(self.self_path()).await;
        self.log_result(&result, "me");
        result
    }

    fn log_result(&self, result: &Result<Value>, action: &str) {
        match result {
            Ok(v) => {
                let count = v
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                tracing::debug!(action, count, "upstream call ok");
            }
            Err(e) => {
                tracing::warn!(action, error = %e, "upstream call failed");
            }
        }
    }
}
