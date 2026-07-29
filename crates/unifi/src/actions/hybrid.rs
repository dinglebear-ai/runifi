use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn resolve(action: &str, params: &Value) -> Result<(&'static str, Value)> {
    let prefer = params
        .get("prefer")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let has_site_id = params.get("siteId").is_some();
    let target = match prefer.as_deref() {
        Some("official") => official_target(action),
        Some("internal") => internal_target(action),
        Some(other) => bail!("unknown hybrid preference: {other}"),
        None if has_site_id => official_target(action),
        None => internal_target(action),
    };
    let Some(target) = target else {
        bail!("unknown hybrid action: {action}");
    };
    Ok((target, normalize_params(params)))
}

fn official_target(action: &str) -> Option<&'static str> {
    match action {
        "list_clients" => Some("official_list_clients"),
        "list_devices" => Some("official_list_devices"),
        "list_networks" => Some("official_list_networks"),
        "list_wifi" => Some("official_list_wifi"),
        "get_system_info" => Some("official_get_info"),
        _ => None,
    }
}

fn internal_target(action: &str) -> Option<&'static str> {
    match action {
        "list_clients" => Some("clients"),
        "list_devices" => Some("devices"),
        "list_networks" => Some("unifi_list_networks"),
        "list_wifi" => Some("wlans"),
        "get_system_info" => Some("sysinfo"),
        _ => None,
    }
}

fn normalize_params(params: &Value) -> Value {
    let mut value = params.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("prefer");
    }
    if value.is_null() { json!({}) } else { value }
}
