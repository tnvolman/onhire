# Onhire

**Container leasing where each container is an NFT that is also its own NEAR account.**

> **Testnet prototype. Unaudited. Not for real funds. Nothing has been deployed to mainnet.**

Native NEAR only (no tokens).

The author also entered openLID, an independent project, in this hackathon: https://github.com/tnvolman/openlid

## What it is

Shipping-container leasing runs on paper and trust: who owns the box today,
whether the lease payment went to the right owner after a sale, whether the repair
reserve is really there, and whether the box was inspected. Onhire puts each
container's rules inside the container itself.

- Each container is an NFT in the Onhire collection **and** a keyless NEAR
  sub-account (`box-0042.<collection>`) running the box contract.
- A lessee makes lease payments in native NEAR into the box. **15%** of every payment is
  locked in a repair reserve that the owner can never withdraw.
- A **1% protocol fee** is taken on every lease payment and every sale, out of
  the owner's (seller's) proceeds only, never out of the reserve. Lease payment of 100:
  reserve 15, fee 1, owner 84. Sale of 100: fee 1, seller 99.
- The box refuses a new lease once its inspection has lapsed. Only a separate
  inspector account (standing in for a certified surveyor or depot
  inspector) can attest; the owner can never attest their own box.
- The owner can list the box; a buyer can buy it **mid-lease**. The lease, the
  reserve and future lease payments move with the box. The seller's sale price and
  already-earned lease payments become a pull balance.

How NEAR fits: NFT-as-account (each box is a real account holding its own
money), native NEAR for lease payments and sale, contract-enforced rules, NEP-591 global
contract code so every box runs identical immutable code, and asynchronous
cross-contract calls with an explicit, testable recovery path.

## Contracts

| Contract | Path | Role |
| --- | --- | --- |
| Collection | `contracts/collection` | NEP-171 registry (+ NEP-177 metadata, NEP-181 enumeration) and factory. Mints keyless box accounts. Every transfer path is gated to the token's box account. Approvals are disabled. |
| Box (item) | `contracts/item` | Lease, reserve, inspection, listing, sale lock, pull balances. Holds the NEAR. |

## How control passes on purchase

1. Owner `list(price)` on the box (1 yocto confirmation).
2. Buyer `buy()` with **exactly** `price` attached. Any other amount is rejected and nothing locks.
3. `buy` moves the payment into `locked_buyer_payments[sale_id]`, writes the lock
   `{sale_id, seller, buyer, price, started_at_block}`, and calls the
   collection's `nft_transfer` **from the box account** (the only account the
   collection accepts for that token), with a callback.
4. `on_nft_transfer(sale_id)`: on success **finalize** (buyer becomes owner;
   seller credited `99% of price + accrued lease payments` as a pull balance, 1% fee
   sent to the fee account; lease, reserve, inspection and deposit
   untouched). On failure **undo immediately** (buyer
   credited the price; nothing else changes; listing stays).
5. If the callback never lands, anyone may call `rescue_sale(sale_id)`. It is
   the only ordinary call that reads the collection (`owner_of`):
   - collection shows buyer → finalize (no delay needed);
   - collection shows seller and `block >= started_at_block + SALE_UNDO_DELAY_BLOCKS` → undo;
   - collection shows seller and the delay has not elapsed → refused, lock stays;
   - anyone else → refused, lock stays.
6. Every callback or rescue carries `sale_id`. A stale `sale_id` is a no-op.
7. Seller (after finalize) or buyer (after undo) calls `withdraw()`. Pull only.
   A failed withdrawal can never unsell the box.

While a sale is locked, `pay_hire`, `list`, `cancel_list`, `start_lease` and
`off_hire` are refused. After a finalize the next lease payment goes to the buyer;
after an undo, to the original owner.

## Do X → see Y

All seven run in order, in a local near-workspaces sandbox, with the rule-14
invariant and **zero access keys on the box** asserted after every step.
One command (builds both contracts first; see
[Prerequisites and build first](#prerequisites-and-build-first)):

```
scripts/test-sandbox.sh
```

| Test | Do | See |
| --- | --- | --- |
| 1. Mid-lease sale (the demo) | Mint, inspector attests, owner leases to lessee; lessee pays 1 NEAR; owner lists 10 NEAR; buyer buys | Buyer owns the box on both box and collection; seller pending = 10.74 NEAR (99% of price + 84% of the lease payment); fee account got exactly 1% of each lease payment and of the price; lease and reserve unchanged. The next lease payment accrues to the buyer. Seller withdraws exactly 10.74 NEAR and nothing more; buyer withdraws only post-sale lease payments; reserve stays. |
| 2. Failed transfer | Collection balance is sandbox-patched to exactly its storage stake, so its `nft_transfer` fails with `LackBalanceForState`; buyer buys | Callback undoes at once: seller still owner on box and collection, listing kept, lock cleared, buyer withdraws the price, seller credited nothing. Retry after restore succeeds. |
| 3. Direct transfer blocked | Owner (listed and unlisted), a would-be approved account, and the collection deployer call `nft_transfer`, `nft_transfer_call`, `nft_approve`, `nft_revoke`, `nft_revoke_all` | Every call fails (`ERR_ONLY_BOX` / `ERR_APPROVALS_DISABLED`). Owner unchanged; no pending changes. |
| 4. Stuck lock | Sandbox state patch writes a current lock with no transfer in flight; then a lock where the collection already shows the buyer | Early rescue refused; after fast-forwarding 32+ blocks rescue undoes and the buyer withdraws. With the collection already showing the buyer, rescue finalizes immediately. Stale `sale_id` rescues and injected callbacks change nothing, including while a newer lock is held. |
| 5. Lapsed inspection | Short attestation; lease starts; fast-forward past it | Active lease still takes `pay_hire`; after `off_hire`, `start_lease` fails; owner/buyer/lessee attest fails; inspector re-attests and `start_lease` works. |
| 6. Reserve | Lessee pays an odd X | Reserve grows by exactly `X*1500/10000`; fee account gets exactly `X*100/10000`; owner's withdraw takes only lease payments; no reserve, key, upgrade or delete method exists; reserve unchanged. |
| 7. Protocol fee | Lease payment of 100 yocto; sale at 100 yocto; then state-patched hypothetical bps that leave the owner less than the fee | 15 / 1 / 84 and 1 / 99 exactly, fee account +1 yocto each; reserve untouched by the sale; `pay_hire` and `buy` revert with `ERR_FEE_EXCEEDS_OWNER_SHARE` and change nothing. |

Every successful lease payment and sale in tests 1–7 also asserts the fee account's
balance grew by exactly 1% and that the reserve is exactly 15%. Undone sales
charge no fee.

Unit tests (`cargo test -p onhire-item`) additionally drive the private
callbacks directly with stale and current `sale_id`s.

The same seven also run on NEAR testnet against the live fee-enabled
deployment `v2.onhire.testnet` (`scripts/test-testnet.sh`; commands, accounts,
explorer links and results in `docs/FEE-PROOF.md`, with the earlier pre-fee
run of tests 1–6 in `docs/TESTNET-PROOF.md`). On testnet, test 2 forces the real collection
transfer to fail by having the collection (deployer key) send away its free
balance; test 4 cannot create a *stuck* lock (no state patching on a live
chain), so it races `rescue_sale` against real in-flight sales instead. The
stuck-lock steps 4.1–4.3 are proven in sandbox only.

## Threat model (short)

| Attacker goal | Why it fails |
| --- | --- |
| Owner calls collection `nft_transfer` and skips `buy` | Collection accepts transfers only from the token's box account (test 3) |
| Approval / `nft_transfer_call` / marketplace bypass | Approvals always fail; `nft_transfer_call` is gated and disabled (test 3) |
| Buyer charged but ownership never moves | Failed transfer undoes; buyer withdraws (test 2) |
| Transfer succeeds, callback never runs | `rescue_sale` finalizes when the collection shows the buyer (test 4) |
| Rescue undoes while the transfer is in flight | Undo only after `SALE_UNDO_DELAY_BLOCKS`; finalize whenever the collection shows the buyer (test 4) |
| Late callback after undo or a newer sale | Stale `sale_id` is a no-op (test 4, unit tests) |
| Lease payment credited to the wrong owner mid-sale | `pay_hire` refused while locked (test 4) |
| Owner attests their own box | Only the inspector, who can never be the owner (buyer ≠ inspector is enforced) (test 5) |
| Owner withdraws the reserve | No such method; `withdraw` never touches it (tests 1, 6) |
| Key holder drains the box or swaps code | Box accounts have zero keys; no add-key, upgrade, deploy or delete method (every step) |
| Seller's `withdraw` fails and reverses the sale | Pull balances, separate call (test 1) |
| Fee eats into the repair reserve | Fee is computed after the reserve split and taken from the owner's share; reverts (`ERR_FEE_EXCEEDS_OWNER_SHARE`) rather than split partially (tests 1, 6, 7) |

## Constants

| Name | Value | Notes |
| --- | --- | --- |
| `RESERVE_BPS` | `1500` | 15% of each lease payment goes to the reserve |
| `SALE_UNDO_DELAY_BLOCKS` | `32` | Undo only at `lock.started_at_block + 32` or later |
| `MIN_RESERVE` | `0` | `start_lease` fails if `reserve < MIN_RESERVE` |
| `BPS_DENOMINATOR` | `10_000` | |
| `FEE_BPS` | `100` | 1% protocol fee on each lease payment and each sale, from the owner's/seller's proceeds, rounded down |

### Fee recipient

The collection takes `fee_account` in `new(minter, fee_account)` and writes it
into every box it mints; a box's `fee_account` can never change. Each fee is
sent straight to it in the same call that earns it (`pay_hire`, or the sale's
finalize), so it is never a liability of the box and rule 14 is checked with
the fee already out. The testnet deploy (`v2.onhire.testnet`) uses the
throwaway **`onhire-fee-n12.testnet`**; see `docs/FEE-PROOF.md`. Nothing has
been deployed to mainnet.

Why 32 blocks is long enough: the `nft_transfer` receipt is created in the
same receipt as `buy` and executes on the collection in the next block or
shortly after (NEAR delivers cross-shard receipts within a block or two under
normal load; a delayed receipt still lands well inside 32 blocks, roughly
20–40 seconds). In sandbox the transfer and its callback complete within the
`buy` transaction's few blocks every time. If the transfer has not landed by
then, the collection can only show the seller or the buyer: rescue finalizes
for the buyer at any time and only undoes for the seller after the delay.
**Measured on testnet (2026-11-12, protocol 88):** in every recorded `buy`
(see `docs/TESTNET-PROOF.md`), the collection `nft_transfer` executed at
`lock + 1` block and the box callback at `lock + 2` blocks, for successful
and failed transfers alike, so 32 blocks leaves a margin of 30 blocks.
`rescue_sale` calls fired during real in-flight sales never undid
anything. **Unchanged at 32.** It will never be lowered below 32.

## Keys and upgrade disclosure

- **Box accounts** are created by the collection with `CreateAccount` +
  `Transfer` + `UseGlobalContract(code_hash)` (or `DeployContract`) +
  `FunctionCall(new)` and **no `AddKey` action**, so they have zero access
  keys from their first block. The box contract has no add-key, upgrade,
  deploy or delete-account method. The global code is referenced by immutable
  hash, not by account, so nobody can change a box's code.
- **The collection account on testnet still has the deployer's key.** The
  deployer is also the prototype's minter. That key cannot move a box token
  (test 3) and cannot touch box funds, but it could redeploy the collection
  contract. Disclosed; acceptable for a testnet prototype only.

## Clarifications made while implementing spec v1.1

- `on_nft_transfer(sale_id)` reads `success` from the promise result rather
  than taking it as an argument.
- An owner collects lease payments with `withdraw()`: when the caller is the
  current owner and no sale is locked, accrued lease payments are swept into their payout. During
  a lock it is frozen and goes to the seller on finalize.
- `start_lease` takes the attached NEAR (≥ 1 yocto) as the lessee deposit.
  `off_hire` credits the full deposit to the lessee as a pull balance (no
  damage model in this version). A lease stays active until `off_hire`.
- `nft_transfer_call` is gated to the box and then always fails; boxes only
  use `nft_transfer`.
- Every user-facing state change asserts rule 14 plus 2,000 bytes of storage
  headroom, so storage growth can never eat into user balances.

## Live accounts

NEAR **testnet** only (throwaway accounts, free faucet NEAR):

| Role | Account |
| --- | --- |
| Collection / factory / minter, fee-enabled build (still has the deployer key) | `v2.onhire.testnet` |
| Box accounts (zero keys, global item code) | `box-NNNN.v2.onhire.testnet` |
| Protocol fee recipient (throwaway) | `onhire-fee-n12.testnet` |
| Inspector (stands in for a certified surveyor or depot inspector) | `onhire-inspector-n12.testnet` |
| Owner | `onhire-owner-n12.testnet` |
| Lessee | `onhire-lessee-n12.testnet` |
| Buyer | `onhire-buyer-n12.testnet` |

Use `v2.onhire.testnet`. The older pre-fee deployment `onhire.testnet`
(boxes `box-NNNN.onhire.testnet`, proof in `docs/TESTNET-PROOF.md`) is kept
only as a historical record.

On `v2.onhire.testnet` the item code is a NEP-591 global contract. The hashes
of the deployed testnet build (item `623920fe…`, collection `6d855a51…`, in
full with deploy transactions in `docs/FEE-PROOF.md`) identify that build on
chain. They were built with `non-reproducible-wasm`, so a local build is not
expected to reproduce them byte for byte.

The source in this commit changed after that proven build in two places
only: the unused `MAINNET_FEE_ACCOUNT` constant was removed from the
collection, and the NFT metadata description text was reworded. Contract
method logic is unchanged.

## Prerequisites and build first

A fresh clone contains no wasm. The sandbox tests load the built contracts,
so **`cargo test --workspace` on its own will fail the sandbox tests on a
fresh clone until both contracts have been built.**

Prerequisites:

- Rust with the `wasm32-unknown-unknown` target
  (`rustup target add wasm32-unknown-unknown`). This repo was built with
  Rust 1.99.1.
- `cargo-near` 0.23.0.
- OpenSSL headers (the test harness links system OpenSSL). If `pkg-config`
  is missing, set:

  ```
  export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
  export OPENSSL_INCLUDE_DIR=/usr/include
  ```

Then either run the one command, which builds both contracts, runs the unit
tests, then the seven sandbox tests:

```
scripts/test-sandbox.sh
```

or build first and test by hand:

```
(cd contracts/item && cargo near build non-reproducible-wasm --no-abi)
(cd contracts/collection && cargo near build non-reproducible-wasm --no-abi)
cargo test --workspace -- --test-threads=1
```

## Libraries and toolchain

Rust 1.99.1, `near-sdk` 5.30.1, `near-contract-standards` 5.30.1,
`cargo-near` 0.23.0 (`cargo near build non-reproducible-wasm`),
`near-workspaces` 0.24.0, `tokio`, `anyhow`, `serde_json`.
The test harness needs OpenSSL headers; `scripts/test-sandbox.sh` points
`openssl-sys` at them when `pkg-config` is absent.

## Credit and license

Built by Michael McClanahan (@tnvolman). Proprietary, all rights reserved.
A limited grant lets the official NEAR HACKATHON organizers, judges, and
screening reviewers copy, build, review and run the code solely to evaluate
this entry, through the official results announcement. See `LICENSE` for the
copyright holder and the exact terms.
