use serde_json::{Value, json};

use unifi::ActionRequest;

use super::AppState;

/// Thin shim — parse args, call service, return Value. No logic here.
pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    match name {
        "unifi" => dispatch(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

async fn dispatch(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action =
        string_arg(&args, "action").ok_or_else(|| anyhow::anyhow!("action is required"))?;
    match action.as_str() {
        "help" => Ok(json!({ "help": HELP_TEXT })),
        _ => {
            let params = args.get("params").cloned().unwrap_or_else(|| json!({}));
            state
                .service
                .execute(ActionRequest { action, params })
                .await
        }
    }
}

fn string_arg(args: &Value, name: &str) -> Option<String> {
    args.get(name).and_then(|v| v.as_str()).map(String::from)
}

const HELP_TEXT: &str = r#"# unifi MCP Tool

Read-only access to UniFi network controllers via REST API.
Set the required `action` argument to select the operation.

## Network
- `clients`   — Connected wireless and wired clients
- `devices`   — Network devices: APs, switches, gateways
- `wlans`     — WiFi network configurations
- `health`    — Site health summary
- `alarms`    — Active alarms and alerts
- `events`    — Recent controller events
- `sysinfo`   — Controller system information
- `me`        — Authenticated user info

## Meta
- `help`      — This documentation
"#;
