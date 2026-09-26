use serde_json::{Map, Value, json};

use unifi::{
    api::ApiSourceFamily,
    capabilities::{Capability, all_capabilities},
};

pub fn tool_definitions() -> Vec<Value> {
    let capabilities = all_capabilities();
    let mut tools = capabilities
        .iter()
        .map(atomic_tool_definition)
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["name"].as_str().unwrap_or_default())
    });
    tools.push(compatibility_router_definition(capabilities));
    tools
}

fn atomic_tool_definition(capability: &Capability) -> Value {
    json!({
        "name": capability.action,
        "description": atomic_description(capability),
        "annotations": {
            "auth_scope": capability.auth_scope.as_str(),
            "verification_mode": capability.verification_mode,
            "mutating": capability.mutating,
            "source": source_name(capability.source),
            "method": capability.method,
            "path": capability.path,
        },
        "inputSchema": atomic_input_schema(capability),
    })
}

fn atomic_description(capability: &Capability) -> String {
    let access = if capability.mutating {
        "Mutating operation; requires UniFi admin authorization."
    } else {
        "Read-only operation."
    };
    match (capability.method.as_deref(), capability.path.as_deref()) {
        (Some(method), Some(path)) => format!(
            "{} Atomic UniFi {} operation: {method} {path}. {access}",
            capability.title,
            source_name(capability.source),
        ),
        _ => format!(
            "{} Atomic UniFi {} operation. {access}",
            capability.title,
            source_name(capability.source),
        ),
    }
}

fn atomic_input_schema(capability: &Capability) -> Value {
    match capability.action.as_str() {
        "unifi_get_client_stats" => client_stats_schema(),
        "unifi_get_client_wifi_details" => client_wifi_details_schema(),
        "get_client_rf_history" => client_rf_history_schema(),
        "events" => json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Maximum number of events to return."
                }
            },
            "additionalProperties": false
        }),
        "list_clients" | "list_devices" | "list_networks" | "list_wifi" | "get_system_info" => {
            hybrid_schema()
        }
        "clients" | "devices" | "wlans" | "health" | "alarms" | "sysinfo" | "me" => {
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        }
        _ => generated_schema(capability),
    }
}

fn client_stats_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "client_id": {
                "type": "string",
                "description": "Client MAC address, UniFi client _id, or current client IP."
            },
            "granularity": {
                "type": "string",
                "enum": ["5minutes", "hourly", "daily", "monthly"],
                "description": "Historical report resolution. Defaults to hourly."
            },
            "duration": {
                "type": "string",
                "enum": ["hourly", "daily", "weekly", "monthly"],
                "description": "Lookback preset when start is omitted. Defaults to hourly."
            },
            "start": {
                "type": "integer",
                "minimum": 0,
                "description": "Optional range start as Unix epoch milliseconds."
            },
            "end": {
                "type": "integer",
                "minimum": 0,
                "description": "Optional range end as Unix epoch milliseconds. Defaults to now."
            }
        },
        "required": ["client_id"],
        "additionalProperties": false
    })
}

fn client_wifi_details_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "client_mac": {
                "type": "string",
                "description": "Client MAC address."
            }
        },
        "required": ["client_mac"],
        "additionalProperties": false
    })
}

fn client_rf_history_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "client_id": {
                "type": "string",
                "description": "Client MAC address, UniFi client _id, or current client IP."
            },
            "client_mac": {
                "type": "string",
                "description": "Client MAC address."
            },
            "ip": {
                "type": "string",
                "description": "Current client IPv4 address."
            },
            "granularity": {
                "type": "string",
                "enum": ["5minutes", "hourly", "daily", "monthly"],
                "description": "Historical report resolution. Auto-selected from the requested range when omitted."
            },
            "duration": {
                "type": "string",
                "enum": ["hourly", "daily", "weekly", "monthly"],
                "description": "Lookback preset when start is omitted. Defaults to daily."
            },
            "start": {
                "type": "integer",
                "minimum": 0,
                "description": "Optional range start as Unix epoch milliseconds."
            },
            "end": {
                "type": "integer",
                "minimum": 0,
                "description": "Optional range end as Unix epoch milliseconds. Defaults to now."
            },
            "ap_mac": {
                "type": "string",
                "description": "Optional AP MAC override used to correlate current AP/radio context."
            }
        },
        "anyOf": [
            {"required": ["client_id"]},
            {"required": ["client_mac"]},
            {"required": ["ip"]}
        ],
        "additionalProperties": false
    })
}

fn hybrid_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "prefer": {
                "type": "string",
                "enum": ["official", "internal"],
                "description": "Select the official or internal backend. Internal is the default."
            },
            "siteId": {
                "type": "string",
                "description": "Official UniFi site UUID. Supplying it without prefer selects the official backend."
            },
            "query": {
                "type": "object",
                "description": "Optional upstream query parameters."
            }
        },
        "additionalProperties": false
    })
}

fn generated_schema(capability: &Capability) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    if let Some(path) = capability.path.as_deref() {
        for parameter in path_parameters(path) {
            properties.insert(parameter.clone(), path_parameter_schema(&parameter));
            required.push(Value::String(parameter));
        }
    }

    properties.insert(
        "query".to_string(),
        json!({
            "type": "object",
            "description": "Optional query parameters forwarded to the UniFi endpoint."
        }),
    );
    if capability.method.as_deref() != Some("GET") {
        properties.insert(
            "body".to_string(),
            json!({
                "type": "object",
                "description": "JSON request body forwarded to the UniFi endpoint."
            }),
        );
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(required));
    }
    schema.insert("additionalProperties".to_string(), Value::Bool(false));
    Value::Object(schema)
}

fn path_parameters(path: &str) -> Vec<String> {
    let mut parameters = Vec::new();
    let mut remaining = path;
    while let Some(start) = remaining.find('{') {
        let after = &remaining[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        let parameter = &after[..end];
        if !parameter.is_empty() {
            parameters.push(parameter.to_string());
        }
        remaining = &after[end + 1..];
    }
    if path.contains("*path") {
        parameters.push("path".to_string());
    }
    parameters.sort();
    parameters.dedup();
    parameters
}

fn path_parameter_schema(name: &str) -> Value {
    json!({
        "description": format!("Required path parameter: {name}."),
        "oneOf": [
            {"type": "string"},
            {"type": "integer"},
            {"type": "boolean"}
        ]
    })
}

fn compatibility_router_definition(capabilities: &[Capability]) -> Value {
    let mut actions = capabilities
        .iter()
        .map(|capability| capability.action.clone())
        .collect::<Vec<_>>();
    actions.push("help".to_string());
    actions.sort();
    actions.dedup();

    json!({
        "name": "unifi",
        "description": "Deprecated compatibility router. Prefer the atomic UniFi MCP tools; this action-dispatched surface will be removed after consumers migrate.",
        "annotations": {
            "deprecated": true
        },
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Legacy operation selector.",
                    "enum": actions
                },
                "params": {
                    "type": "object",
                    "description": "Legacy action-specific parameters."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        }
    })
}

fn source_name(source: ApiSourceFamily) -> &'static str {
    match source {
        ApiSourceFamily::Official => "official",
        ApiSourceFamily::Internal => "internal",
        ApiSourceFamily::Hybrid => "hybrid",
    }
}
