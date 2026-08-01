# unifi-rmcp

MCP server and CLI for UniFi Network controllers: clients, devices, WLANs,
firewall, and health over stdio or streamable HTTP, with auth.

It exposes one MCP tool, `unifi`, plus the `runifi` CLI. Agents can inspect
clients, devices, WiFi networks, health, alarms, events, controller sysinfo, and
authenticated identity, and can use generated `official_*` / `unifi_*` actions
when their MCP auth scope permits it.

**30-second path:** set `UNIFI_URL` and `UNIFI_API_KEY`, then run
`npx -y @dinglebear/runifi-mcp health --json` -> start loopback HTTP with
`UNIFI_MCP_HOST=127.0.0.1 npx -y @dinglebear/runifi-mcp serve` -> call `tools/call` with
`{"action":"health"}`.

**Status:** operational RMCP upstream-client server. The preserved convenience
actions are read-oriented; generated mutating actions require `unifi:admin`
authorization. HTTP MCP supports loopback dev mode, static bearer tokens, and
Google OAuth through `lab-auth`.

**Not for:** replacing the UniFi console, storing controller credentials for
callers, bypassing UniFi permissions, generic HTTP proxying, multi-tenant
isolation, or passing UniFi API keys through MCP tool arguments.

## Contents

- [Naming](#naming)
- [Capabilities And Boundaries](#capabilities-and-boundaries)
- [Install](#install)
- [Quickstart](#quickstart)
- [Client Configuration](#client-configuration)
- [Runtime Surfaces](#runtime-surfaces)
- [MCP Tool Reference](#mcp-tool-reference)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Safety And Trust Model](#safety-and-trust-model)
- [Architecture](#architecture)
- [Distribution Contract](#distribution-contract)
- [Development](#development)
- [Verification](#verification)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Related Servers](#related-servers)
- [Documentation](#documentation)
- [License](#license)

## Naming

| Surface | This repo |
|---|---|
| Repository | `unifi-rmcp` |
| Rust crate | `unifi-rmcp` |
| Binary / CLI | `runifi` |
| npm package | `@dinglebear/runifi-mcp` |
| npm binary aliases | `unifi-rmcp`, `runifi` |
| MCP tool | `unifi` |
| Config home | `~/.unifi-rmcp` on hosts, `/data` in containers |
| Env prefixes | `UNIFI_*`, `UNIFI_MCP_*`, `UNIFI_RMCP_*` for npm launcher controls |

The repo and npm package use the RMCP family name, while the shipped binary uses
the short Rust CLI name `runifi`.

## Capabilities And Boundaries

- Read connected wireless and wired clients, network devices, WLAN configs,
  site health, active alarms, recent events, controller sysinfo, and current
  authenticated user.
- Dispatch generated `official_*` actions for documented Network Integration API
  endpoints.
- Dispatch model-backed `unifi_*` internal controller actions and hybrid aliases
  such as `list_clients`, `list_devices`, `list_networks`, `list_wifi`, and
  `get_system_info`.
- Enforce `unifi:read` for read actions and `unifi:admin` for mutating actions
  in mounted HTTP MCP mode.
- Provide setup, doctor, and endpoint-verification commands for local runtime
  checks.

| This repo owns | UniFi owns | Explicitly out of scope |
|---|---|---|
| MCP/CLI projection, action registry, request validation, HTTP MCP auth policy, response shaping, generated action dispatch, setup checks, and endpoint verification. | Controller state, site/device/client data, UniFi users, API key issuance, upstream authorization, and actual network mutations. | Replacing the controller UI, credential brokerage, arbitrary HTTP proxying, long-lived polling, policy-as-code, multi-tenant sandboxing, and local gateway provisioning. |

## Install

| Path | Command | Best for | Notes |
|---|---|---|---|
| npm / npx | `npx -y @dinglebear/runifi-mcp --help` | Local MCP clients and quick trials. | Downloads the matching `runifi` binary from GitHub Releases. |
| Release installer | `curl -fsSL https://raw.githubusercontent.com/dinglebear-ai/runifi/main/scripts/install.sh \| bash` | Host installs without Node. | Installs `runifi` for the current Linux host. |
| Docker / Compose | `docker compose up -d` | Shared HTTP MCP deployments. | Reads `.env` and exposes container port `40030`. |
| Build from source | `cargo build --release` | Development and audits. | Produces `target/release/runifi`. |
| Plugin | `claude plugin install plugins/unifi` | Claude Code local plugin setup from this checkout. | Ships the skill and `.mcp.json`. No session hooks — plugin settings reach the server through `.mcp.json`'s `${user_config.*}` env block, so no manual setup step is needed. |

### npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y @dinglebear/runifi-mcp --help
npx -y @dinglebear/runifi-mcp mcp
npx -y @dinglebear/runifi-mcp health --json
```

The npm package downloads `runifi` during `postinstall`. Override download
behavior only when testing packaging:

| Variable | Purpose |
|---|---|
| `UNIFI_RMCP_SKIP_DOWNLOAD=1` | Skip postinstall binary download. |
| `UNIFI_RMCP_VERSION` or `UNIFI_RMCP_BINARY_VERSION` | Select the GitHub Release tag. |
| `UNIFI_RMCP_REPO` | Select the GitHub repo used for release downloads. |
| `UNIFI_RMCP_RELEASE_BASE_URL` | Select a custom release base URL. |

### Build From Source

```bash
git clone git@github.com:dinglebear-ai/runifi.git
cd runifi
cargo build --release
./target/release/runifi --help
```

Minimum supported Rust version: 1.86. The Cargo workspace has three members:
the root `unifi-rmcp` package, `crates/unifi` (reusable core), and `xtask`
(dev tooling). All members inherit Rust edition 2024 and MSRV 1.97.1 from the
workspace root.

## Quickstart

### 1. Create A UniFi API Key

In UniFi OS, go to Settings -> Admins & Users -> API Keys, create a key, and
copy it into `UNIFI_API_KEY`.

### 2. Configure The Controller

```bash
export UNIFI_URL=https://unifi.local
export UNIFI_API_KEY=...
export UNIFI_SITE=default
export UNIFI_SKIP_TLS_VERIFY=true
```

Set `UNIFI_LEGACY=true` only for older non-UDM controllers that do not use the
`/proxy/network` path prefix.

### 3. Run A Safe CLI Call

```bash
npx -y @dinglebear/runifi-mcp health --json
```

### 4. Start Loopback HTTP MCP

```bash
UNIFI_MCP_HOST=127.0.0.1 npx -y @dinglebear/runifi-mcp serve
```

In another shell:

```bash
curl -sf http://127.0.0.1:40030/health
```

### 5. Make A First MCP Call

```bash
curl -s -X POST http://127.0.0.1:40030/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"unifi","arguments":{"action":"health"}}}'
```

## Client Configuration

### Claude Code Stdio

```json
{
  "mcpServers": {
    "unifi": {
      "command": "npx",
      "args": ["-y", "unifi-rmcp", "mcp"],
      "env": {
        "UNIFI_URL": "https://unifi.local",
        "UNIFI_API_KEY": "...",
        "UNIFI_SITE": "default",
        "UNIFI_SKIP_TLS_VERIFY": "true"
      }
    }
  }
}
```

### Claude Code HTTP

```json
{
  "mcpServers": {
    "unifi": {
      "type": "http",
      "url": "http://127.0.0.1:40030/mcp",
      "headers": {
        "Authorization": "Bearer ${UNIFI_MCP_TOKEN}"
      }
    }
  }
}
```

### Codex / Labby Gateway

Register UniFi through Labby as an HTTP upstream when sharing one long-running
server, or run it directly as stdio for local-only use.

```toml
[mcp_servers.unifi]
command = "npx"
args = ["-y", "unifi-rmcp", "mcp"]
```

### Generic MCP JSON

```json
{
  "command": "runifi",
  "args": ["mcp"],
  "env": {
    "UNIFI_URL": "https://unifi.local",
    "UNIFI_API_KEY": "..."
  }
}
```

Do not put `UNIFI_API_KEY`, OAuth secrets, passwords, SSH keys, or upstream
bearer tokens in MCP tool arguments. Use env, config files, or the MCP client's
secret storage. MCP callers never provide credentials, tokens, keys, or secrets
as action arguments.

## Runtime Surfaces

| Surface | Status | Entry point | Purpose |
|---|---:|---|---|
| MCP stdio | Supported | `runifi mcp`, `npx -y @dinglebear/runifi-mcp mcp` | Local child-process MCP clients. |
| MCP HTTP | Supported | `runifi serve`, `POST /mcp` | Streamable HTTP MCP for local or shared server deployments. |
| CLI | Supported | `runifi <command>` | Scriptable parity and debugging. |
| Prompts | Supported | `network_summary` | Agent prompt for UniFi status summaries. |
| Resource | Supported | MCP tool schema resource | Client-side schema discovery. |
| REST API | Not shipped | N/A | UniFi already owns the REST APIs. |
| Web UI | Not shipped | N/A | UniFi already owns the controller UI. |

## MCP Tool Reference

One MCP tool is exposed: `unifi`. Pass the required `action` argument to select
the operation. The action enum is generated from the inventories in `data/`, so
the surface is large:

| Family | Actions | Mutating |
|---|---:|---:|
| `official_*` | 78 | 36 |
| `unifi_*` | 175 | 87 |
| Preserved convenience | 8 | 0 |
| Hybrid aliases | 5 | 0 |
| `help` | 1 | 0 |
| **Total** | **267** | **123** |

Counts are derived from `data/unifi_official_network_v10_3_58.json` and the
`runtime: true` entries of `data/unifi_internal_endpoint_models.json`.
`crates/unifi/src/capabilities.rs` is the source of truth at runtime.

### Preserved Convenience Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `clients` | Connected wireless and wired clients. | none | none |
| `devices` | Network devices: APs, switches, gateways. | none | none |
| `wlans` | WiFi network configurations. | none | none |
| `health` | Site health summary. | none | none |
| `alarms` | Active alarms and alerts. | none | none |
| `events` | Recent controller events. | none | `limit` |
| `sysinfo` | Controller system information. | none | none |
| `me` | Authenticated UniFi identity. | none | none |
| `help` | Built-in action documentation. | none | none |

### Generated Action Families

| Family | Description | Scope behavior |
|---|---|---|
| `official_*` | Documented Network Integration API operations under `/proxy/network/integration/v1`. | Read operations require `unifi:read`; mutating operations require `unifi:admin`. |
| `unifi_*` | Model-backed internal controller-compatible actions under `/proxy/network/api/s/{site}` and `/proxy/network/v2/api/site/{site}`. | Read operations require `unifi:read`; mutating operations require `unifi:admin`. |
| Hybrid aliases | `list_clients`, `list_devices`, `list_networks`, `list_wifi`, `get_system_info`. | Use internal actions by default, or official API when `siteId` or `params.prefer="official"` is supplied. |

Endpoint coverage is tracked in `docs/unifi_api_coverage.md`; contract and safe
live verification are documented in `docs/unifi_endpoint_verification.md`.

## CLI Reference

The binary calls the same service layer as the MCP tool:

```bash
runifi clients [--json]
runifi devices [--json]
runifi wlans [--json]
runifi health [--json]
runifi alarms [--json]
runifi events [--limit N] [--json]
runifi sysinfo [--json]
runifi me [--json]
runifi official_list_clients --param siteId=<uuid> --json
runifi official_create_network --param siteId=<uuid> --body-json '{"name":"IoT"}' --json
runifi list_clients --param siteId=<uuid> --json
runifi doctor [--json]
runifi setup check [--json]
runifi setup repair [--json]
runifi setup install [--json]
runifi setup plugin-hook [--no-repair] [--json]
```

Generated actions accept `--param k=v`, `--body-json JSON`, and `--json`.

`setup plugin-hook` maps `CLAUDE_PLUGIN_OPTION_*` env vars into `UNIFI_*`,
refreshes the `~/.local/bin` copy of the binary, and runs check + repair. It
used to run automatically from a Claude Code `SessionStart`/`ConfigChange`
hook; those hooks have been removed.

You do **not** need to run it to configure the plugin: `plugins/unifi/.mcp.json`
maps your plugin settings into the server's environment directly via
`${user_config.*}`. Run it by hand only when you want the `~/.local/bin` binary
refresh or the preflight checks.

## Configuration

Configuration loads from `config.toml` first, then `UNIFI_*` environment
variables override it. Host installs read `~/.unifi-rmcp/.env` before loading
config; containers read `/data/.env`. Process environment overrides both.

| Variable | Default | Purpose |
|---|---|---|
| `UNIFI_URL` | unset | Controller base URL, e.g. `https://unifi.local`. |
| `UNIFI_API_KEY` | unset | API key for the `X-API-KEY` header. |
| `UNIFI_SITE` | `default` | UniFi site name. |
| `UNIFI_SITE_ID` | unset | Official API site UUID used by live tests and explicit generated calls. |
| `UNIFI_SKIP_TLS_VERIFY` | `true` | Skip TLS certificate verification for self-signed controllers. |
| `UNIFI_LEGACY` | `false` | Legacy controller mode without `/proxy/network` prefix. |
| `UNIFI_MCP_HOST` | `0.0.0.0` | HTTP bind host. |
| `UNIFI_MCP_PORT` | `40030` | HTTP bind port. |
| `UNIFI_MCP_TOKEN` | unset | Static bearer token for HTTP MCP. |
| `UNIFI_MCP_NO_AUTH` | `false` | Disable auth only for loopback development. |
| `UNIFI_MCP_DISABLE_HTTP_AUTH` | `false` | Compatibility alias for `UNIFI_MCP_NO_AUTH`. |
| `UNIFI_NOAUTH` | `false` | Trust an upstream gateway to enforce auth. |
| `UNIFI_MCP_ALLOWED_HOSTS` | empty | Comma-separated `Host` header allowlist. |
| `UNIFI_MCP_ALLOWED_ORIGINS` | empty | Comma-separated CORS `Origin` allowlist. |
| `UNIFI_MCP_PUBLIC_URL` | unset | Public URL for OAuth metadata. |
| `UNIFI_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth`. |
| `UNIFI_MCP_GOOGLE_CLIENT_ID` | unset | Google OAuth client ID. |
| `UNIFI_MCP_GOOGLE_CLIENT_SECRET` | unset | Google OAuth client secret. |
| `UNIFI_MCP_AUTH_ADMIN_EMAIL` | unset | Admin email for OAuth bootstrap. |
| `UNIFI_MCP_AUTH_SQLITE_PATH` | `/data/auth.db` | OAuth state database path. |
| `UNIFI_MCP_AUTH_KEY_PATH` | `/data/auth-jwt.pem` | OAuth JWT signing key path. |
| `UNIFI_MCP_HOME` | `~/.unifi-rmcp` | Appdata dir override; read by `setup` only. |

`config.toml`-only settings with no environment override: `mcp.server_name`
and the `[mcp.auth]` TTL, rate-limit, and allowlist fields
(`access_token_ttl_secs`, `refresh_token_ttl_secs`, `auth_code_ttl_secs`,
`register_rpm`, `authorize_rpm`, `disable_static_token_with_oauth`,
`allowed_emails`, `allowed_client_redirect_uris`). See `config.example.toml`.

`xtask` endpoint verification reads its own variables that the server never
touches: `UNIFI_ALLOW_INSECURE_TLS`, `UNIFI_RESOLVE_IP`,
`UNIFI_VERIFY_TIMEOUT_SECS`, `UNIFI_VERIFY_RATE_LIMIT_MS`,
`UNIFI_VERIFY_MAX_REQUESTS`, `UNIFI_VERIFY_UNVERIFIED_INTERNAL`.

## Authentication

Stdio MCP runs as a local trusted child process and does not use HTTP auth.

HTTP MCP auth policy:

| State | Condition | Behavior |
|---|---|---|
| Loopback dev | `UNIFI_MCP_HOST` starts with `127.` or auth is explicitly disabled on loopback | Local unauthenticated development is allowed. |
| Mounted bearer | Non-loopback with `UNIFI_MCP_TOKEN` | Requires `Authorization: Bearer <token>` and action scopes. |
| Mounted OAuth | `UNIFI_MCP_AUTH_MODE=oauth` | Uses Google OAuth/JWT through `lab-auth`. |
| Trusted gateway | `UNIFI_NOAUTH=true` | Assumes a reverse proxy or gateway already enforced auth. |

`unifi:admin` satisfies `unifi:read`; `unifi:read` does not satisfy mutating
actions.

## Safety And Trust Model

- UniFi API keys are loaded from config/env only.
- MCP callers select actions, params, and request bodies, not upstream
  credentials.
- Preserved convenience actions are read-oriented.
- Generated mutating actions require `unifi:admin` in mounted HTTP MCP mode.
- Unknown actions and malformed generated-action params fail before upstream
  calls.
- Non-loopback HTTP deployments must use bearer auth, OAuth, or a trusted
  authenticated gateway.
- This bridge does not sandbox UniFi itself. UniFi remains responsible for API
  permissions and the actual network-side effect of a mutating call.

## Architecture

```text
Capabilities   (src/capabilities.rs)   official/internal/hybrid registry
      |
UnifiClient    (src/api.rs, src/unifi.rs) HTTP path families and transport
      |
UnifiService   (src/app.rs)            action execution boundary
      |
MCP shim       (src/mcp/tools.rs)      JSON args -> service -> Value
CLI shim       (src/cli.rs)            argv -> service -> stdout
```

## Distribution Contract

- `Cargo.toml`, `Cargo.lock`, `packages/unifi-rmcp/package.json`,
  `.release-please-manifest.json`, and `server.json` must agree on the released
  version.
- GitHub Releases publish the `runifi` binary consumed by the npm launcher.
- The npm package name is `@dinglebear/runifi-mcp`; binary aliases are `@dinglebear/runifi-mcp` and
  `runifi`.
- Docker/OCI metadata uses `ghcr.io/dinglebear-ai/runifi:<version>`. The GHCR
  namespace still reads `jmagar` because that is what
  `.github/workflows/docker-publish.yml` publishes; the git remote is
  `dinglebear-ai/runifi`. Do not "fix" one without the other.
- `plugins/unifi/.mcp.json` must launch `npx -y @dinglebear/runifi-mcp mcp` so stdio
  clients start the MCP transport rather than the HTTP server.
- `plugins/unifi/` ships no Claude Code hooks. `scripts/validate-plugin-layout.sh`
  and `tests/setup_cli.rs` both fail if a `hooks/` directory or a manifest
  `hooks` key reappears.
- The root README is curated. Source of truth for current actions and generated
  endpoint coverage is `src/capabilities.rs`, the `data/` inventories,
  `docs/unifi_api_coverage.md`, and `docs/unifi_endpoint_verification.md`.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
npm --prefix packages/unifi-rmcp run check
```

## Verification

```bash
npm --prefix packages/unifi-rmcp run check
cargo check
cargo test
cargo clippy -- -D warnings
just validate-plugin
cargo run -p xtask -- verify-api-endpoints --mode contract
git diff --check
```

Live read probes require a controller:

```bash
UNIFI_URL=https://<gateway> \
UNIFI_API_KEY=<network-api-key> \
UNIFI_SITE=default \
UNIFI_SITE_ID=<official-site-uuid> \
UNIFI_SKIP_TLS_VERIFY=true \
cargo run -p xtask -- verify-api-endpoints --mode safe_live
```

## Deployment

Use loopback for local development:

```bash
UNIFI_MCP_HOST=127.0.0.1 runifi serve
```

Use Docker Compose for shared HTTP deployment:

```bash
cp .env.example .env
docker compose up -d
```

When binding to a non-loopback address, configure `UNIFI_MCP_TOKEN`,
`UNIFI_MCP_AUTH_MODE=oauth`, or `UNIFI_NOAUTH=true` behind an authenticated
gateway.

## Troubleshooting

| Symptom | Check |
|---|---|
| `UNIFI_URL` or `UNIFI_API_KEY` is missing | Set it in env or `~/.unifi-rmcp/.env`. |
| TLS errors against a UniFi controller | Keep `UNIFI_SKIP_TLS_VERIFY=true` unless the controller has a trusted certificate. |
| Legacy controller paths fail | Set `UNIFI_LEGACY=true` for older non-UDM controllers. |
| HTTP `/mcp` returns unauthorized | Set `UNIFI_MCP_TOKEN` and send `Authorization: Bearer <token>`. |
| Stdio client hangs or logs JSON errors | Ensure client config runs `unifi-rmcp mcp`, not the default HTTP server mode. |
| Generated official action needs `siteId` | Pass `--param siteId=<uuid>` or MCP `params.siteId`. |
| Mutating generated action is forbidden | Use an HTTP MCP token/session with `unifi:admin`. |

## Related Servers

All sibling servers live under the `dinglebear-ai` org.

- [soma](https://github.com/dinglebear-ai/soma) - RMCP runtime and scaffold for provider-backed MCP servers.
- [rtailscale](https://github.com/dinglebear-ai/rtailscale) - Tailscale API bridge for devices, users, and tailnet operations.
- [unraid](https://github.com/dinglebear-ai/unraid) - Unraid monorepo; `unraid-rs/` is the GraphQL bridge (binary `runraid`).
- [rapprise](https://github.com/dinglebear-ai/rapprise) - Apprise notification fan-out bridge for many delivery backends.
- [rgotify](https://github.com/dinglebear-ai/rgotify) - Gotify push notification bridge for sends, messages, apps, and clients.
- [rarcane](https://github.com/dinglebear-ai/rarcane) - Arcane Docker management bridge for containers and related resources.
- [yarr](https://github.com/dinglebear-ai/yarr) - Media-stack bridge for Sonarr, Radarr, Prowlarr, Plex, and related services.
- [rytdl](https://github.com/dinglebear-ai/rytdl) - Media download and metadata workflow server.
- [synapse](https://github.com/dinglebear-ai/synapse) - Local Synapse workflow server for scout and flux actions.
- [cortex](https://github.com/dinglebear-ai/cortex) - Syslog and homelab log aggregation MCP server.
- [axon](https://github.com/dinglebear-ai/axon) - RAG, crawl, scrape, extract, and semantic search project.
- [labby](https://github.com/dinglebear-ai/labby) - Homelab control plane and MCP gateway project.

## Documentation

- `CLAUDE.md` is the curated local operating guide for contributors and agents.
- `docs/unifi_api_coverage.md` is the curated API-family and coverage summary.
- `docs/unifi_endpoint_verification.md` is the curated endpoint verification
  guide.
- `docs/SETUP.md` is curated plugin/setup guidance.
- `docs/OAUTH.md` is curated OAuth setup guidance.
- `plugins/unifi/skills/unifi/SKILL.md` is the agent usage guide.
- `data/` inventories are generated/reference inputs for the action registry.
- `src/` is the source of truth for current action dispatch, config defaults,
  auth behavior, and CLI parsing.

## License

MIT. See [LICENSE](LICENSE).
