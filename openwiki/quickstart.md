---
type: "Reference"
title: "unifi-rmcp - UniFi MCP Server"
openwiki_generated: true
---

# unifi-rmcp - UniFi MCP Server

**unifi-rmcp** is a Rust-based MCP (Model Context Protocol) server that bridges AI clients to Ubiquiti UniFi network controllers. It provides read-only access to UniFi networks through both CLI tools and MCP server interfaces.

## What unifi-rmcp Does

unifi-rmcp exposes UniFi controller REST APIs to AI clients (Claude, Cursor, etc.) and provides a CLI for direct querying. It supports:

- **Official Network Integration API** — Documented UniFi OS endpoints under `/proxy/network/integration/v1/`
- **Internal V1/V2 Site APIs** — Preserved controller actions under `/proxy/network/api/s/{site}/` and `/proxy/network/v2/api/site/{site}/`
- **Hybrid Convenience Actions** — Smart actions that choose the best API based on parameters

## Quick Start

### Prerequisites

- Rust 1.86+
- UniFi controller (UDM/UDR or legacy)
- API key from your UniFi controller

### 1. Clone and Build

```bash
git clone https://github.com/jmagar/runifi.git
cd unifi-rmcp
cargo build --release
```

### 2. Configure Environment

```bash
cp .env.example .env
# Edit .env and set:
#   UNIFI_URL=https://your-controller.local
#   UNIFI_API_KEY=your-api-key
```

### 3. Run CLI Commands

```bash
# Load environment and run commands
source .env
cargo run --bin runifi -- health --json
cargo run --bin runifi -- clients --json
cargo run --bin runifi -- devices --json
```

### 4. Start MCP Server

**HTTP server (port 40030):**
```bash
cargo run --bin runifi
```

**stdio transport (for Claude Desktop):**
```bash
cargo run --bin runifi -- mcp
```

## Architecture Overview

unifi-rmcp follows a strict layered architecture:

```
UnifiClient (src/unifi.rs)   — HTTP client, no business logic
    ↓
UnifiService (src/app.rs)    — all business logic
    ↓
tools.rs / cli.rs            — thin shims (parse + dispatch + format)
```

**Key principle**: Never add business logic to tools.rs, cli.rs, or main.rs.

### Action Dispatch System

unifi-rmcp uses a capability-based action dispatch system:

- **official_* actions** — 78+ documented Network Integration API operations
- **unifi_* actions** — Internal controller-compatible actions
- **Hybrid actions** — Smart actions like `list_clients`, `list_devices` that choose the best API

The action dispatcher (`src/actions.rs`) routes requests to the appropriate API client based on capability definitions.

### API Families

| Family | Path | Description |
|--------|------|-------------|
| Official | `/proxy/network/integration/v1/...` | Documented Network Integration API |
| Internal V1 | `/proxy/network/api/s/{site}/...` | Internal site API (legacy) |
| Internal V2 | `/proxy/network/v2/api/site/{site}/...` | Internal site API (modern) |

**Legacy controllers** (non-UDM, port 8443) use the same paths without the `/proxy/network` prefix. Set `UNIFI_LEGACY=true` for those.

## Key Concepts

### UniFi Controller Types

- **UniFi OS (UDM/UDR)** — Modern controllers with `/proxy/network` prefix
- **Legacy controllers** — Older controllers on port 8443 without prefix

### TLS Verification

UniFi controllers use self-signed certificates by default. Always set `UNIFI_SKIP_TLS_VERIFY=true` unless you have installed a valid certificate.

### Authentication Modes

The MCP server supports three authentication modes:

| Mode | Use Case |
|------|----------|
| `bearer` | Static bearer token (default) |
| `oauth` | Google OAuth flow |
| `loopback` | No auth (local development only) |

See [OAUTH.md](docs/OAUTH.md) for OAuth setup details.

### MCP Tool Interface

unifi-rmcp exposes one MCP tool named `unifi`. The required `action` argument selects the operation:

```json
{
  "action": "clients",
  "params": {}
}
```

Available actions include:
- `clients` — Connected wireless and wired clients
- `devices` — Network devices (APs, switches, gateways)
- `wlans` — WiFi network configurations
- `health` — Site health summary
- `alarms` — Active alarms and alerts
- `events` — Recent controller events
- `sysinfo` — Controller system information
- `me` — Authenticated user info
- `help` — Tool documentation

Plus generated `official_*` and `unifi_*` actions for full API coverage.

## Development Workflow

### Adding a New Action

1. **src/unifi.rs** — Add REST method
2. **src/app.rs** — Delegate to UnifiService
3. **src/mcp/tools.rs** — Add dispatch arm
4. **src/mcp/schemas.rs** — Update schema if needed
5. **src/cli.rs** — Add CLI command variant

### Testing

The repository includes:
- **Unit tests** — `cargo test` (no network required)
- **Live smoke tests** — `tests/live_internal_smoke.rs`, `tests/live_official_smoke.rs`
- **Endpoint verification** — `cargo run -p xtask -- verify-api-endpoints --mode contract`

Run verification:
```bash
cargo run -p xtask -- verify-api-endpoints --mode contract
```

### Project Structure

```
src/
├── main.rs           # Entry point, CLI modes
├── lib.rs            # Module exports
├── config.rs         # Configuration loading
├── unifi.rs          # HTTP client
├── app.rs            # Business service layer
├── actions.rs        # Action dispatcher
├── actions/          # Action family implementations
├── api.rs            # API client abstractions
├── api/              # API family clients
├── cli.rs            # CLI commands
├── mcp.rs            # MCP server setup
├── mcp/              # MCP server internals
└── ...

tests/                # Unit and integration tests
xtask/                # Build utilities and verification
docs/                 # Comprehensive documentation
```

## Configuration

### Environment Variables

Required:
- `UNIFI_URL` — Controller base URL (e.g., `https://unifi.local`)
- `UNIFI_API_KEY` — API key for `X-API-KEY` header

Optional (with defaults):
- `UNIFI_SITE` — Site name (default: `default`)
- `UNIFI_SKIP_TLS_VERIFY` — Skip TLS verification (default: `true`)
- `UNIFI_LEGACY` — Legacy controller mode (default: `false`)
- `UNIFI_MCP_HOST` — MCP server bind host (default: `0.0.0.0`)
- `UNIFI_MCP_PORT` — MCP server bind port (default: `40030`)

See [CONFIG.md](docs/CONFIG.md) for complete configuration reference.

### Config File

Local development can use `config.toml` (not used in Docker):

```toml
[unifi]
url = "https://unifi.local"
api_key = "your-api-key"
site = "default"
skip_tls_verify = true

[mcp]
host = "0.0.0.0"
port = 40030
```

## Further Reading

### Project Documentation

- [SETUP.md](docs/SETUP.md) — Complete setup guide (clone, build, configure, deploy)
- [CLI.md](docs/CLI.md) — Full CLI command reference
- [CONFIG.md](docs/CONFIG.md) — All configuration options
- [OAUTH.md](docs/OAUTH.md) — OAuth authentication setup
- [GUARDRAILS.md](docs/GUARDRAILS.md) — Security guardrails

### MCP Server Documentation

- [docs/mcp/AGENTS.md](docs/mcp/AGENTS.md) — MCP agent instructions
- [docs/mcp/AUTH.md](docs/mcp/AUTH.md) — Authentication implementation
- [docs/mcp/TOOLS.md](docs/mcp/TOOLS.md) — MCP tool reference
- [docs/mcp/DEPLOY.md](docs/mcp/DEPLOY.md) — Deployment guide

### Developer Documentation

- [AGENTS.md](AGENTS.md) — Essential commands for working on unifi-rmcp
- [docs/RUST.md](docs/RUST.md) — Rust toolchain and workspace notes
- [docs/INVENTORY.md](docs/INVENTORY.md) — Component inventory

### API Coverage

- [docs/unifi_api_coverage.md](docs/unifi_api_coverage.md) — API coverage inventory
- [docs/unifi_endpoint_verification.md](docs/unifi_endpoint_verification.md) — Endpoint verification documentation

## Essential Commands

For developers working on unifi-rmcp:

```bash
cargo check                              # Type-check (must pass before any PR)
cargo test                               # Run all tests (no network required)
cargo run --bin runifi -- --help         # CLI help
cargo run --bin runifi -- health --json  # Test a live action
cargo run --bin runifi                   # HTTP MCP server on :40030
cargo run --bin runifi -- mcp            # stdio MCP transport
```

See [AGENTS.md](AGENTS.md) for complete development instructions.
