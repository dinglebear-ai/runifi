---
title: "UniFi Endpoint Verification"
created: 2026-07-05
updated: 2026-07-30
---

# UniFi Endpoint Verification

`cargo run -p xtask -- verify-api-endpoints --mode contract` accounts for registry, path, auth-scope, and request-policy coverage without network access.

`cargo run -p xtask -- verify-api-endpoints --mode safe_live` additionally probes safe read endpoints against a configured controller.

`cargo run -p xtask -- verify-api-endpoints --mode mutating_live` is reserved for disposable or controlled sites.

Live reports are local artifacts under `target/unifi_verification/` and must not be committed.

Live request budget exhaustion is reported as `budget_exhausted` and fails the verifier. Increase `UNIFI_VERIFY_MAX_REQUESTS` when a live run must probe more endpoints.
