use serde_json::{Value, json};

use unifi::capabilities::all_capabilities;

pub fn tool_definitions() -> Vec<Value> {
    let capabilities = all_capabilities();
    let mut actions = capabilities
        .iter()
        .map(|capability| capability.action.clone())
        .collect::<Vec<_>>();
    actions.push("help".to_string());
    actions.sort();
    actions.dedup();
    let auth_scopes = capabilities
        .iter()
        .map(|capability| {
            json!({
                "action": capability.action.clone(),
                "auth_scope": capability.auth_scope.as_str(),
                "verification_mode": capability.verification_mode.clone(),
            })
        })
        .collect::<Vec<_>>();

    vec![json!({
        "name": "unifi",
        "description": "Query and manage a UniFi network controller via official, internal, and hybrid API actions. Mutating actions require admin authorization.",
        "annotations": {
            "auth_scopes": auth_scopes
        },
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation to perform.",
                    "enum": actions
                },
                "params": {
                    "type": "object",
                    "description": "Action-specific parameters, including path values, query, and body."
                }
            },
            "required": ["action"]
        }
    })]
}
