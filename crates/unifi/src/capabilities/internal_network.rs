use serde::Deserialize;

use crate::api::ApiSourceFamily;
use crate::capabilities::{AuthScope, Capability};

#[derive(Debug, Deserialize)]
struct Inventory {
    tools: Vec<Tool>,
}

#[derive(Debug, Deserialize)]
struct Tool {
    action: String,
    method: String,
    path: String,
    title: String,
    mutating: bool,
    runtime: bool,
    auth_scope: String,
    verification_mode: String,
}

pub fn capabilities() -> Vec<Capability> {
    let inventory: Inventory = serde_json::from_str(include_str!(
        "../../../../data/unifi_internal_endpoint_models.json"
    ))
    .expect("internal UniFi endpoint models should be valid JSON");
    let mut caps = inventory
        .tools
        .into_iter()
        .filter(|tool| tool.runtime)
        .map(|tool| Capability {
            action: tool.action,
            title: tool.title,
            source: ApiSourceFamily::Internal,
            method: Some(tool.method),
            path: Some(tool.path),
            mutating: tool.mutating,
            auth_scope: auth_scope(&tool.auth_scope),
            verification_mode: Some(tool.verification_mode),
        })
        .collect::<Vec<_>>();
    caps.extend([
        legacy("clients", "Clients", "GET", "/stat/sta"),
        legacy("devices", "Devices", "GET", "/stat/device"),
        legacy("wlans", "WLANs", "GET", "/rest/wlanconf"),
        legacy("health", "Health", "GET", "/stat/health"),
        legacy("alarms", "Alarms", "GET", "/stat/alarm"),
        legacy("events", "Events", "GET", "/rest/event"),
        legacy("sysinfo", "System Info", "GET", "/stat/sysinfo"),
        legacy("me", "Current User", "GET", "/api/self"),
        hybrid("list_clients", "List Clients"),
        hybrid("list_devices", "List Devices"),
        hybrid("list_networks", "List Networks"),
        hybrid("list_wifi", "List WiFi"),
        hybrid("get_system_info", "Get System Info"),
        composite("get_client_rf_history", "Get Client RF History"),
    ]);
    caps
}

fn legacy(action: &str, title: &str, method: &str, path: &str) -> Capability {
    Capability {
        action: action.to_string(),
        title: title.to_string(),
        source: ApiSourceFamily::Internal,
        method: Some(method.to_string()),
        path: Some(path.to_string()),
        mutating: false,
        auth_scope: AuthScope::Read,
        verification_mode: Some("legacy_alias".to_string()),
    }
}

fn hybrid(action: &str, title: &str) -> Capability {
    Capability {
        action: action.to_string(),
        title: title.to_string(),
        source: ApiSourceFamily::Hybrid,
        method: None,
        path: None,
        mutating: false,
        auth_scope: AuthScope::Read,
        verification_mode: Some("contract_ok".to_string()),
    }
}

fn composite(action: &str, title: &str) -> Capability {
    Capability {
        action: action.to_string(),
        title: title.to_string(),
        source: ApiSourceFamily::Internal,
        method: None,
        path: None,
        mutating: false,
        auth_scope: AuthScope::Read,
        verification_mode: Some("composite".to_string()),
    }
}

fn auth_scope(scope: &str) -> AuthScope {
    match scope {
        "read" => AuthScope::Read,
        "admin" => AuthScope::Admin,
        other => panic!("unknown internal auth_scope {other}"),
    }
}
