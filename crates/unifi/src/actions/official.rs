use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};

use crate::{
    UnifiConfig,
    api::{ApiSourceFamily, official::OfficialNetworkApi, path},
    capabilities::Capability,
    http,
};

const CONNECTOR_PREFIXES: &[&str] = &["/proxy/network/integration/", "/proxy/protect/integration/"];

pub async fn execute(cfg: &UnifiConfig, capability: &Capability, params: &Value) -> Result<Value> {
    if capability.source != ApiSourceFamily::Official {
        bail!("{} is not an official API action", capability.action);
    }
    let path_template = capability
        .path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("official action {} has no path", capability.action))?;
    let method = capability
        .method
        .as_deref()
        .unwrap_or("GET")
        .parse::<Method>()
        .context("invalid official HTTP method")?;
    let mut effective_params = params.clone();
    normalize_official_request(capability.action.as_str(), &mut effective_params);
    let path = path::substitute_path(path_template, &effective_params, CONNECTOR_PREFIXES)?;
    let api = OfficialNetworkApi::new(&cfg.url);
    let full_path = api.path(&path);
    http::request_json(
        cfg,
        method,
        &full_path,
        effective_params.get("query"),
        effective_params.get("body"),
    )
    .await
}

fn normalize_official_request(action: &str, params: &mut Value) {
    if action != "official_get_firewall_policy_ordering" {
        return;
    }
    let Some(zone_id) = params
        .get("query")
        .and_then(|query| query.get("firewallZoneId"))
        .cloned()
    else {
        return;
    };
    if !params.is_object() {
        *params = json!({});
    }
    let object = params.as_object_mut().expect("params object");
    let query = object.entry("query").or_insert_with(|| json!({}));
    if !query.is_object() {
        return;
    }
    let query = query.as_object_mut().expect("query object");
    query
        .entry("sourceFirewallZoneId".to_string())
        .or_insert_with(|| zone_id.clone());
    query
        .entry("destinationFirewallZoneId".to_string())
        .or_insert(zone_id);
}
