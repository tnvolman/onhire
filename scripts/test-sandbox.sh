#!/usr/bin/env bash
# Build both contracts, run the unit tests, then the seven sandbox tests in a
# near-workspaces sandbox, in order (t1..t7), with invariant + zero-key checks after every step.
# Usage: scripts/test-sandbox.sh            (global item code, NEP-591)
#        ONHIRE_CODE_MODE=local scripts/test-sandbox.sh   (fallback deploy)
set -euo pipefail
cd "$(dirname "$0")/.."
# near-workspaces links system OpenSSL; help openssl-sys find it if pkg-config is absent.
if ! command -v pkg-config >/dev/null && [ -d /usr/include/openssl ]; then
  export OPENSSL_LIB_DIR="${OPENSSL_LIB_DIR:-/usr/lib/x86_64-linux-gnu}" OPENSSL_INCLUDE_DIR="${OPENSSL_INCLUDE_DIR:-/usr/include}"
fi
(cd contracts/item && cargo near build non-reproducible-wasm --no-abi)
(cd contracts/collection && cargo near build non-reproducible-wasm --no-abi)
cargo test -p onhire-item -p onhire-collection
cargo test -p onhire-tests --test sandbox -- --test-threads=1 --nocapture
