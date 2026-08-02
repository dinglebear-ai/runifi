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
    populate_default_site_id(cfg, path_template, &mut effective_params).await?;
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

async fn populate_default_site_id(
    cfg: &UnifiConfig,
    path_template: &str,
    params: &mut Value,
) -> Result<()> {
    if !path_template.contains("{siteId}") || params.get("siteId").is_some() {
        return Ok(());
    }
    let api = OfficialNetworkApi::new(&cfg.url);
    let sites_path = api.path("/v1/sites");
    let response = http::request_json(cfg, Method::GET, &sites_path, None, None)
        .await
        .context("failed to resolve UniFi official site ID")?;
    let site_id = select_site_id(&response, &cfg.site)?;
    let object = params
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("official action parameters must be a JSON object"))?;
    object.insert("siteId".to_string(), Value::String(site_id));
    Ok(())
}

fn select_site_id(response: &Value, configured_site: &str) -> Result<String> {
    let sites = response
        .get("data")
        .or_else(|| response.get("items"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("official site list response did not contain data/items"))?;

    let matches: Vec<&Value> = sites
        .iter()
        .filter(|site| {
            site.get("id").and_then(Value::as_str) == Some(configured_site)
                || site.get("internalReference").and_then(Value::as_str) == Some(configured_site)
                || site
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name.eq_ignore_ascii_case(configured_site))
        })
        .collect();

    let selected = match matches.as_slice() {
        [site] => *site,
        [] if sites.len() == 1 => &sites[0],
        [] => {
            let available = sites
                .iter()
                .filter_map(|site| {
                    let id = site.get("id")?.as_str()?;
                    let reference = site
                        .get("internalReference")
                        .and_then(Value::as_str)
                        .or_else(|| site.get("name").and_then(Value::as_str))
                        .unwrap_or("<unnamed>");
                    Some(format!("{reference} ({id})"))
                })
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "configured UniFi site {configured_site:?} did not match an official site; available: {available}"
            );
        }
        _ => bail!("configured UniFi site {configured_site:?} matched multiple official sites"),
    };

    selected
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("selected official UniFi site has no id"))
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
