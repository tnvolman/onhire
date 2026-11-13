#!/usr/bin/env bash
# Run the seven tests (t1..t7) against the contracts already deployed on
# NEAR *testnet* (collection $ONHIRE_COLLECTION, default v3.onhire.testnet,
# the two-step-lease build; item code as a NEP-591 global
# contract), in order t1..t7 by default, with invariant + zero-key checks after every
# step. Every tx hash is appended to logs/testnet-records.jsonl.
# Prereqs: scripts/testnet/create-accounts.sh <suffix>, onhire-testnet fee-setup, then
#   onhire-testnet deploy (one time). t1-t6 also check the 1% fee; t7 = exact fee splits.
# Testnet only. Usage: scripts/test-testnet.sh [t1 t2 ...]
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v pkg-config >/dev/null && [ -d /usr/include/openssl ]; then
  export OPENSSL_LIB_DIR="${OPENSSL_LIB_DIR:-/usr/lib/x86_64-linux-gnu}" OPENSSL_INCLUDE_DIR="${OPENSSL_INCLUDE_DIR:-/usr/include}"
fi
export NEAR_TESTNET_RPC="${NEAR_TESTNET_RPC:-https://rpc.testnet.fastnear.com}"
cargo build -q -p onhire-tests --bin onhire-testnet
TESTS=("$@"); [ ${#TESTS[@]} -eq 0 ] && TESTS=(t1 t2 t3 t4 t5 t6 t7)
rc=0
for t in "${TESTS[@]}"; do
  target/debug/onhire-testnet "$t" || rc=1
done
exit $rc
