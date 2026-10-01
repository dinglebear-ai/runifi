/// Tests for MCP tool dispatch logic via the loopback AppState.
/// These tests do not make real network calls — they verify routing and
/// error handling within the tool shim itself.
use unifi_rmcp::testing::loopback_state;

/// The help action returns a JSON object with a "help" key without hitting
/// the network.
#[tokio::test]
async fn help_action_returns_help_text() {
    let state = loopback_state();
    // Invoke the tool indirectly through the service layer by checking
    // that help resolves without error and contains expected content.
    // We call the internal dispatch via the public service interface.
    // Since help is handled before any network call, this always succeeds.
    let args = serde_json::json!({ "action": "help" });
    // Access via the public testing API — call execute_tool through the
    // documented module path.
    let result = unifi_rmcp::testing::call_tool(&state, "unifi", args).await;
    assert!(result.is_ok(), "help action should succeed: {:?}", result);
    let val = result.unwrap();
    let help_text = val["help"].as_str().unwrap_or("");
    assert!(
        help_text.contains("clients"),
        "help text should mention 'clients'"
    );
    assert!(
        help_text.contains("devices"),
        "help text should mention 'devices'"
    );
}

/// An unknown action produces an error mentioning the action name.
#[tokio::test]
async fn unknown_action_returns_error() {
    let state = loopback_state();
    let args = serde_json::json!({ "action": "nonexistent_action_xyz" });
    let result = unifi_rmcp::testing::call_tool(&state, "unifi", args).await;
    assert!(result.is_err(), "unknown action should return an error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("nonexistent_action_xyz"),
        "error should name the bad action, got: {msg}"
    );
}

/// Missing action argument produces a clear error.
#[tokio::test]
async fn missing_action_returns_error() {
    let state = loopback_state();
    let args = serde_json::json!({});
    let result = unifi_rmcp::testing::call_tool(&state, "unifi", args).await;
    assert!(result.is_err(), "missing action should return an error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("action"),
        "error should mention 'action', got: {msg}"
    );
}

/// Unknown tool name (not "unifi") returns an error.
#[tokio::test]
async fn unknown_tool_name_returns_error() {
    let state = loopback_state();
    let args = serde_json::json!({ "action": "clients" });
    let result = unifi_rmcp::testing::call_tool(&state, "unknown_tool", args).await;
    assert!(result.is_err(), "unknown tool name should return an error");
}

#[test]
fn mcp_auth_scope_comes_from_capability_registry() {
    assert_eq!(unifi_rmcp::mcp::required_scope_for("help"), None);
    assert_eq!(
        unifi_rmcp::mcp::required_scope_for("official_list_clients"),
        Some("unifi:read")
    );
    assert_eq!(
        unifi_rmcp::mcp::required_scope_for("unifi_list_networks"),
        Some("unifi:read")
    );
    assert_eq!(
        unifi_rmcp::mcp::required_scope_for("official_create_network"),
        Some("unifi:admin")
    );
    assert_eq!(
        unifi_rmcp::mcp::required_scope_for("get_client_rf_history"),
        Some("unifi:read")
    );
    assert_eq!(
        unifi_rmcp::mcp::required_scope_for("missing_action"),
        Some("unifi:__deny__")
    );
}

#[tokio::test]
async fn atomic_tool_dispatches_without_action_wrapper() {
    let state = loopback_state();
    let result =
        unifi_rmcp::testing::call_tool(&state, "get_client_rf_history", serde_json::json!({}))
            .await;

    let message = result
        .expect_err("RF history should validate its selector before network I/O")
        .to_string();
    assert!(message.contains("client_mac"));
    assert!(!message.contains("unknown tool"));
}

#[tokio::test]
async fn mutating_actions_require_admin_scope() {
    let rf_scan = unifi_rmcp::capabilities::find_capability("unifi_trigger_rf_scan")
        .expect("rf scan capability");
    assert!(rf_scan.mutating);
    assert_eq!(rf_scan.auth_scope.as_str(), "admin");

    let clients = unifi_rmcp::capabilities::find_capability("clients").expect("clients capability");
    assert_eq!(clients.auth_scope.as_str(), "read");
}
