#!/usr/bin/env bash
set -euo pipefail

staged=$(git diff --cached --name-only)
blocked=$(printf '%s
' "$staged" | grep -E '(^|/)[^/]*\.env[^/]*$' | grep -v '\.env\.example$' || true)

if [[ -n "$blocked" ]]; then
  echo "block-env-commits: BLOCKED — .env file(s) staged for commit:" >&2
  while IFS= read -r file; do
    printf '  %s\n' "$file"
  done <<< "$blocked" >&2
  echo "Only .env.example is allowed. Remove staged file(s) and try again." >&2
  exit 1
fi
