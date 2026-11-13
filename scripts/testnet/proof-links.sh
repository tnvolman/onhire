#!/usr/bin/env bash
# Print a markdown list of every recorded testnet tx for the given runner
# PIDs (from logs/testnet-records.jsonl), grouped by test, with explorer links.
# Usage: scripts/testnet/proof-links.sh <pid> [<pid> ...]
set -euo pipefail
cd "$(dirname "$0")/../.."
for pid in "$@"; do
  t=$(jq -r --argjson p "$pid" 'select(.pid == $p) | .test' logs/testnet-records.jsonl | head -1)
  echo "#### $t (pid $pid)"
  echo
  echo "| after step | call | signer | result | tx |"
  echo "| --- | --- | --- | --- | --- |"
  jq -r --argjson p "$pid" 'select(.pid == $p and .tx != null) |
    "| \(.step) | `\(.method)` → `\(.receiver)` | `\(.signer)` | \(if .ok then "ok" elif .landed == false then "rejected (not executed)" else "refused (expected)" end) | [\(.tx[0:8])…](https://testnet.nearblocks.io/txns/\(.tx)) |"' logs/testnet-records.jsonl
  echo
done
