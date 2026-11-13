#!/usr/bin/env bash
# Cross-check: for every throwaway testnet signer, the number of transactions
# its key has signed on chain (access-key nonce minus the creation base,
# block_height * 1e6) must equal the number recorded in
# logs/testnet-records.jsonl (+ helper/faucet creation, which does not use our
# key). Any surplus means a transaction was signed that we did not record.
set -euo pipefail
cd "$(dirname "$0")/../.."
RPC="${NEAR_TESTNET_RPC:-https://rpc.testnet.fastnear.com}"
KEYS="${ONHIRE_KEYS:-$HOME/.onhire-testnet}"
REC=logs/testnet-records.jsonl
printf "%-34s %8s %8s %s\n" account onchain recorded status
for f in "$KEYS"/*.json; do
  id=$(jq -r .account_id "$f"); pk=$(jq -r .public_key "$f")
  r=$(curl -s "$RPC" -H 'Content-Type: application/json' -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"query\",\"params\":{\"request_type\":\"view_access_key\",\"finality\":\"final\",\"account_id\":\"$id\",\"public_key\":\"$pk\"}}")
  nonce=$(echo "$r" | jq -r '.result.nonce // empty')
  [ -z "$nonce" ] && { printf "%-34s %8s %8s %s\n" "$id" - - "no key (deleted account)"; continue; }
  n=$((nonce % 1000000))
  rec=$(jq -r --arg id "$id" 'select(.signer == $id and .tx != null) | .tx' "$REC" | sort -u | wc -l)
  st=OK; [ "$n" -ne "$rec" ] && st="MISMATCH ($((n - rec)) unrecorded)"
  printf "%-34s %8s %8s %s\n" "$id" "$n" "$rec" "$st"
done
