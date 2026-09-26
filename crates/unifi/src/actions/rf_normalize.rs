use serde_json::{Map, Value, json};

pub(super) fn find_client(value: &Value, selector: &str) -> Option<Value> {
    let selector_lower = selector.to_ascii_lowercase();
    let selector_mac = looks_like_mac(selector).then(|| normalize_mac(selector));
    extract_rows(value).into_iter().find(|client| {
        let mac_match = client
            .get("mac")
            .and_then(Value::as_str)
            .is_some_and(|mac| selector_mac.as_deref() == Some(normalize_mac(mac).as_str()));
        let text_match = ["_id", "ip", "last_ip"]
            .into_iter()
            .filter_map(|key| client.get(key).and_then(Value::as_str))
            .any(|value| value.to_ascii_lowercase() == selector_lower);
        mac_match || text_match
    })
}

pub(super) fn find_device(value: &Value, mac: &str) -> Option<Value> {
    let mac = normalize_mac(mac);
    extract_rows(value).into_iter().find(|device| {
        device
            .get("mac")
            .and_then(Value::as_str)
            .is_some_and(|value| normalize_mac(value) == mac)
    })
}

pub(super) fn normalize_current_client(client: &Value) -> Value {
    let mut out = Map::new();
    for key in [
        "mac",
        "ip",
        "last_ip",
        "name",
        "hostname",
        "signal",
        "noise",
        "satisfaction",
        "tx_rate",
        "rx_rate",
        "tx_retries",
        "wifi_tx_attempts",
        "wifi_tx_dropped",
        "wifi_tx_retries_percentage",
        "channel",
        "channelWidth",
        "channel_width",
        "radio",
        "essid",
        "bssid",
        "ap_mac",
        "roam_count",
        "last_seen",
        "nss",
        "is_11r",
    ] {
        copy_field(client, &mut out, key);
    }
    if let Some(mac) = out.get("mac").and_then(Value::as_str).map(normalize_mac) {
        out.insert("mac".into(), Value::String(mac));
    }
    if let (Some(signal), Some(noise)) = (
        integer_field(client, "signal"),
        integer_field(client, "noise"),
    ) {
        out.insert("snr".into(), json!(signal - noise));
    }
    if !out.contains_key("retry_percentage")
        && let Some(percent) = retry_percentage(client)
    {
        out.insert("retry_percentage".into(), json!(percent));
    }
    if let Some(radio) = client.get("radio").and_then(Value::as_str) {
        out.insert("band".into(), Value::String(band_name(radio).to_string()));
    }
    out.insert(
        "provenance".into(),
        json!({"endpoint": "/stat/sta", "kind": "current_client"}),
    );
    Value::Object(out)
}

pub(super) fn normalize_client_samples(value: &Value, granularity: &str) -> Vec<Value> {
    extract_rows(value)
        .into_iter()
        .map(|sample| {
            let mut out = Map::new();
            out.insert("kind".into(), Value::String("client_sample".into()));
            for key in [
                "time",
                "signal",
                "rssi",
                "tx_rate",
                "rx_rate",
                "satisfaction",
                "anomalies",
                "duration",
                "bytes",
                "tx_bytes",
                "rx_bytes",
                "tx_retries",
                "tx_packets",
                "rx_packets",
                "wifi_tx_attempts",
                "wifi_tx_dropped",
                "radio_protocol_most_common",
                "rx_rate_most_common",
                "x-set-ap_macs",
                "duration_map-ap_duration",
                "6e-signal",
                "na-signal",
                "ng-signal",
            ] {
                copy_field(&sample, &mut out, key);
            }
            if let Some(timestamp) = timestamp_field(&sample, "time") {
                out.insert("timestamp".into(), json!(timestamp));
            }
            if let Some(percent) = retry_percentage(&sample) {
                out.insert("retry_percentage".into(), json!(percent));
            }
            if let Some(band) = sample_band(&sample) {
                out.insert("band".into(), Value::String(band.into()));
            }
            out.insert(
                "provenance".into(),
                json!({
                    "endpoint": format!("/stat/report/{granularity}.user"),
                    "kind": "historical_client_report",
                }),
            );
            Value::Object(out)
        })
        .collect()
}

pub(super) fn normalize_events(value: &Value, mac: &str, ip: Option<&str>) -> Vec<Value> {
    extract_rows(value)
        .into_iter()
        .filter(|event| event_matches_client(event, mac, ip))
        .map(|event| {
            let parameters = event.get("parameters").unwrap_or(&Value::Null);
            json!({
                "kind": "wifi_event",
                "timestamp": timestamp_field(&event, "timestamp"),
                "event": event.get("event").or_else(|| event.get("key")).cloned(),
                "category": event.get("category").cloned(),
                "severity": event.get("severity").cloned(),
                "title": event.get("title_raw").or_else(|| event.get("title")).cloned(),
                "signal": parameter_value(parameters, "SIGNAL_STRENGTH"),
                "channel": parameter_value(parameters, "CHANNEL"),
                "band": parameter_value(parameters, "RADIO_BAND"),
                "reason": parameter_value(parameters, "REASON")
                    .or_else(|| parameter_value(parameters, "DISCONNECT_REASON")),
                "ap": {
                    "mac": nested_parameter(parameters, "DEVICE", "id"),
                    "name": nested_parameter(parameters, "DEVICE", "name"),
                },
                "client": {
                    "mac": nested_parameter(parameters, "CLIENT", "id"),
                    "ip": nested_parameter(parameters, "CLIENT", "ip"),
                    "name": nested_parameter(parameters, "CLIENT", "name"),
                },
                "provenance": {
                    "endpoint": "/v2/system-log/all",
                    "kind": "wifi_event",
                }
            })
        })
        .collect()
}

pub(super) fn normalize_ap_context(device: &Value, client: Option<&Value>) -> Value {
    let channel = client.and_then(|value| integer_field(value, "channel"));
    let radio = client
        .and_then(|value| value.get("radio"))
        .and_then(Value::as_str);

    let radio_stats = device
        .get("radio_table_stats")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find(|item| {
                let channel_match = channel.is_some_and(|expected| {
                    integer_field(item, "channel").is_some_and(|actual| actual == expected)
                });
                let radio_match = radio.is_some_and(|expected| {
                    item.get("radio").and_then(Value::as_str) == Some(expected)
                });
                channel_match || radio_match
            })
        });

    let mut out = Map::new();
    for key in ["mac", "name", "model", "version"] {
        copy_field(device, &mut out, key);
    }
    if let Some(stats) = radio_stats {
        if let Some(value) = stats.get("cu_total").cloned() {
            out.insert("channel_utilization".into(), value);
        }
        if let Some(value) = stats
            .get("cu_other")
            .or_else(|| stats.get("interference"))
            .cloned()
        {
            out.insert("interference".into(), value);
        }
        for key in ["radio", "channel", "channel_width", "num_sta", "tx_retry"] {
            copy_field(stats, &mut out, key);
        }
    }
    out.insert(
        "provenance".into(),
        json!({"endpoint": "/stat/device", "kind": "current_ap_radio"}),
    );
    Value::Object(out)
}

pub(super) fn event_matches_client(event: &Value, mac: &str, ip: Option<&str>) -> bool {
    let normalized_mac = normalize_mac(mac);
    let parameters = event.get("parameters").unwrap_or(&Value::Null);
    let event_mac = nested_parameter(parameters, "CLIENT", "id")
        .or_else(|| event.get("mac").cloned())
        .and_then(|value| value.as_str().map(normalize_mac));
    if event_mac.as_deref() == Some(normalized_mac.as_str()) {
        return true;
    }

    let Some(ip) = ip else {
        return false;
    };
    nested_parameter(parameters, "CLIENT", "ip")
        .or_else(|| nested_parameter(parameters, "IP", "name"))
        .or_else(|| event.get("ip").cloned())
        .and_then(|value| value.as_str().map(str::to_string))
        .is_some_and(|value| value == ip)
}

pub(super) fn extract_rows(value: &Value) -> Vec<Value> {
    if let Some(rows) = value.get("data").and_then(Value::as_array) {
        return rows.clone();
    }
    if let Some(rows) = value.as_array() {
        if rows.len() == 1
            && let Some(data) = rows[0].get("data").and_then(Value::as_array)
        {
            return data.clone();
        }
        return rows.clone();
    }
    Vec::new()
}

fn copy_field(source: &Value, target: &mut Map<String, Value>, key: &str) {
    if let Some(value) = source.get(key) {
        target.insert(key.to_string(), value.clone());
    }
}

fn parameter_value(parameters: &Value, key: &str) -> Option<Value> {
    parameters
        .get(key)
        .and_then(|value| value.get("name").or(Some(value)))
        .cloned()
}

fn nested_parameter(parameters: &Value, key: &str, field: &str) -> Option<Value> {
    parameters
        .get(key)
        .and_then(|value| value.get(field))
        .cloned()
}

fn integer_field(value: &Value, key: &str) -> Option<i64> {
    let value = value.get(key)?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn timestamp_field(value: &Value, key: &str) -> Option<u64> {
    let value = value.get(key)?;
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn retry_percentage(value: &Value) -> Option<f64> {
    if let Some(percent) = value
        .get("wifi_tx_retries_percentage")
        .and_then(Value::as_f64)
    {
        return Some(percent);
    }
    let retries = value.get("tx_retries")?.as_f64()?;
    let attempts = value.get("wifi_tx_attempts")?.as_f64()?;
    if attempts <= 0.0 {
        return None;
    }
    Some((retries / attempts * 10_000.0).round() / 100.0)
}

fn sample_band(sample: &Value) -> Option<&'static str> {
    if sample
        .get("6e-signal")
        .is_some_and(|value| !value.is_null())
    {
        Some("6GHz")
    } else if sample
        .get("na-signal")
        .is_some_and(|value| !value.is_null())
    {
        Some("5GHz")
    } else if sample
        .get("ng-signal")
        .is_some_and(|value| !value.is_null())
    {
        Some("2.4GHz")
    } else {
        None
    }
}

fn band_name(radio: &str) -> &'static str {
    match radio {
        "6e" | "wifi2" => "6GHz",
        "na" | "wifi1" => "5GHz",
        "ng" | "wifi0" => "2.4GHz",
        _ => "unknown",
    }
}

pub(super) fn event_timestamp(value: &Value) -> u64 {
    value
        .get("timestamp")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX)
}

pub(super) fn normalize_mac(value: &str) -> String {
    let cleaned = value.trim().to_ascii_lowercase().replace('-', ":");
    if cleaned.contains(':') {
        return cleaned;
    }
    let hex = cleaned
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    if hex.len() == 12 {
        return (0..6)
            .map(|index| &hex[index * 2..index * 2 + 2])
            .collect::<Vec<_>>()
            .join(":");
    }
    cleaned
}

pub(super) fn looks_like_mac(value: &str) -> bool {
    normalize_mac(value)
        .split(':')
        .collect::<Vec<_>>()
        .as_slice()
        .iter()
        .count()
        == 6
        && normalize_mac(value)
            .split(':')
            .all(|part| part.len() == 2 && part.chars().all(|ch| ch.is_ascii_hexdigit()))
}
