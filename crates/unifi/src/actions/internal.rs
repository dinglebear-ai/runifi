use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    UnifiClient, UnifiConfig,
    api::{ApiSourceFamily, internal::InternalNetworkApi, path},
    capabilities::Capability,
    http,
};

pub async fn execute(cfg: &UnifiConfig, capability: &Capability, params: &Value) -> Result<Value> {
    if capability.source != ApiSourceFamily::Internal {
        bail!("{} is not an internal API action", capability.action);
    }

    let client = UnifiClient::new(cfg)?;
    match capability.action.as_str() {
        "clients" => client.clients().await,
        "devices" => client.devices().await,
        "wlans" => client.wlans().await,
        "health" => client.health().await,
        "alarms" => client.alarms().await,
        "events" => {
            let mut events = execute_generic(cfg, capability, params).await?;
            truncate_data_array(
                &mut events,
                params
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
            );
            Ok(events)
        }
        "sysinfo" => client.sysinfo().await,
        "me" => client.me().await,
        _ => execute_generic(cfg, capability, params).await,
    }
}

async fn execute_generic(
    cfg: &UnifiConfig,
    capability: &Capability,
    params: &Value,
) -> Result<Value> {
    let mut method = capability
        .method
        .as_deref()
        .unwrap_or("GET")
        .parse::<Method>()
        .context("invalid internal HTTP method")?;
    let mut path = capability
        .path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("internal action {} has no path", capability.action))?;
    let mut effective_params = params.clone();
    normalize_internal_request(
        capability.action.as_str(),
        &mut method,
        &mut path,
        &mut effective_params,
    );
    let path = path::substitute_path(path, params, &[])?;
    let api = InternalNetworkApi::new(&cfg.url, &cfg.site, cfg.legacy);
    let full_path = if path == "/api/self" {
        if cfg.legacy {
            path
        } else {
            "/proxy/network/api/self".to_string()
        }
    } else if path.starts_with("/api/") {
        if cfg.legacy {
            path
        } else {
            format!("/proxy/network{path}")
        }
    } else if let Some(suffix) = path.strip_prefix("/v2/") {
        api.v2_site_path(suffix.trim_start_matches("api/site/{site}/"))
    } else {
        api.v1_site_path(&path)
    };
    let mut value = http::request_json(
        cfg,
        method,
        &full_path,
        effective_params.get("query"),
        effective_params.get("body"),
    )
    .await?;
    if capability.action == "unifi_get_ips_events" {
        retain_security_events(&mut value);
    }
    Ok(value)
}

fn normalize_internal_request(
    action: &str,
    method: &mut Method,
    path: &mut &str,
    params: &mut Value,
) {
    match action {
        "events" | "unifi_recent_events" => {
            *method = Method::POST;
            *path = "/v2/system-log/all";
            ensure_body(params, json!({}));
        }
        "unifi_get_ips_events" => {
            *method = Method::POST;
            *path = "/v2/system-log/all";
            ensure_body(params, json!({}));
        }
        "unifi_get_traffic_flow_statistics" => {
            *method = Method::POST;
            *path = "/v2/traffic-flows";
            ensure_body(params, json!({}));
        }
        "unifi_get_gateway_settings" => {
            *method = Method::GET;
            *path = "/get/setting/mgmt";
        }
        "unifi_get_client_sessions" => {
            ensure_body(params, default_session_body());
        }
        "unifi_get_alerts"
        | "unifi_get_event_types"
        | "unifi_get_traffic_flows"
        | "unifi_list_alarms"
        | "unifi_list_events" => {
            ensure_body(params, json!({}));
        }
        _ => {}
    }
}

fn ensure_body(params: &mut Value, body: Value) {
    if params.get("body").is_some() {
        return;
    }
    if !params.is_object() {
        *params = json!({});
    }
    if let Some(object) = params.as_object_mut() {
        object.insert("body".to_string(), body);
    }
}

fn default_session_body() -> Value {
    let end = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let start = end.saturating_sub(24 * 60 * 60 * 1000);
    json!({ "start": start, "end": end })
}

fn retain_security_events(value: &mut Value) {
    let Some(items) = value.get_mut("data").and_then(Value::as_array_mut) else {
        return;
    };
    items.retain(|item| {
        matches!(
            item.get("category").and_then(Value::as_str),
            Some("SECURITY")
        ) || item
            .get("key")
            .and_then(Value::as_str)
            .is_some_and(|key| key.contains("THREAT") || key.contains("IPS"))
            || item
                .get("subcategory")
                .and_then(Value::as_str)
                .is_some_and(|subcategory| subcategory.contains("SECURITY"))
    });
}

fn truncate_data_array(value: &mut Value, limit: Option<usize>) {
    let Some(limit) = limit else {
        return;
    };
    if let Some(items) = value.get_mut("data").and_then(Value::as_array_mut) {
        items.truncate(limit);
    }
}
