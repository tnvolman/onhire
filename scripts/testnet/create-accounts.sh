#!/usr/bin/env bash
# Create the throwaway NEAR *testnet* accounts used by the testnet proof, via
# the public testnet faucet helper (10 test NEAR each, free). Testnet only.
# Keys are written OUTSIDE the repo to $ONHIRE_KEYS (default ~/.onhire-testnet).
# Usage: scripts/testnet/create-accounts.sh <suffix>
#   -> onhire.testnet (or $ONHIRE_COLLECTION) plus onhire-<role>-<suffix>.testnet
set -euo pipefail
cd "$(dirname "$0")"
SUFFIX="${1:?usage: create-accounts.sh <suffix>}"
KEYS="${ONHIRE_KEYS:-$HOME/.onhire-testnet}"
HELPER="https://helper.testnet.near.org/account"
RPC="${NEAR_TESTNET_RPC:-https://rpc.testnet.near.org}"
COLL="${ONHIRE_COLLECTION:-onhire.testnet}"
mkdir -p "$KEYS"; chmod 700 "$KEYS"
ROLES=(owner lessee buyer buyer2 inspector outsider racer1 racer2 racer3 funder1 funder2 funder3)
create() {
  local id="$1" f="$KEYS/$1.json"
  case "$id" in *.testnet) ;; *) echo "refusing non-testnet id $id" >&2; exit 1;; esac
  if [ -f "$f" ]; then echo "exists: $id"; return; fi
  node keygen.js | jq --arg id "$id" '. + {account_id: $id}' > "$f.tmp"
  local pk; pk=$(jq -r .public_key "$f.tmp")
  local out; out=$(curl -s -m 90 -X POST "$HELPER" -H 'Content-Type: application/json' \
      -d "{\"newAccountId\":\"$id\",\"newAccountPublicKey\":\"$pk\"}")
  local tx; tx=$(echo "$out" | jq -r '.transaction.hash // empty' 2>/dev/null || true)
  local st; st=$(echo "$out" | jq -c '.status' 2>/dev/null || echo "$out" | head -c 300)
  if [ -z "$tx" ] || ! echo "$st" | grep -q SuccessValue; then
    echo "FAILED to create $id: $(echo "$out" | head -c 400)" >&2; rm -f "$f.tmp"; return 1
  fi
  mv "$f.tmp" "$f"; chmod 600 "$f"
  echo "created $id tx=$tx https://testnet.nearblocks.io/txns/$tx"
}
create "$COLL"
for r in "${ROLES[@]}"; do create "onhire-$r-$SUFFIX.testnet"; sleep 2; done
for f in "$KEYS"/*.json; do
  id=$(jq -r .account_id "$f"); [ "$id" = null ] && continue
  amt=$(curl -s "$RPC" -H 'Content-Type: application/json' -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"query\",\"params\":{\"request_type\":\"view_account\",\"finality\":\"final\",\"account_id\":\"$id\"}}" | jq -r '.result.amount // "missing"')
  echo "$id $amt"
done
