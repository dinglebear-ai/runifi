pub mod doctor;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use unifi::UnifiService;
use unifi::{ActionRequest, capabilities::find_capability};

// ── command enum ──────────────────────────────────────────────────────────────

pub enum CliCommand {
    Clients,
    Devices,
    Wlans,
    Health,
    Alarms,
    Events { limit: Option<usize> },
    Sysinfo,
    Me,
    Doctor,
    Action { action: String, params: Value },
}

impl CliCommand {
    pub fn parse(args: &[String]) -> Result<(Self, bool)> {
        let json = args.iter().any(|a| a == "--json");
        let rest: Vec<&str> = args
            .iter()
            .filter(|a| !matches!(a.as_str(), "--json"))
            .map(String::as_str)
            .collect();

        let cmd = match rest.as_slice() {
            ["clients"] => Self::Clients,
            ["devices"] => Self::Devices,
            ["wlans"] => Self::Wlans,
            ["health"] => Self::Health,
            ["alarms"] => Self::Alarms,
            ["events", rest @ ..] => Self::Events {
                limit: parse_limit(rest)?,
            },
            ["sysinfo"] => Self::Sysinfo,
            ["me"] => Self::Me,
            ["doctor"] => Self::Doctor,
            [action, ..] if find_capability(action).is_some() => Self::Action {
                action: (*action).to_string(),
                params: parse_params(&rest)?,
            },
            other => bail!(
                "unknown command: {}\n\nRun `runifi --help` for usage.",
                other.join(" ")
            ),
        };
        Ok((cmd, json))
    }
}

fn parse_params(args: &[&str]) -> Result<Value> {
    let mut params = json!({});
    let mut idx = 1;
    while idx < args.len() {
        match args[idx] {
            "--param" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| anyhow::anyhow!("--param requires key=value"))?;
                let (key, raw) = value
                    .split_once('=')
                    .ok_or_else(|| anyhow::anyhow!("--param requires key=value"))?;
                merge_param(&mut params, key, Value::String(raw.to_string()));
                idx += 2;
            }
            "--body-json" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| anyhow::anyhow!("--body-json requires a JSON object"))?;
                let body: Value = serde_json::from_str(value)?;
                if !body.is_object() {
                    bail!("--body-json requires a JSON object");
                }
                merge_param(&mut params, "body", body);
                idx += 2;
            }
            "--limit" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| anyhow::anyhow!("--limit requires a value"))?;
                merge_param(&mut params, "limit", json!(value.parse::<usize>()?));
                idx += 2;
            }
            other => bail!("unknown argument for generated action: {other}"),
        }
    }
    Ok(params)
}

fn parse_limit(args: &[&str]) -> Result<Option<usize>> {
    match args {
        [] => Ok(None),
        ["--limit", value] => Ok(Some(value.parse::<usize>()?)),
        [other, ..] => bail!("unknown argument for events: {other}"),
    }
}

fn merge_param(params: &mut Value, key: &str, value: Value) {
    if let Some(object) = params.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

// ── dispatch ──────────────────────────────────────────────────────────────────

pub async fn run(service: &UnifiService, cmd: CliCommand, json: bool) -> Result<()> {
    let (label, data) = match cmd {
        CliCommand::Clients => ("clients".to_string(), service.clients().await?),
        CliCommand::Devices => ("devices".to_string(), service.devices().await?),
        CliCommand::Wlans => ("wlans".to_string(), service.wlans().await?),
        CliCommand::Health => ("health".to_string(), service.health().await?),
        CliCommand::Alarms => ("alarms".to_string(), service.alarms().await?),
        CliCommand::Events { limit } => ("events".to_string(), service.events(limit).await?),
        CliCommand::Sysinfo => ("sysinfo".to_string(), service.sysinfo().await?),
        CliCommand::Me => ("me".to_string(), service.me().await?),
        CliCommand::Action { action, params } => {
            let data = service
                .execute(ActionRequest {
                    action: action.clone(),
                    params,
                })
                .await?;
            (action, data)
        }
        CliCommand::Doctor => {
            // Doctor is intercepted in main.rs before service construction
            unreachable!("Doctor is dispatched directly in main.rs")
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        print_human(&label, &data);
    }
    Ok(())
}

// ── human-readable output ─────────────────────────────────────────────────────

fn print_human(cmd: &str, data: &Value) {
    match cmd {
        "clients" => fmt_clients(data),
        "devices" => fmt_devices(data),
        "wlans" => fmt_wlans(data),
        "health" => fmt_health(data),
        "alarms" => fmt_alarms(data),
        "events" => println!("{}", serde_json::to_string_pretty(data).unwrap_or_default()),
        "sysinfo" => fmt_sysinfo(data),
        "me" => fmt_me(data),
        _ => println!("{}", serde_json::to_string_pretty(data).unwrap_or_default()),
    }
}

// ── formatters ────────────────────────────────────────────────────────────────

fn fmt_clients(data: &Value) {
    let clients = match data["data"].as_array() {
        Some(c) => c,
        None => {
            println!("No clients found (or unexpected response shape).");
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    if clients.is_empty() {
        println!("No connected clients.");
        return;
    }
    println!(
        "{:<20} {:<18} {:<16} {:<10} SSID/PORT",
        "HOSTNAME", "MAC", "IP", "TYPE"
    );
    for c in clients {
        let hostname = str_val_or(&c["hostname"], str_val_or(&c["name"], "--"));
        let mac = str_val_or(&c["mac"], "--");
        let ip = str_val_or(&c["ip"], "--");
        let is_wireless = c["is_wired"].as_bool() != Some(true);
        let kind = if is_wireless { "wireless" } else { "wired" };
        let network = if is_wireless {
            str_val_or(&c["essid"], "--")
        } else {
            str_val_or(&c["sw_port"], "--")
        };
        println!(
            "{:<20} {:<18} {:<16} {:<10} {}",
            hostname, mac, ip, kind, network
        );
    }
    println!("\n{} client(s)", clients.len());
}

fn fmt_devices(data: &Value) {
    let devices = match data["data"].as_array() {
        Some(d) => d,
        None => {
            println!("No devices found.");
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    if devices.is_empty() {
        println!("No network devices.");
        return;
    }
    println!(
        "{:<24} {:<12} {:<18} {:<10} IP",
        "NAME", "TYPE", "MAC", "STATE"
    );
    for d in devices {
        println!(
            "{:<24} {:<12} {:<18} {:<10} {}",
            str_val_or(&d["name"], str_val_or(&d["model"], "--")),
            str_val_or(&d["type"], "--"),
            str_val_or(&d["mac"], "--"),
            str_val_or(
                &d["state_str"],
                if d["state"].as_i64() == Some(1) {
                    "connected"
                } else {
                    "--"
                }
            ),
            str_val_or(&d["ip"], "--"),
        );
    }
    println!("\n{} device(s)", devices.len());
}

fn fmt_wlans(data: &Value) {
    let wlans = match data["data"].as_array() {
        Some(w) => w,
        None => {
            println!("No WLANs found.");
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    if wlans.is_empty() {
        println!("No WiFi networks configured.");
        return;
    }
    println!("{:<32} {:<8} {:<6} SECURITY", "SSID", "BAND", "VLAN");
    for w in wlans {
        let enabled = w["enabled"].as_bool().unwrap_or(false);
        let ssid = format!(
            "{}{}",
            str_val_or(&w["name"], "--"),
            if enabled { "" } else { " [disabled]" }
        );
        println!(
            "{:<32} {:<8} {:<6} {}",
            ssid,
            str_val_or(&w["band"], "--"),
            w["vlan_enabled"]
                .as_bool()
                .map(|_| w["vlanid"].as_i64().unwrap_or(0).to_string())
                .unwrap_or_else(|| "--".into()),
            str_val_or(&w["security"], "--"),
        );
    }
}

fn fmt_health(data: &Value) {
    let items = match data["data"].as_array() {
        Some(h) => h,
        None => {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    for item in items {
        let subsystem = str_val_or(&item["subsystem"], "?");
        let status = str_val_or(&item["status"], "?");
        println!("{:<20} {}", subsystem, status);
        if let Some(num_ap) = item["num_ap"].as_i64() {
            println!(
                "  APs: {num_ap} adopted  {} disconnected",
                item["num_disconnected"].as_i64().unwrap_or(0)
            );
        }
        if let Some(num_sta) = item["num_user"].as_i64() {
            println!(
                "  Clients: {num_sta} users  {} guests",
                item["num_guest"].as_i64().unwrap_or(0)
            );
        }
    }
}

fn fmt_alarms(data: &Value) {
    let alarms = match data["data"].as_array() {
        Some(a) => a,
        None => {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    if alarms.is_empty() {
        println!("No active alarms.");
        return;
    }
    for a in alarms {
        let key = str_val_or(&a["key"], "?");
        let msg = str_val_or(&a["msg"], str_val_or(&a["message"], "--"));
        println!("[{}] {}", key, msg);
    }
    println!("\n{} alarm(s)", alarms.len());
}

fn fmt_sysinfo(data: &Value) {
    let info = match data["data"].as_array().and_then(|a| a.first()) {
        Some(i) => i,
        None => {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    println!("Version:    {}", str_val_or(&info["version"], "--"));
    println!("Build:      {}", str_val_or(&info["build"], "--"));
    println!("Hostname:   {}", str_val_or(&info["hostname"], "--"));
    if let Some(uptime) = info["uptime"].as_i64() {
        let h = uptime / 3600;
        let m = (uptime % 3600) / 60;
        println!("Uptime:     {h}h {m}m");
    }
    println!("Timezone:   {}", str_val_or(&info["timezone"], "--"));
}

fn fmt_me(data: &Value) {
    // /api/self returns {"data": [{...}]} or similar
    let user = data.get("data").and_then(|d| {
        if d.is_array() {
            d.as_array()?.first()
        } else {
            Some(d)
        }
    });
    match user {
        Some(u) => {
            println!("Name:  {}", str_val_or(&u["name"], "--"));
            println!("Email: {}", str_val_or(&u["email"], "--"));
            println!("Role:  {}", str_val_or(&u["role"], "--"));
            if let Some(is_super) = u["is_super_admin"].as_bool() {
                println!("Super: {}", if is_super { "yes" } else { "no" });
            }
        }
        None => {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
        }
    }
}

// ── format helpers ────────────────────────────────────────────────────────────

fn str_val_or<'a>(v: &'a Value, fallback: &'a str) -> &'a str {
    v.as_str().unwrap_or(fallback)
}
