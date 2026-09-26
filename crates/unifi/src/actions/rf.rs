use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::{Value, json};

use super::rf_normalize::{
    event_matches_client, event_timestamp, extract_rows, find_client, find_device, looks_like_mac,
    normalize_ap_context, normalize_client_samples, normalize_current_client, normalize_events,
    normalize_mac,
};

use crate::{UnifiConfig, api::internal::InternalNetworkApi, http};

const CLIENT_REPORT_ATTRS: &[&str] = &[
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
];

const VALID_GRANULARITIES: &[&str] = &["5minutes", "hourly", "daily", "monthly"];
const EVENT_PAGE_SIZE: u64 = 100;
const MAX_EVENT_PAGES: u64 = 100;

pub async fn execute_client_stats(cfg: &UnifiConfig, params: &Value) -> Result<Value> {
    let selector = params
        .get("client_id")
        .or_else(|| params.get("client_mac"))
        .or_else(|| params.get("ip"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("client_id is required"))?;

    let range = TimeRange::from_params(params, DurationPreset::Hourly)?;
    let granularity = resolve_granularity(params, &range, Some("hourly"))?;
    let mac = resolve_client_mac(cfg, selector).await?;

    client_report(cfg, &mac, granularity, &range).await
}

pub async fn execute_client_wifi_details(cfg: &UnifiConfig, params: &Value) -> Result<Value> {
    let selector = params
        .get("client_mac")
        .or_else(|| params.get("client_id"))
        .or_else(|| params.get("ip"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("client_mac is required"))?;

    let clients = get_v1(cfg, "stat/sta").await?;
    let client = find_client(&clients, selector)
        .ok_or_else(|| anyhow::anyhow!("client {selector} is not currently connected"))?;
    Ok(normalize_current_client(&client))
}

pub async fn execute_client_rf_history(cfg: &UnifiConfig, params: &Value) -> Result<Value> {
    let selector = params
        .get("client_mac")
        .or_else(|| params.get("client_id"))
        .or_else(|| params.get("ip"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("one of client_mac, client_id, or ip is required"))?;

    let range = TimeRange::from_params(params, DurationPreset::Daily)?;
    let granularity = resolve_granularity(params, &range, None)?;

    let current_clients = get_v1(cfg, "stat/sta").await?;
    let current_client = find_client(&current_clients, selector);
    let mac = match current_client
        .as_ref()
        .and_then(|client| client.get("mac"))
        .and_then(Value::as_str)
    {
        Some(mac) => normalize_mac(mac),
        None if looks_like_mac(selector) => normalize_mac(selector),
        None => resolve_client_mac(cfg, selector).await?,
    };

    let current = current_client.as_ref().map(normalize_current_client);
    let ip = current_client
        .as_ref()
        .and_then(|client| client.get("ip").or_else(|| client.get("last_ip")))
        .and_then(Value::as_str)
        .map(str::to_string);
    let client_name = current_client
        .as_ref()
        .and_then(|client| client.get("name").or_else(|| client.get("hostname")))
        .and_then(Value::as_str)
        .map(str::to_string);
    let current_ap_mac = current_client
        .as_ref()
        .and_then(|client| client.get("ap_mac"))
        .and_then(Value::as_str)
        .map(normalize_mac);
    let requested_ap_mac = params
        .get("ap_mac")
        .and_then(Value::as_str)
        .map(normalize_mac);
    let ap_mac = requested_ap_mac.or(current_ap_mac);

    let mut warnings = Vec::new();
    let history = client_report(cfg, &mac, granularity, &range).await?;
    let mut timeline = normalize_client_samples(&history, granularity);

    match client_events(cfg, &mac, ip.as_deref(), &range).await {
        Ok((events, truncated)) => {
            timeline.extend(normalize_events(&events, &mac, ip.as_deref()));
            if truncated {
                warnings.push(format!(
                    "event correlation truncated after {} system-log events; narrow the time range for complete event history",
                    EVENT_PAGE_SIZE * MAX_EVENT_PAGES
                ));
            }
        }
        Err(error) => warnings.push(format!("event correlation unavailable: {error}")),
    }
    timeline.sort_by_key(event_timestamp);

    let ap_context = match ap_mac.as_deref() {
        Some(ap_mac) => match get_v1(cfg, "stat/device").await {
            Ok(devices) => find_device(&devices, ap_mac)
                .map(|device| normalize_ap_context(&device, current_client.as_ref())),
            Err(error) => {
                warnings.push(format!("AP context unavailable: {error}"));
                None
            }
        },
        None => None,
    };

    Ok(json!({
        "client": {
            "mac": mac,
            "ip": ip,
            "name": client_name,
        },
        "range": {
            "start": range.start,
            "end": range.end,
            "granularity": granularity,
        },
        "current": current,
        "ap_context": ap_context,
        "timeline": timeline,
        "warnings": warnings,
        "session_semantics": "/stat/session is hotspot/captive-portal authorization history; Wi-Fi association and roaming events come from the system log.",
    }))
}

async fn client_report(
    cfg: &UnifiConfig,
    mac: &str,
    granularity: &str,
    range: &TimeRange,
) -> Result<Value> {
    let api = InternalNetworkApi::new(&cfg.url, &cfg.site, cfg.legacy);
    let path = api.v1_site_path(&format!("stat/report/{granularity}.user"));
    let body = json!({
        "attrs": CLIENT_REPORT_ATTRS,
        "mac": normalize_mac(mac),
        "start": range.start,
        "end": range.end,
    });
    http::request_json(cfg, Method::POST, &path, None, Some(&body)).await
}

async fn client_events(
    cfg: &UnifiConfig,
    mac: &str,
    ip: Option<&str>,
    range: &TimeRange,
) -> Result<(Value, bool)> {
    let api = InternalNetworkApi::new(&cfg.url, &cfg.site, cfg.legacy);
    let path = api.v2_site_path("system-log/all");
    let mut page_number = 0_u64;
    let mut matched = Vec::new();

    loop {
        let body = json!({
            "timestampFrom": range.start,
            "timestampTo": range.end,
            "severities": ["LOW", "MEDIUM", "HIGH", "VERY_HIGH"],
            "categories": ["CLIENT_DEVICES", "UNIFI_DEVICES"],
            "type": "GENERAL",
            "pageNumber": page_number,
            "pageSize": EVENT_PAGE_SIZE,
            "searchText": "",
        });
        let value = http::request_json(cfg, Method::POST, &path, None, Some(&body)).await?;
        let rows = extract_rows(&value);
        let page_len = rows.len() as u64;
        matched.extend(
            rows.into_iter()
                .filter(|event| event_matches_client(event, mac, ip)),
        );

        let total_pages = value
            .get("total_page_count")
            .or_else(|| value.get("totalPageCount"))
            .and_then(Value::as_u64);
        let last_page = total_pages.is_some_and(|total| page_number.saturating_add(1) >= total)
            || (total_pages.is_none() && page_len < EVENT_PAGE_SIZE);
        if last_page {
            return Ok((json!({ "data": matched }), false));
        }

        page_number = page_number.saturating_add(1);
        if page_number >= MAX_EVENT_PAGES {
            return Ok((json!({ "data": matched }), true));
        }
    }
}

async fn resolve_client_mac(cfg: &UnifiConfig, selector: &str) -> Result<String> {
    if looks_like_mac(selector) {
        return Ok(normalize_mac(selector));
    }

    let clients = get_v1(cfg, "stat/sta").await?;
    if let Some(mac) = find_client(&clients, selector)
        .as_ref()
        .and_then(|client| client.get("mac"))
        .and_then(Value::as_str)
    {
        return Ok(normalize_mac(mac));
    }

    let known_clients = get_v1(cfg, "rest/user").await?;
    if let Some(mac) = find_client(&known_clients, selector)
        .as_ref()
        .and_then(|client| client.get("mac"))
        .and_then(Value::as_str)
    {
        return Ok(normalize_mac(mac));
    }

    bail!("unable to resolve client selector {selector} to a MAC address")
}

async fn get_v1(cfg: &UnifiConfig, suffix: &str) -> Result<Value> {
    let api = InternalNetworkApi::new(&cfg.url, &cfg.site, cfg.legacy);
    let path = api.v1_site_path(suffix);
    http::request_json(cfg, Method::GET, &path, None, None).await
}

fn resolve_granularity<'a>(
    params: &'a Value,
    range: &TimeRange,
    default: Option<&'a str>,
) -> Result<&'a str> {
    let granularity = params
        .get("granularity")
        .and_then(Value::as_str)
        .or(default)
        .unwrap_or_else(|| {
            let duration = range.end.saturating_sub(range.start);
            if duration <= 12 * 60 * 60 * 1000 {
                "5minutes"
            } else if duration <= 7 * 24 * 60 * 60 * 1000 {
                "hourly"
            } else if duration <= 180 * 24 * 60 * 60 * 1000 {
                "daily"
            } else {
                "monthly"
            }
        });
    if VALID_GRANULARITIES.contains(&granularity) {
        Ok(granularity)
    } else {
        bail!(
            "invalid granularity {granularity}; expected one of: {}",
            VALID_GRANULARITIES.join(", ")
        )
    }
}

#[derive(Clone, Copy)]
enum DurationPreset {
    Hourly,
    Daily,
}

impl DurationPreset {
    fn millis(self) -> u64 {
        match self {
            Self::Hourly => 60 * 60 * 1000,
            Self::Daily => 24 * 60 * 60 * 1000,
        }
    }
}

struct TimeRange {
    start: u64,
    end: u64,
}

impl TimeRange {
    fn from_params(params: &Value, default: DurationPreset) -> Result<Self> {
        let end = params
            .get("end")
            .and_then(timestamp_value)
            .unwrap_or_else(now_millis);
        let duration = params
            .get("duration")
            .and_then(Value::as_str)
            .map(duration_millis)
            .transpose()?
            .unwrap_or_else(|| default.millis());
        let start = params
            .get("start")
            .and_then(timestamp_value)
            .unwrap_or_else(|| end.saturating_sub(duration));

        if start > end {
            bail!("start must be less than or equal to end");
        }
        Ok(Self { start, end })
    }
}

fn timestamp_value(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn duration_millis(value: &str) -> Result<u64> {
    match value {
        "hourly" => Ok(60 * 60 * 1000),
        "daily" => Ok(24 * 60 * 60 * 1000),
        "weekly" => Ok(7 * 24 * 60 * 60 * 1000),
        "monthly" => Ok(30 * 24 * 60 * 60 * 1000),
        _ => bail!("invalid duration {value}; expected hourly, daily, weekly, or monthly"),
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
