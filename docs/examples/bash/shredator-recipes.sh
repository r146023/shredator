#!/usr/bin/env bash
set -euo pipefail

# Example: strict JSON run.
shredator_strict() {
  local target="$1"
  local out
  out="$(shredator "$target" --force --json)"

  local status warnings errors
  status="$(printf '%s' "$out" | jq -r '.status')"
  warnings="$(printf '%s' "$out" | jq -r '.summary.warnings')"
  errors="$(printf '%s' "$out" | jq -r '.summary.errors')"

  if [[ "$status" != "completed" || "$warnings" != "0" || "$errors" != "0" ]]; then
    echo "Shredator did not complete cleanly" >&2
    printf '%s\n' "$out" | jq . >&2
    return 1
  fi

  printf '%s\n' "$out"
}

# Example usage:
# printf 'secret\n' > /tmp/shredator-example.txt
# shredator_strict /tmp/shredator-example.txt | jq .
