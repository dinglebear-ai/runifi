---
name: unifi
description: >
  Use this skill whenever the user asks about their UniFi network — connected clients, who's
  on the WiFi, which devices are online, access points, switches, gateways, network health,
  site health, active alarms, network events, WiFi configurations (SSIDs), client RF/RSSI history,
  retries/roams, controller sysinfo, or their authenticated UniFi identity. This skill covers the unifi-rmcp MCP server, a Rust bridge
  to official and internal UniFi APIs via X-API-KEY. Legacy convenience actions are read-only;
  mutating actions require explicit admin authorization. Trigger phrases include: "UniFi clients",
  "connected clients", "who's on the network", "UniFi devices", "access points", "APs",
  "UniFi switches", "WiFi networks", "WLAN config", "SSIDs", "network health", "UniFi health",
  "site health", "UniFi alarms", "network alerts", "network events", "UniFi events",
  "sysinfo", "controller version", "UniFi me". Always use this skill rather than guessing
  at curl commands or API paths — the UniFi REST API has several gotchas around path prefixes
  and auth that this skill encodes.
---

# UniFi Skill (unifi-rmcp)

Access to a UniFi network controller via the **unifi-rmcp** MCP server. Data is fetched from the documented Network Integration API or internal controller APIs using X-API-KEY authentication. Mutating actions require MCP admin authorization.

## Quick Reference

Use the **atomic MCP tools directly**. Tool names match registered capabilities, so Code Mode and MCP clients can discover the exact operation and schema without routing through an `action` string.

```text
clients()                                      # who's connected
devices()                                      # APs, switches, gateways
health()                                       # site health summary
events(limit=25)                               # recent controller events
unifi_get_client_wifi_details(client_mac=...) # current client RF snapshot
unifi_get_client_stats(client_id=..., duration="daily")
get_client_rf_history(client_mac=..., duration="daily")
official_list_clients(siteId="<uuid>")
```

The deprecated compatibility router still accepts `unifi(action="...", params={...})` for older consumers. Do not choose it for new calls when the atomic tool exists.

Atomic surface summary:

- `official_*`: 78 documented Network Integration API operations; mutating operations require admin authorization.
- `unifi_*`: 175 model-backed internal controller endpoint operations.
- Legacy conveniences: 8 direct tools including `clients`, `devices`, `wlans`, `health`, `alarms`, `events`, `sysinfo`, and `me`.
- Hybrid actions: 5 direct read tools that use internal APIs by default and official APIs when `siteId` or `prefer="official"` is supplied.
- RF composite: `get_client_rf_history` correlates current RF state, historical client samples, paginated system-log association/roam/disconnect events, and current AP radio context; a warning is returned if the 10,000-event safety cap is reached.
- Total atomic tools: 267, plus the deprecated `unifi` compatibility router.

---

## Tier 1 — Atomic MCP Tools (preferred)

Discover and call the capability-named tool directly. The server advertises per-tool auth scope and an input schema for every atomic operation. Typed schemas are provided for the common conveniences, hybrid tools, and RF/client-statistics operations; generated endpoint tools expose their path placeholders directly plus `query` and, when applicable, `body`.

### Tool Reference

| tool/family | description | notable inputs |
|-------------|-------------|----------------|
| `clients` | Connected wireless and wired clients | none |
| `devices` | Network devices: APs, switches, gateways | none |
| `wlans` | WiFi network configurations | none |
| `health` | Site health summary | none |
| `events` | Recent controller events | optional `limit` |
| `unifi_get_client_wifi_details` | Current connected-client RF snapshot from `/stat/sta` | `client_mac` |
| `unifi_get_client_stats` | Historical client report | `client_id`; optional `granularity`, `duration`, `start`, `end` |
| `get_client_rf_history` | Correlated RF/client timeline | one of `client_id`, `client_mac`, `ip`; optional range controls and `ap_mac` |
| `official_*` | Documented Network Integration API under `/proxy/network/integration/v1` | direct path params such as `siteId`, `networkId`; optional `query`/`body` |
| `unifi_*` | Internal controller-compatible endpoint operations | path params when present; optional `query`/`body` |
| hybrid tools | `list_clients`, `list_devices`, `list_networks`, `list_wifi`, `get_system_info` | internal by default; `siteId` or `prefer="official"` selects official API |

Historical client reports use `POST /stat/report/{granularity}.user` with the requested RF/statistics attributes, client MAC, and millisecond `start`/`end` range in the request body. Supported granularities are `5minutes`, `hourly`, `daily`, and `monthly`.

`/stat/session` is hotspot/captive-portal authorization history, not normal WiFi association history. For connect/disconnect/roam correlation, use `get_client_rf_history`, which reads the system log.

### Response Shape

Internal controller actions usually return: `{"meta": {"rc": "ok"}, "data": [...]}`

Always index into `["data"]` for the actual records.

Official `official_*` actions return the documented Network Integration API response shape for that endpoint. Hybrid actions return the shape of whichever family they resolve to.

**Exception — `me`:** Returns `{"data": {...}}` (object, not array). The `/api/self` endpoint
it calls does not use the `/proxy/network` prefix — this is intentional and unique to this action.

### Example Calls

```text
clients()
# → data[].{hostname, mac, ip, is_wired, essid, sw_port}

devices()
# → data[].{name, model, type, mac, ip, state, state_str}

health()
# → data[].{subsystem, status, num_ap, num_disconnected, num_user, num_guest}

events(limit=25)
# → data[] event records

unifi_get_client_wifi_details(client_mac="aa:bb:cc:dd:ee:ff")
# → normalized current signal/noise/SNR/rates/retries/channel/band/AP fields

unifi_get_client_stats(client_id="aa:bb:cc:dd:ee:ff", duration="daily", granularity="hourly")
# → historical /stat/report/hourly.user response

get_client_rf_history(client_mac="aa:bb:cc:dd:ee:ff", duration="daily")
# → {client, range, current, ap_context, timeline, warnings, session_semantics}

official_list_clients(siteId="<uuid>")
# → documented Network Integration API response
```

For an older caller that only knows the router, `unifi(action="clients")` remains available as deprecated compatibility behavior.

---

## Tier 2 — CLI Binary (fallback when MCP is unavailable)

Binary: `/home/jmagar/workspace/unifi-rmcp/target/release/runifi`

If the binary does not exist, build it first:
```bash
cd /home/jmagar/workspace/unifi-rmcp && cargo build --release
# or run without building:
cargo run --bin runifi -- <command>
```

| command | output |
|---------|--------|
| `runifi clients` | HOSTNAME / MAC / IP / TYPE / SSID or PORT |
| `runifi devices` | NAME / TYPE / MAC / STATE / IP |
| `runifi wlans` | SSID / BAND / VLAN / SECURITY |
| `runifi health` | subsystem status with AP and client counts |
| `runifi alarms` | `[key] message` per alarm |
| `runifi events [--limit N]` | recent controller events; optional limit truncates returned events |
| `runifi sysinfo` | Version, Build, Hostname, Uptime, Timezone |
| `runifi me` | Name, Email, Role, Super admin flag |

All commands accept `--json` for raw JSON output.

```bash
# Examples
runifi clients
runifi devices --json
runifi health
```

---

## Tier 3 — Direct REST API (emergency fallback)

Use when neither MCP nor CLI is available. Requires `UNIFI_URL` and `UNIFI_API_KEY` in environment.

**Auth:** `X-API-KEY` header — only works on UniFi OS consoles (UDM, UDR, UCG, UX, UDW).  
**TLS:** Self-signed certs are normal — always use `-sk` with curl.  
**Site:** Defaults to `default`.

**UDM/UniFi OS paths** (include `/proxy/network` prefix):

```bash
SITE=${UNIFI_SITE:-default}

# Clients
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/stat/sta" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data[] | {hostname, mac, ip, is_wired}'

# Devices
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/stat/device" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data[] | {name, type, mac, ip, state}'

# WLANs
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/rest/wlanconf" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data[] | {name, band, security, enabled}'

# Health
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/stat/health" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data'

# Alarms
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/rest/alarm" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data[] | {key, msg}'

# Events
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/rest/event" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data[]'

# Sysinfo
curl -sk "$UNIFI_URL/proxy/network/api/s/$SITE/stat/sysinfo" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data[0]'

# Me
curl -sk "$UNIFI_URL/proxy/network/api/self" \
  -H "X-API-KEY: $UNIFI_API_KEY" | jq '.data'
```

**Legacy controllers** (`UNIFI_LEGACY=true`, typically port 8443): use the same paths but
omit the `/proxy/network` prefix entirely.

---

## Key Gotchas

1. **`me` has a unique path.** On modern UniFi OS hardware it uses
   `/proxy/network/api/self` rather than the site-scoped `/api/s/{site}` prefix.

2. **`wlans` is configuration, not client counts.** It returns SSID names, band, security
   mode, and VLAN settings. To count clients per SSID, cross-reference `clients` by `essid`.

3. **Wireless vs wired clients.** In `clients` data: `is_wired=false` means wireless — check
   `essid` for the SSID. `is_wired=true` means wired — check `sw_port` for the switch port.

4. **Device state.** In `devices` data: `state==1` means connected. Prefer `state_str` for
   human display; fall back to checking `state==1` when `state_str` is absent.

5. **Self-signed TLS is expected.** The UniFi controller uses a self-signed certificate by
   default. `UNIFI_SKIP_TLS_VERIFY=true` is the default in unifi-rmcp; use `-sk` in curl.

6. **`meta.rc` should be `"ok"`.** If the UniFi API returns an error, `meta.rc` will not be
   `"ok"`. The client raises an HTTP error in this case, so you'll see an anyhow error rather
   than an unexpected data shape.

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `UNIFI_URL` | Controller base URL, e.g. `https://192.168.1.1` | required |
| `UNIFI_API_KEY` | X-API-KEY header value | required |
| `UNIFI_SITE` | Site name | `default` |
| `UNIFI_SKIP_TLS_VERIFY` | Skip TLS certificate check | `true` |
| `UNIFI_LEGACY` | Omit `/proxy/network` prefix (legacy controllers) | `false` |
| `UNIFI_MCP_PORT` | MCP server bind port | `40030` |
| `UNIFI_MCP_TOKEN` | Static bearer token for MCP auth | — |
| `UNIFI_MCP_NO_AUTH` | Disable MCP auth (loopback only) | — |
