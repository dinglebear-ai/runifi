use unifi_rmcp::mcp::schemas::tool_definitions;

#[test]
fn schema_contains_official_and_internal_actions() {
    let tools = tool_definitions();
    let schema = &tools[0]["inputSchema"]["properties"]["action"]["enum"];
    let actions = schema.as_array().expect("action enum");

    assert!(actions.iter().any(|value| value == "clients"));
    assert!(actions.iter().any(|value| value == "official_list_clients"));
    assert!(actions.iter().any(|value| value == "unifi_list_alarms"));
    assert!(
        actions
            .iter()
            .any(|value| value == "unifi_create_firewall_policy")
    );
}
