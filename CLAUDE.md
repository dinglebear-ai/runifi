# unifi-rmcp — CLAUDE.md

Rust MCP server and CLI bridging the **UniFi Network controller REST API**.
Exposes one action-dispatched MCP tool (`unifi`) plus the `runifi` CLI at
full parity.

> **Not read-only.** An earlier version of this file described this as a
> "read-only REST bridge". That has not been true since the generated action
> registry landed: 123 of the 266 actions are mutating and require
> `unifi:admin`.

## Repo Facts

| Fact | Value |
|---|---|
| Remote | `git@github.com:dinglebear-ai/runifi.git` |
| Default branch | `main` |
| Cargo workspace | 3 members: `.` (root), `crates/unifi`, `xtask` |
| Root package | `unifi-rmcp` (edition 2024) |
| Binary / CLI | `runifi` |
| MCP tool | `unifi` |
| Service port | **40030** |
| MSRV | 1.97.1 |
| npm package | `@dinglebear/runifi` (`packages/unifi-rmcp/`) |

All three members inherit edition 2024, Rust 1.97.1, package metadata, shared
dependencies, and the workspace lint policy. `rmcp` is exactly pinned to
`=3.0.0-beta.2`, matching `Cargo.lock`; do not replace it with a caret range.

`lab-auth` is a pinned git dependency on `dinglebear-ai/labby` (rev
`87cec32`), already repointed off the dead `jmagar/lab.git` URL.

## The Controller

The target controller in this homelab is a **UniFi Cloud Gateway Max at
`10.1.0.1`**, aliased **`mothership`**. It is the LAN router, gateway, DHCP
server, WLAN controller, and UniFi Network controller in one box. Mutating
actions against it change live network state — there is no staging tier.

## Module Map

```
src/                          # unifi-rmcp — MCP/CLI projection layer
  main.rs                     Dispatch: serve_mcp / serve_stdio_mcp / doctor / setup / run_cli
                              + print_usage + validate_bind_security
  lib.rs                      Module declarations, OAuth wiring, testing:: hook
  config.rs                   Config/McpConfig/AuthConfig — config.toml then UNIFI_* env overrides
  setup.rs                    setup check | repair | install | plugin-hook; apply_plugin_options()
  cli.rs                      argv -> UnifiService -> stdout (thin shim)
  cli/doctor.rs               Pre-flight environment diagnosis
  logging.rs, logging/aurora.rs   Tracing setup + Aurora CLI theming
  observability.rs            Tracing/metrics init
  token_limit.rs              Response size guardrail
  mcp.rs                      AppState, AuthPolicy, build_auth_layer, pub exports
  mcp/rmcp_server.rs          UnifiRmcpServer — rmcp ServerHandler (tools/resources/prompts) + scope checks
  mcp/tools.rs                execute_tool() -> dispatch() — thin shim, plus HELP_TEXT
  mcp/schemas.rs              tool_definitions() — builds the action enum from all_capabilities()
  mcp/prompts.rs              list_prompts() / get_prompt() — network_summary
  mcp/routes.rs               axum router, auth middleware, /health

crates/unifi/                 # unifi — reusable core (no MCP/CLI types)
  lib.rs                      ActionRequest, re-exports
  service.rs                  UnifiService — the action execution boundary
  client.rs, http.rs          HTTP transport
  config.rs                   Controller-side config struct
  api.rs, api/{official,internal,path}.rs   Path families + ApiSourceFamily
  actions.rs, actions/{official,internal,hybrid}.rs   Action routing; hybrid::resolve()
  capabilities.rs             Capability, AuthScope, all_capabilities(), find_capability()
  capabilities/official_network.rs   Parses data/unifi_official_network_v10_3_58.json
  capabilities/internal_network.rs   Parses data/unifi_internal_endpoint_models.json + legacy/hybrid

xtask/                        # dev tooling (edition 2024)
  verify_endpoints.rs, endpoint_probe.rs   contract + safe_live endpoint verification
  official_api.rs, internal_reference.rs   inventory regeneration
  forbidden_strings.rs, verify_policy.rs   guardrails
```

## Action Surface

The action enum is **generated at build time from JSON inventories in
`data/`** — it is not hand-maintained. `all_capabilities()` is the single
source of truth.

| Family | Count | Source | Path base |
|---|---:|---|---|
| `official_*` | 78 (36 mutating) | `data/unifi_official_network_v10_3_58.json` | `/proxy/network/integration/v1` |
| `unifi_*` | 175 (87 mutating) | `data/unifi_internal_endpoint_models.json` (`runtime: true` only) | `/proxy/network/api/s/{site}`, `/proxy/network/v2/api/site/{site}` |
| Legacy convenience | 8 (read-only) | hardcoded in `internal_network.rs` | internal paths |
| Hybrid aliases | 5 | hardcoded in `internal_network.rs` | resolves to official or internal |
| `help` | 1 | `mcp/tools.rs` | n/a |
| **Total in enum** | **267** | | |

**Legacy convenience actions** (all read-only, `verification_mode:
"legacy_alias"`): `clients`, `devices`, `wlans`, `health`, `alarms`,
`events`, `sysinfo`, `me`.

**Hybrid aliases**: `list_clients`, `list_devices`, `list_networks`,
`list_wifi`, `get_system_info`. `hybrid::resolve()` picks the backend:

- `params.prefer = "official" | "internal"` — explicit
- no `prefer`, but `params.siteId` present — official
- otherwise — internal (default)

`prefer` is stripped from params before dispatch.

## How to Add a New Action

Generated actions come from the `data/` inventories, so most additions are
data changes, not code changes:

1. **Generated action** — add/regenerate the entry in the relevant `data/`
   JSON (`cargo run -p xtask -- official-api` / `internal-reference`), then
   run `cargo run -p xtask -- verify-api-endpoints --mode contract`.
   Registration, scope, and the MCP schema enum follow automatically.
2. **Hybrid alias** — add to `official_target()` and `internal_target()` in
   `crates/unifi/src/actions/hybrid.rs`, plus a `hybrid(...)` entry in
   `capabilities/internal_network.rs`.
3. **Legacy convenience action** — add a `legacy(...)` entry in
   `capabilities/internal_network.rs` and a `CliCommand` variant + formatter
   in `src/cli.rs`.

Do **not** hand-edit the action enum in `src/mcp/schemas.rs` — it is derived.

## Strict Layering Rules

- **All business logic** lives in `crates/unifi` (`UnifiService`).
- **All HTTP calls** live in `crates/unifi/src/{client,http}.rs`.
- `src/mcp/tools.rs` and `src/cli.rs` are thin shims: parse args, call the
  service, return/print.
- No logic in `main.rs` beyond dispatch and bind-security validation.
- `tests/core_crate_boundary.rs` enforces that `crates/unifi` stays free of
  MCP/CLI types.

## UniFi API Path Families

**UniFi OS / UDM (default):**
```
/proxy/network/integration/v1/...           official_* (Network Integration API)
/proxy/network/api/s/{site}/...             unifi_* + legacy convenience
/proxy/network/v2/api/site/{site}/...       unifi_* (v2 endpoints)
/api/self                                   me (no /proxy/network prefix)
```

**Legacy (`UNIFI_LEGACY=true`):** same paths without the `/proxy/network`
prefix, for pre-UDM controllers on port 8443.

**Response shape:** site-scoped internal endpoints return
`{"meta": {"rc": "ok"}, "data": [...]}`; `me` returns `{"data": {...}}`. The
client returns the raw `Value`; callers index `["data"]`.

## Auth

`AuthPolicy` (in `src/mcp.rs`) has three effective states:

| Variant | Meaning |
|---|---|
| `LoopbackDev` | No auth. Legal only on a loopback bind — the bind *is* the trust boundary. Scope checks are bypassed. |
| `Mounted { auth_state: None }` | Static bearer token (`UNIFI_MCP_TOKEN`). Scope checks run. |
| `Mounted { auth_state: Some(_) }` | Google OAuth/JWT via `lab-auth`. Scope checks run. |

Scopes: `unifi:read` and `unifi:admin`. **`unifi:admin` satisfies
`unifi:read`; the reverse is not true.** The required scope per action comes
from `Capability::auth_scope` — GET/non-mutating maps to read, everything
else to admin. The static bearer token is granted both scopes.

`validate_bind_security()` in `main.rs` refuses a non-loopback bind unless
`UNIFI_MCP_TOKEN` is set, OAuth mode is on, or `UNIFI_NOAUTH=true` declares
that an upstream gateway handles auth.

Stdio MCP runs as a trusted local child process and does no HTTP auth.

## Configuration

Precedence: **`config.toml` → `UNIFI_*` env overrides** (env always wins).
Host installs also source `~/.unifi-rmcp/.env` first; containers source
`/data/.env` via `entrypoint.sh`.

### Env vars read by the runtime

```
UNIFI_URL                     Controller base URL (required)
UNIFI_API_KEY                 X-API-KEY header value (required)
UNIFI_SITE                    Site name (default: default)
UNIFI_SITE_ID                 Official API site UUID (official_* live calls/tests)
UNIFI_SKIP_TLS_VERIFY         Skip TLS cert check (default: true)
UNIFI_LEGACY                  Drop /proxy/network prefix (default: false)

UNIFI_MCP_HOST                Bind host (default: 0.0.0.0)
UNIFI_MCP_PORT                Bind port (default: 40030)
UNIFI_MCP_TOKEN               Static bearer token
UNIFI_MCP_NO_AUTH             Disable auth (loopback only)
UNIFI_MCP_DISABLE_HTTP_AUTH   Compatibility alias for UNIFI_MCP_NO_AUTH
UNIFI_NOAUTH                  Assert an upstream gateway enforces auth
UNIFI_MCP_ALLOWED_HOSTS       Comma-separated Host allowlist
UNIFI_MCP_ALLOWED_ORIGINS     Comma-separated Origin allowlist
UNIFI_MCP_PUBLIC_URL          Public URL for OAuth metadata
UNIFI_MCP_AUTH_MODE           bearer | oauth
UNIFI_MCP_AUTH_ADMIN_EMAIL    OAuth bootstrap admin
UNIFI_MCP_GOOGLE_CLIENT_ID    OAuth client id
UNIFI_MCP_GOOGLE_CLIENT_SECRET  OAuth client secret
UNIFI_MCP_AUTH_SQLITE_PATH    OAuth state DB
UNIFI_MCP_AUTH_KEY_PATH       OAuth JWT signing key
UNIFI_MCP_HOME                Override appdata dir (setup/plugin-hook only)
```

**`config.toml`-only** (no env override): `mcp.server_name`, and the
`[mcp.auth]` TTL/rate-limit fields (`access_token_ttl_secs`,
`refresh_token_ttl_secs`, `auth_code_ttl_secs`, `register_rpm`,
`authorize_rpm`, `disable_static_token_with_oauth`, `allowed_emails`,
`allowed_client_redirect_uris`). There is **no** `UNIFI_MCP_SERVER_NAME` env
var despite what older docs claimed.

**xtask-only** (endpoint verification, never read by the server):
`UNIFI_ALLOW_INSECURE_TLS`, `UNIFI_RESOLVE_IP`, `UNIFI_VERIFY_TIMEOUT_SECS`,
`UNIFI_VERIFY_RATE_LIMIT_MS`, `UNIFI_VERIFY_MAX_REQUESTS`,
`UNIFI_VERIFY_UNVERIFIED_INTERNAL`.

## CLI ↔ MCP Parity

Every action is reachable from both surfaces through the same
`UnifiService`. Named CLI subcommands exist for the 8 legacy convenience
actions; every other action is reached generically:

```bash
runifi <action> [--param k=v]... [--body-json JSON] [--json]
```

| Surface | Invocation |
|---|---|
| Legacy convenience | `runifi clients --json` / `unifi(action="clients")` |
| Events with limit | `runifi events --limit 50` / `unifi(action="events", params={"limit":50})` |
| Official | `runifi official_list_clients --param siteId=<uuid>` |
| Internal | `runifi unifi_list_wlans --json` |
| Hybrid | `runifi list_clients --param prefer=official --param siteId=<uuid>` |
| Mutating | `runifi official_create_network --param siteId=<uuid> --body-json '{"name":"IoT"}'` |
| Help | `runifi --help` / `unifi(action="help")` |

## Plugin

`plugins/unifi/` ships the Claude Code and Codex plugin: manifests, `.mcp.json`,
the bundled `bin/runifi`, and `skills/unifi/SKILL.md`.

**Claude Code plugin hooks have been retired.** There is no
`plugins/unifi/hooks/` directory and neither manifest declares a `hooks` key;
`scripts/validate-plugin-layout.sh` and `tests/setup_cli.rs` both assert they
stay gone.

The binary command the hook used to run still exists and is now **manual**:

```bash
runifi setup plugin-hook [--no-repair] [--json]
```

It maps `CLAUDE_PLUGIN_OPTION_*` env vars into `UNIFI_*`, refreshes the
`~/.local/bin` copy of the binary, and runs check + auto-repair.

**Credentials do not depend on it.** `plugins/unifi/.mcp.json` carries an `env`
block that maps 12 `userConfig` keys straight into the server process
(`${user_config.unifi_url}` → `UNIFI_URL`, and so on), so plugin settings reach
the runtime on every launch with no hook and no manual step. `apply_plugin_options()`
in `src/setup.rs` is the parallel `CLAUDE_PLUGIN_OPTION_*` translation used only
when you invoke `setup plugin-hook` by hand — **keep the two maps in sync when you
add or rename a `userConfig` key.**

What `setup plugin-hook` still buys you manually: the `~/.local/bin` binary
refresh and the preflight checks. Neither is required for the server to start
configured.

## Build & Test

```bash
cargo check                                   # type-check
cargo test                                    # unit + integration (no network)
cargo clippy -- -D warnings
cargo fmt --check
just validate-plugin                          # plugin manifests/skills/no-hooks
cargo run -p xtask -- verify-api-endpoints --mode contract

cargo run --bin runifi -- --help
cargo run --bin runifi -- doctor --json
cargo run --bin runifi -- health --json
cargo run --bin runifi                        # HTTP MCP server on :40030
cargo run --bin runifi -- mcp                 # stdio MCP transport
```

Live probes need a real controller and are opt-in:

```bash
UNIFI_URL=https://10.1.0.1 UNIFI_API_KEY=... UNIFI_SITE_ID=<uuid> \
  cargo run -p xtask -- verify-api-endpoints --mode safe_live
```

`tests/live_internal_smoke.rs` and `tests/live_official_smoke.rs` are the
network-touching tests; the rest run offline.

## Agent Memory Files

`AGENTS.md` and `GEMINI.md` are symlinks to this file. Never edit them
directly — edit `CLAUDE.md`. Recreate with:

```bash
ln -sf CLAUDE.md AGENTS.md
ln -sf CLAUDE.md GEMINI.md
```

## Known Doc Debt

`docs/INVENTORY.md`, `docs/repo/REPO.md`, and `docs/mcp/PRE-COMMIT.md` are
boilerplate copied from `syslog-mcp`/cortex — they reference `skills/syslog/`,
a SQLite WAL backup script, and a `hooks/scripts/` tree that have never
existed in this repo. Treat them as untrusted; `src/`, `data/`, and this file
are authoritative.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
