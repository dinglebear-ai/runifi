---
title: "UniFi Endpoint Verification"
created: 2026-07-05
updated: 2026-09-26
---

# UniFi Endpoint Verification

`cargo run -p xtask -- verify-api-endpoints --mode contract` accounts for registry, path, auth-scope, and request-policy coverage without network access. The MCP layer projects each registered runtime capability as an atomic tool and retains the single `unifi(action=..., params=...)` entry point only as deprecated compatibility behavior.

The RF/client-statistics request contract is intentionally explicit: `unifi_get_client_stats` sends `POST /stat/report/{granularity}.user` with `attrs`, normalized `mac`, and Unix-millisecond `start`/`end` values in the request body. `unifi_get_client_wifi_details` reads the `/stat/sta` collection and filters the requested client locally. `get_client_rf_history` additionally correlates historical samples, system-log client events, and current AP radio context.

`cargo run -p xtask -- verify-api-endpoints --mode safe_live` additionally probes safe read endpoints against a configured controller.

`cargo run -p xtask -- verify-api-endpoints --mode mutating_live` is reserved for disposable or controlled sites.

Live reports are local artifacts under `target/unifi_verification/` and must not be committed.

Live request budget exhaustion is reported as `budget_exhausted` and fails the verifier. Increase `UNIFI_VERIFY_MAX_REQUESTS` when a live run must probe more endpoints.
