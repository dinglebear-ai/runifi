---
title: "UniFi API Coverage"
created: 2026-07-05
updated: 2026-07-30
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
- Existing live-verified unifi-rmcp actions preserved: clients, devices, wlans, health, alarms, events, sysinfo, me.

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
| `unifi_get_network_health` | internal | `GET /stat/health` | generic internal dispatcher |
| `unifi_list_networks` | internal | `GET /rest/networkconf` | generic internal dispatcher |
| `unifi_list_port_forwards` | internal | `GET /rest/portforward` | generic internal dispatcher |
| `unifi_trigger_rf_scan` | internal | `POST /cmd/devmgr` | admin-authorized generic dispatcher |
| `list_clients` | hybrid | official clients or `clients` | implemented |
| `list_devices` | hybrid | official devices or `devices` | implemented |
| `list_networks` | hybrid | official networks or `unifi_list_networks` | implemented |
| `list_wifi` | hybrid | official WiFi or `wlans` | implemented |
| `get_system_info` | hybrid | official info or `sysinfo` | implemented |

Official endpoint parity means every operation in `data/unifi_official_network_v10_3_58.json` is registered as an action, has a valid path template, has an auth scope, and is either contract-verified or safe-live verified. Contract verification is the CI-safe floor; live probing is an operator action.

The internal runtime surface is model-backed by `data/unifi_internal_endpoint_models.json` and exposes only controller endpoint rows with `runtime=true`. The five upstream-style meta helpers remain accounted in the source inventory, but they are not controller endpoints and are not exposed as runtime actions.

Internal endpoint parity proves action registration and route construction. Full upstream-style tool parity also requires action-specific argument mapping into request bodies and query strings. That request-construction layer is tracked in `docs/superpowers/plans/2026-07-05-wiremock-api-call-tests.md`.

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
