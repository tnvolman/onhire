# Onhire — sandbox proof (tests 1–6, before the protocol fee)

Spec `docs/ONHIRE-SPEC-v1.1.md`. Run 2026-11-12 (UTC),
local near-workspaces sandbox.

Sandbox run of 2026-11-12, before testnet and before the 1% protocol fee. This document covers tests 1–6. Test 7 (protocol fee) was added later and is proven in `docs/FEE-PROOF.md`. Later testnet results for tests 1–6 are in `docs/TESTNET-PROOF.md`.

## Commands

```bash
# one command: builds both contracts with cargo-near, runs unit tests,
# then the six sandbox tests in order (t1..t6, --test-threads=1)
scripts/test-sandbox.sh

# same six tests using the documented fallback (item code deployed per box
# instead of a NEP-591 global contract)
ONHIRE_CODE_MODE=local cargo test -p onhire-tests --test sandbox -- --test-threads=1 --nocapture
```

What `scripts/test-sandbox.sh` runs:

```bash
(cd contracts/item && cargo near build non-reproducible-wasm --no-abi)
(cd contracts/collection && cargo near build non-reproducible-wasm --no-abi)
cargo test -p onhire-item -p onhire-collection
cargo test -p onhire-tests --test sandbox -- --test-threads=1 --nocapture
```

Toolchain: rustc/cargo 1.99.1, cargo-near 0.23.0 (official prebuilt release
binary, sha256 verified; `cargo install` failed here for lack of `make`),
near-sdk 5.30.1, near-workspaces 0.24.0. On this machine `pkg-config` is
absent, so the script exports `OPENSSL_LIB_DIR`/`OPENSSL_INCLUDE_DIR` for
`openssl-sys`.

Built wasm (sha256):

- `onhire_item.wasm` `0826451ccbd0d4512f4d02f04a675c0a89bbf39258da66ae5589a69c07987f79`
- `onhire_collection.wasm` `3de6249e3e1f14877a9b89b2656f3e26281c8bd4dc19d3be829ea5c5743e6c88`

## Results

After **every** step, the harness (`tests/src/lib.rs`, `Env::check`) asserts
rule 14 twice — from the box's `get_invariant_parts` and independently from
the RPC account view (`balance >= storage_usage*1e19 + reserve + pending +
deposits + locked`) — and asserts the box account has **zero access keys**.

| Test | Result (global code) | Result (local fallback) | Checked steps |
| --- | --- | --- | --- |
| 1 — Mid-lease sale | PASS | PASS | 13 |
| 2 — Failed collection transfer undoes the sale | PASS | PASS | 10 |
| 3 — Direct transfer is blocked | PASS | PASS | 16 |
| 4 — Stuck lock (state patching) | PASS | PASS | 20 |
| 5 — Lapsed inspection | PASS | PASS | 11 |
| 6 — Reserve cut; owner cannot take it | PASS | PASS | 7 |

`test result: ok. 6 passed; 0 failed` in 78 s (global) and 78 s (local).
Unit tests: `8 passed` (item: reserve split, callback finalize/undo, stale
callback no-op including while a newer lock is held, rescue rules,
wrong-amount buy, pay_hire while locked, inspector ≠ owner).
77 invariant + zero-key checks, all OK.

### How each test is driven

- **Test 2:** the collection account's balance is sandbox-patched to exactly
  its storage stake, so the real collection `nft_transfer` fails with
  `LackBalanceForState` (asserted from the receipt outcome). The box callback
  undoes immediately. The balance is then restored and a retry finalizes.
  No test-only hook exists in the contract.
- **Test 3:** owner (listed and unlisted), an "approved account" with
  `approval_id`, and the collection deployer key all fail `nft_transfer`,
  `nft_transfer_call`, `nft_approve`, `nft_revoke`, `nft_revoke_all`.
- **Test 4:** (1) box STATE patched with a current lock, buyer NEAR added to
  the box balance, collection still showing the seller → early rescue refused
  (`ERR_UNDO_DELAY_NOT_ELAPSED`); `pay_hire` refused while locked;
  `fast_forward(34)` → rescue undoes. (3) A real sale lands, then the box is
  patched back to its pre-callback locked state (collection shows buyer) →
  rescue finalizes immediately, seller credited price + accrued lease payments.
  (4) With a newer patched lock `sid3`, rescues for older ids fail as stale
  and state is byte-identical; then `sid3` is undone after the delay and a
  real new buy finalizes.
- **Test 5:** inspection valid for 120 s, `fast_forward` until block time is
  past it.

### Limitation (honest note)

A *late `on_nft_transfer` delivered by the box itself* cannot be produced on
demand in sandbox: the callback is `#[private]` and the box has no keys. The
sandbox tests show that injected callbacks are refused (`Method
on_nft_transfer is private`) and that stale rescues leave state byte-identical;
the stale-`sale_id` no-op branch of the callback itself is covered by the unit
tests `stale_callback_is_noop` and `callback_success_finalizes_and_failure_undoes`.

The first-ever full run (`logs/sandbox-run1.log`, not committed) took about
two hours and had harness-only failures (an error-string needle and a
balance check confused by late gas refunds); both were fixed in the harness,
not the contracts. Every run since completes in about 80 s.

## `SALE_UNDO_DELAY_BLOCKS`

**Unchanged at 32.** No raise and no lower. In sandbox, transfer + callback
land within the `buy` transaction's own few blocks. Testnet re-measurement is
still required on testnet day one, per spec 2.1.
