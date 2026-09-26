use serde_json::Value;
use unifi_rmcp::mcp::schemas::tool_definitions;

fn tool<'a>(tools: &'a [Value], name: &str) -> &'a Value {
    tools
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("missing tool {name}"))
}

#[test]
fn schema_exposes_atomic_tools_and_compatibility_router() {
    let tools = tool_definitions();

    for name in [
        "clients",
        "official_list_clients",
        "unifi_list_alarms",
        "unifi_create_firewall_policy",
        "unifi_get_client_stats",
        "unifi_get_client_wifi_details",
        "get_client_rf_history",
    ] {
        tool(&tools, name);
    }

    let router = tool(&tools, "unifi");
    assert!(
        router["description"]
            .as_str()
            .expect("router description")
            .to_ascii_lowercase()
            .contains("deprecated")
    );
}

#[test]
fn client_stats_atomic_schema_is_explicit() {
    let tools = tool_definitions();
    let schema = &tool(&tools, "unifi_get_client_stats")["inputSchema"];
    let properties = schema["properties"].as_object().expect("properties");
    let required = schema["required"].as_array().expect("required");

    assert!(properties.contains_key("client_id"));
    assert!(properties.contains_key("granularity"));
    assert!(properties.contains_key("duration"));
    assert!(properties.contains_key("start"));
    assert!(properties.contains_key("end"));
    assert!(required.iter().any(|value| value == "client_id"));

    let granularities = properties["granularity"]["enum"]
        .as_array()
        .expect("granularity enum");
    for value in ["5minutes", "hourly", "daily", "monthly"] {
        assert!(
            granularities.iter().any(|entry| entry == value),
            "missing granularity {value}"
        );
    }
    assert_eq!(schema["additionalProperties"], false);
}

#[test]
fn rf_history_atomic_schema_accepts_mac_ip_or_identifier() {
    let tools = tool_definitions();
    let schema = &tool(&tools, "get_client_rf_history")["inputSchema"];
    let properties = schema["properties"].as_object().expect("properties");

    for field in [
        "client_id",
        "client_mac",
        "ip",
        "granularity",
        "duration",
        "start",
        "end",
        "ap_mac",
    ] {
        assert!(properties.contains_key(field), "missing {field}");
    }

    assert!(
        schema["anyOf"]
            .as_array()
            .expect("selector anyOf")
            .iter()
            .any(|entry| entry["required"] == serde_json::json!(["client_mac"]))
    );
}

#[test]
fn official_atomic_schema_exposes_path_parameters_directly() {
    let tools = tool_definitions();
    let schema = &tool(&tools, "official_get_network_details")["inputSchema"];
    let properties = schema["properties"].as_object().expect("properties");
    let required = schema["required"].as_array().expect("required");

    assert!(properties.contains_key("siteId"));
    assert!(properties.contains_key("networkId"));
    assert!(properties.contains_key("query"));
    assert!(
        required.iter().any(|value| value == "siteId")
            && required.iter().any(|value| value == "networkId")
    );
}
