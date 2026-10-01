---
title: "UniFi API Coverage"
created: 2026-07-05
updated: 2026-09-26
---

# UniFi API Coverage

## Sources

- Official Network API: `data/unifi_official_network_v10_3_58.json`
- Internal endpoint models: `data/unifi_internal_endpoint_models.json`

## API Families

- `official`: documented Network Integration API under `/proxy/network/integration/v1`.
- `internal`: Network controller APIs under `/proxy/network/api/s/{site}` and `/proxy/network/v2/api/site/{site}`.
- `hybrid`: convenience actions that use internal actions by default and switch to official API when `siteId` or `prefer="official"` is supplied.

## Coverage

- Official Network operations targeted: 78.
- Internal Network reference rows sourced: 180.
- Internal controller endpoint rows exposed at runtime: 175.
- Internal reference meta tools accounted but not exposed as controller endpoints: 5.
- Existing live-verified conveniences preserved as direct tools: clients, devices, wlans, health, alarms, events, sysinfo, me.
- Hybrid direct tools: 5.
- RF composite direct tools: 1 (`get_client_rf_history`).
- Total atomic MCP tools projected from registered capabilities: 267.
- The legacy `unifi(action=..., params=...)` router remains only as deprecated compatibility behavior and is not included in the 267 atomic-tool count.

## Implementation Status

| Action | Family | Endpoint | Status |
|---|---|---|---|
| `official_*` | official | `/proxy/network/integration/v1/...` | implemented by generic dispatcher |
| `clients` | internal | `GET /stat/sta` | preserved |
| `devices` | internal | `GET /stat/device` | preserved |
| `wlans` | internal | `GET /rest/wlanconf` | preserved |
| `health` | internal | `GET /stat/health` | preserved |
| `alarms` | internal | `GET /rest/alarm` | preserved |
| `events` | internal | `GET /rest/event` | preserved |
| `sysinfo` | internal | `GET /stat/sysinfo` | preserved |
| `me` | internal | `GET /proxy/network/api/self` | preserved |
| `unifi_list_alarms` | internal | `POST /v2/system-log/critical` | generic internal dispatcher |
| `unifi_get_client_wifi_details` | internal | `GET /stat/sta` | typed direct tool; filters the requested current client locally |
| `unifi_get_client_stats` | internal | `POST /stat/report/{granularity}.user` | typed direct tool; body carries attrs, client MAC, start, and end |
| `get_client_rf_history` | composite | client report + `/stat/sta` + `/v2/system-log/all` + `/stat/device` | typed direct tool; correlated RF/client timeline |
| `unifi_get_network_health` | internal | `GET /stat/health` | generic internal dispatcher |
| `unifi_list_networks` | internal | `GET /rest/networkconf` | generic internal dispatcher |
| `unifi_list_port_forwards` | internal | `GET /rest/portforward` | generic internal dispatcher |
| `unifi_trigger_rf_scan` | internal | `POST /cmd/devmgr` | admin-authorized generic dispatcher |
| `list_clients` | hybrid | official clients or `clients` | implemented |
| `list_devices` | hybrid | official devices or `devices` | implemented |
| `list_networks` | hybrid | official networks or `unifi_list_networks` | implemented |
| `list_wifi` | hybrid | official WiFi or `wlans` | implemented |
| `get_system_info` | hybrid | official info or `sysinfo` | implemented |

Official endpoint parity means every operation in `data/unifi_official_network_v10_3_58.json` is registered as a capability, has a valid path template, has an auth scope, and is either contract-verified or safe-live verified. Each registered runtime capability is projected as its own MCP tool; atomic dispatch uses the tool name as the capability action, while the deprecated `unifi` router resolves its explicit `action` argument for compatibility. Contract verification is the CI-safe floor; live probing is an operator action.

The internal runtime surface is model-backed by `data/unifi_internal_endpoint_models.json` and exposes only controller endpoint rows with `runtime=true`. The five upstream-style meta helpers remain accounted in the source inventory, but they are not controller endpoints and are not exposed as runtime actions. Generated endpoint tools expose path placeholders directly and generic `query`/`body` fields where applicable; the client-statistics and RF tools use richer typed schemas.

Historical client statistics use `POST /stat/report/{granularity}.user`, not GET. The request body contains the selected RF/statistics attributes, normalized client MAC, and Unix-millisecond `start`/`end` bounds. Supported granularities are `5minutes`, `hourly`, `daily`, and `monthly`. `/stat/session` is hotspot/captive-portal authorization history and is not used as normal WiFi association history; RF history correlates connect/disconnect/roam events from `/v2/system-log/all`. System-log correlation paginates using `total_page_count` (or short-page termination) and reports a warning if its 100-page / 10,000-event safety cap is reached.

Internal endpoint parity proves capability registration and route construction. Full operation-specific schema parity for every generated endpoint is still broader than the current generic path/query/body projection and remains follow-up work.

## Endpoint Verification

Run contract verification without network access:

```bash
cargo run -p xtask -- verify-api-endpoints --mode contract
```

Run live read probes against a controller with:

```bash
UNIFI_URL=https://<gateway> \
UNIFI_API_KEY=<network-api-key> \
UNIFI_SITE=default \
UNIFI_SITE_ID=<official-site-uuid> \
UNIFI_SKIP_TLS_VERIFY=true \
cargo run -p xtask -- verify-api-endpoints --mode safe_live
```

The verifier writes local reports under `target/unifi_verification/`; these reports must not be committed.

Result interpretation:

- `live_ok`: endpoint returned a 2xx response in live mode.
- `contract_ok`: endpoint is registered, path-valid, auth-scoped, and safe by policy in contract mode.
- `requires_fixture`: endpoint needs a concrete object ID or fixture before live probing; it is accounted, not live-verified.
- `unsupported`: reference row is accounted but not exposed as a runtime endpoint.
- `auth_failed`: API key was rejected or lacks permission.
- `server_error`: request failed or controller returned 5xx.
- `skipped`: endpoint was disabled by mode or request budget.
- `budget_exhausted`: live mode ran out of request budget; this fails verification.

`mutating_live` is reserved for disposable or controlled sites. It is never the default.
