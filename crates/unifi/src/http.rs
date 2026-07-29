use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};

use crate::UnifiConfig;

pub fn client(cfg: &UnifiConfig) -> Result<Client> {
    reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(cfg.skip_tls_verify)
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")
}

pub async fn request_json(
    cfg: &UnifiConfig,
    method: Method,
    path: &str,
    query: Option<&Value>,
    body: Option<&Value>,
) -> Result<Value> {
    let client = client(cfg)?;
    let url = format!("{}{}", cfg.url.trim_end_matches('/'), path);
    let mut request = client
        .request(method.clone(), &url)
        .header("X-API-KEY", &cfg.api_key)
        .header("Accept", "application/json");

    if let Some(query) = query {
        let query = query
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("query must be a JSON object"))?;
        request = request.query(query);
    }
    if let Some(body) = body {
        request = request.json(body);
    }

    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("UniFi controller at {url} timed out after 30s")
        } else if e.is_connect() {
            anyhow::anyhow!("UniFi controller at {url} unreachable")
        } else {
            anyhow::anyhow!("{method} {url} failed: {e}")
        }
    })?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read response body from {url}"))?;

    if status == StatusCode::UNAUTHORIZED {
        anyhow::bail!("UNIFI_API_KEY rejected by {url} (HTTP 401)");
    }
    if status == StatusCode::FORBIDDEN {
        anyhow::bail!("UniFi API key lacks permission for {method} {url} (HTTP 403)");
    }
    if status == StatusCode::NOT_FOUND {
        anyhow::bail!("UniFi endpoint not found for {method} {url} (HTTP 404)");
    }

    let value = if bytes.is_empty() {
        if method == Method::GET {
            anyhow::bail!("UniFi returned an empty body for {method} {url}");
        }
        json!({
            "success": true,
            "status": status.as_u16(),
            "method": method.as_str(),
            "path": path,
        })
    } else {
        serde_json::from_slice::<Value>(&bytes)
            .with_context(|| format!("failed to parse JSON response from {url}"))?
    };

    if !status.is_success() {
        anyhow::bail!("UniFi HTTP {status} from {url}: {value}");
    }
    Ok(value)
}
