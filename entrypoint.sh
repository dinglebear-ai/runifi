#!/bin/sh
# entrypoint.sh — Docker entrypoint for runifi (UniFi MCP server)
# Runs as root, validates config, sets up /data, then exec's as USER 1000:1000
set -e

SERVICE_NAME="runifi"
BINARY="/usr/local/bin/${SERVICE_NAME}"

DATA_DIR="${DATA_DIR:-/data}"

case "${1:-}" in
    --help|-h|--version|-V|version)
        exec gosu 1000:1000 "${BINARY}" "$@"
        ;;
esac

# Load mounted runtime config before validation. The Rust binary also loads
# /data/.env, but entrypoint validates required vars first.
if [ -f "${DATA_DIR}/.env" ]; then
    set -a
    # shellcheck disable=SC1091
    . "${DATA_DIR}/.env"
    set +a
fi

# Validate required environment variables
if [ -z "${UNIFI_URL:-}" ]; then
    echo "ERROR: UNIFI_URL is not set" >&2
    echo "  Set UNIFI_URL to your UniFi controller URL, e.g. https://unifi.local" >&2
    exit 1
fi

if [ -z "${UNIFI_API_KEY:-}" ]; then
    echo "ERROR: UNIFI_API_KEY is not set" >&2
    echo "  Generate an API key in: UniFi OS → Settings → Admins & Users → API Keys" >&2
    exit 1
fi

# Ensure data directory exists and is owned by the service user
mkdir -p "${DATA_DIR}"
chown -R 1000:1000 "${DATA_DIR}"
chmod 750 "${DATA_DIR}"

# Tighten permissions on secrets if present
if [ -f "${DATA_DIR}/config.toml" ]; then
    chmod 640 "${DATA_DIR}/config.toml"
fi
if [ -f "${DATA_DIR}/.env" ]; then
    chmod 600 "${DATA_DIR}/.env"
fi

# Drop to service user and exec the binary
exec gosu 1000:1000 "${BINARY}" "$@"
