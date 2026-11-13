# Onhire — NEAR testnet proof (tests 1–6, before the protocol fee)

Commit IDs in this document refer to earlier development history, which is kept private. Spec `docs/ONHIRE-SPEC-v1.1.md`. Run 2026-11-12
(UTC) on **NEAR testnet only** (chain id `testnet`, protocol 88, nearcore
2.14.1). Native test NEAR only, from the public faucet. No mainnet, no real
funds, no USDC.

This run predates the 1% protocol fee. There is no fee in this proof. The fee was added later and is proven in `docs/FEE-PROOF.md`.

## Result

Final proof run **B**: one run of all six tests in order, on one binary built
from commit `7057b06`. Log: `logs/testnet-proof.log` (not committed).

| Test | Testnet result | Checked steps | Box |
| --- | --- | --- | --- |
| 1 — Mid-lease sale | **PASS** | 13 | `box-0018.onhire.testnet` |
| 2 — Failed collection transfer undoes the sale | **PASS** | 10 | `box-0019.onhire.testnet` |
| 3 — Direct transfer is blocked | **PASS** | 16 | `box-0021.onhire.testnet` |
| 4 — Stuck lock / rescue | **PASS on what a live chain can show** (see the test 4 section). Steps 4.1–4.3 (a *stuck* lock) need state patching and are proven in sandbox only. | 20 | `box-0022.onhire.testnet` |
| 5 — Lapsed inspection | **PASS** | 11 | `box-0023.onhire.testnet` |
| 6 — Reserve cut; owner cannot take it | **PASS** | 7 | `box-0024.onhire.testnet` |

`scripts/test-testnet.sh` exited 0. All 77 checked steps printed
`INVARIANT OK, ZERO KEYS`. After every step the runner checks rule 14 in two
ways, the same as the sandbox harness: from the box's `get_invariant_parts`,
and separately from the RPC account view
(`balance >= storage_usage*1e19 + reserve + pending + deposits + locked`).
It also asserts the box account has **zero access keys**.

The nonce audit (`scripts/testnet/audit-nonces.sh`, output in
`logs/testnet-audit.log`) matches every signer's on-chain access-key nonce
against the transactions the runner recorded. All accounts show `OK`, so no
transaction signed with these keys is missing from the record.

## Commands

```bash
# 0. build both contracts (unchanged since the sandbox proof)
(cd contracts/item && cargo near build non-reproducible-wasm --no-abi)
(cd contracts/collection && cargo near build non-reproducible-wasm --no-abi)

# 1. throwaway testnet accounts from the public faucet (10 test NEAR each);
#    keys go to ~/.onhire-testnet (outside the repo)
scripts/testnet/create-accounts.sh n12

# 2. one time: fold two donor accounts into the collection, deploy, top up
export NEAR_TESTNET_RPC=https://rpc.testnet.fastnear.com
cargo run -p onhire-tests --bin onhire-testnet -- fund     # delete donors -> onhire.testnet
cargo run -p onhire-tests --bin onhire-testnet -- deploy   # collection wasm, new, set_item_code(global=true)
cargo run -p onhire-tests --bin onhire-testnet -- topup    # 6 NEAR buyer2 -> collection (before run B)

# 3. the six tests, in order (each mints its own fresh box)
scripts/test-testnet.sh            # = onhire-testnet t1 … t6

# 4. checks
scripts/testnet/audit-nonces.sh            # on-chain nonces vs recorded txs
scripts/testnet/proof-links.sh <pid> …     # markdown tx list (appendix below)
```

The first `deploy` and the first test runs used the default
`https://rpc.testnet.near.org`, which rate-limited us mid-test. Every run
after that, including run B, used `https://rpc.testnet.fastnear.com`. The
runner refuses any account id that does not end in `.testnet`, and its
worker is `near_workspaces::testnet()`. It contains no mainnet configuration.

## Accounts (all throwaway, testnet)

| Role | Account | Created |
| --- | --- | --- |
| Collection / factory / minter (keeps its deployer key, as disclosed in the README) | [`onhire.testnet`](https://testnet.nearblocks.io/address/onhire.testnet) | [6m6GqBwJ…](https://testnet.nearblocks.io/txns/6m6GqBwJzQSHfq7T5j19cuHZ61QPxhD5dxKKgifVHv3x) |
| Owner | `onhire-owner-n12.testnet` | [8gbjQDPD…](https://testnet.nearblocks.io/txns/8gbjQDPDdZrq84j81MRZimGsSgtuP6FwEjC1HqQhEzmF) |
| Lessee | `onhire-lessee-n12.testnet` | [2ZnR6AZr…](https://testnet.nearblocks.io/txns/2ZnR6AZrqHr5WeQdBmV53YJt4AoCJqqJXk9T3Vrq6bDj) |
| Buyer | `onhire-buyer-n12.testnet` | [Ab9ByM8z…](https://testnet.nearblocks.io/txns/Ab9ByM8zGNDFaFAwLtTtWvE1iANA8UuVSEvCXT1FtX72) |
| Inspector (stands in for a certified surveyor / depot inspector) | `onhire-inspector-n12.testnet` | [F742SWcQ…](https://testnet.nearblocks.io/txns/F742SWcQiVPKQqdz7cdd3sjmcSGeEGKU47jnZwLzCKno) |
| Outsider (rescue caller; parent of the fresh buyers) | `onhire-outsider-n12.testnet` | [4XwbjfoL…](https://testnet.nearblocks.io/txns/4XwbjfoLQiTvkSKo73BiCU6QQwErF85sGxUzkBjvZwBT) |
| Racer (second rescue caller; drain sink in test 4) | `onhire-racer1-n12.testnet` | [28sSagSP…](https://testnet.nearblocks.io/txns/28sSagSPmutRY8RCKn25EACHKn1BU8GwEZ6L9xBzpnSi) |
| buyer2 (test 2 buyer in early runs; later the top-up donor) | `onhire-buyer2-n12.testnet` | [2DTo2zqQ…](https://testnet.nearblocks.io/txns/2DTo2zqQFR8fkafqPh91DjfDjQ2D7283hj1c5tf5F4Vx) |
| Fresh buyer, run B test 2 (sub-account, never owned a token) | `t2b1794479639.onhire-outsider-n12.testnet` | by the outsider (appendix) |
| Fresh buyer, run B test 4 round F | `t4b1794479881.onhire-outsider-n12.testnet` | by the outsider (appendix) |
| Donor, deleted into the collection | `onhire-racer2-n12.testnet`, `onhire-probe-1794476160.testnet` | faucet |

Owner, lessee, buyer and inspector are four separate accounts. The faucet
helper allows 10 accounts per 12 hours, and the tenth creation
(`onhire-racer3-n12`) was refused with `Rate limit exceeded, retry in 12
hours`. So there are no dedicated funder accounts. Instead the probe and
`racer2` were deleted into the collection, and later fresh buyers were made
as sub-accounts of the outsider, which needs no faucet.

## Deploy

| Step | Tx |
| --- | --- |
| Donor `onhire-probe-1794476160.testnet` deleted → `onhire.testnet` | [8f1xzFTf…](https://testnet.nearblocks.io/txns/8f1xzFTfWPV8VJ1fwMcamEwcebQ68ZnR88cB6WK9qhtQ) |
| Donor `onhire-racer2-n12.testnet` deleted → `onhire.testnet` (collection then 29.999 NEAR) | [2srKgzsF…](https://testnet.nearblocks.io/txns/2srKgzsFsV2nLH6nf9CUb6cptNCe9xQ28NAN2sqLYZq8) |
| Deploy `onhire_collection.wasm` to `onhire.testnet` | [2mcxXRF4…](https://testnet.nearblocks.io/txns/2mcxXRF4fJrFujn5yRXGQJQST6MnAjUfdVgjh3G4w9Aj) |
| `new({minter: onhire.testnet})` | [5DjqXEkp…](https://testnet.nearblocks.io/txns/5DjqXEkpexSLpPKr41TXJzNSiTL4sTqah4AQUbJcpFuk) |
| `set_item_code(code, global=true)`: NEP-591 global contract deploy, burned **17.626566 NEAR** for 176,231 bytes | [4XW7ut1j…](https://testnet.nearblocks.io/txns/4XW7ut1jcDJiohB7U3unofwF3TnkXcD4Y3z6UZwHCMiY) |
| Top-up 6 NEAR `onhire-buyer2-n12.testnet` → `onhire.testnet` (before run B) | [7U2vs67m…](https://testnet.nearblocks.io/txns/7U2vs67m6rKenagAUiK1P2rYHRaJjDvYGtZ3poB7a4QS) |

The global contract deployed on day one, so the local-deploy fallback was
**not** used on testnet. Every box account reports
`global_contract_hash = Yp7FS19k5ADj2V9sKxrF59rEDqgKgNVv41EaXegs29e`, with
`code_hash = 1111…` and zero keys (re-checked on chain for boxes
0018–0024 after run B).

### Wasm hashes (sha256)

| File | sha256 | On chain |
| --- | --- | --- |
| `onhire_item.wasm` (176,231 B) | `0826451ccbd0d4512f4d02f04a675c0a89bbf39258da66ae5589a69c07987f79` | Global code hash: `get_item_code_info` → `CCZFHMvQ…` (base64) and box `global_contract_hash` `Yp7FS19k…` (base58). Both decode to this hash. |
| `onhire_collection.wasm` (221,209 B) | `3de6249e3e1f14877a9b89b2656f3e26281c8bd4dc19d3be829ea5c5743e6c88` | `onhire.testnet` `code_hash` `5AdQjfXh…` (base58) decodes to this hash. |

These are byte-identical to the wasm in `docs/SANDBOX-PROOF.md`. The
contract source was unchanged for this run.

## How each test runs on testnet

The test bodies mirror `tests/tests/sandbox.rs`, with smaller amounts
(prices 0.5–1 NEAR, lease payments 0.1–0.2 NEAR). Each run mints its own box with a
0.5 NEAR deposit. The only differences come from the sandbox-only tools
(state patching and fast-forward):

- **Test 1 — PASS.** Same as sandbox. The buyer's `buy` of 1 NEAR
  [6TKMmgYC…](https://testnet.nearblocks.io/txns/6TKMmgYCNhhBmv9pDZrBXS9D91BbdsWdcZdKkK7SjEKU)
  finalized through the callback. The seller withdrew exactly 1.085 NEAR
  (price plus 85% of the 0.1 NEAR pre-sale lease payment)
  [BsQyMzgi…](https://testnet.nearblocks.io/txns/BsQyMzgiGjbJWi2jGDcpP8EFeLnSytdQagKBXnSKTEQC).
  The seller's second withdraw was refused
  [2MXaDHhN…](https://testnet.nearblocks.io/txns/2MXaDHhNvjB23QKpJk8GbxQTEWyyUKc1HhDPPE4vJ3DD).
  The buyer withdrew only the post-sale lease payment
  [HPgjnWSB…](https://testnet.nearblocks.io/txns/HPgjnWSB5JaQEb1HS3khgcUcXDf62AJc1ek72LHyTMUK),
  and the reserve was unchanged.
- **Test 2 — PASS, and the transfer failure is real.** Sandbox patched the
  collection balance. On testnet, the collection account uses its own
  (disclosed) deployer key to send its free balance away until only its
  storage stake plus about 25 bytes is left. Drain txs:
  [EZfdJq23…](https://testnet.nearblocks.io/txns/EZfdJq23hV1B3sQpn1SuGbXpZmgPL6XDZCbMJVRYpH6j)
  and [9vYxLGvi…](https://testnet.nearblocks.io/txns/9vYxLGviuPKdY3w2E6viir5BsqXB9idWH29PdbejERKk).
  One overshooting drain,
  [3E1A3Mp3…](https://testnet.nearblocks.io/txns/3E1A3Mp3EX4pypZ85cTx2qpS3WFm92ymAWmAabusEt2Q),
  was rejected at validation and moved nothing. A fresh buyer that has never
  owned a token then buys
  [9HiDGNQc…](https://testnet.nearblocks.io/txns/9HiDGNQcFwEwibNiEtT43yut1z3GPqzCGxTwZSYyPdqX).
  The collection's `nft_transfer` receipt fails with `LackBalanceForState`
  (asserted from the receipt), and the box callback returns `undone`. The
  balance is restored
  [HLYMEjEE…](https://testnet.nearblocks.io/txns/HLYMEjEEM6Hq3NgRaMUF8AUdgb861dJEbcBdQ6eqRnLs).
  Checks: seller is still owner on box and collection, listing kept, reserve,
  lease and accrued lease payments unchanged, lock cleared, seller credited nothing. The
  buyer withdraws exactly the price
  [ApyJQqhd…](https://testnet.nearblocks.io/txns/ApyJQqhdYNj36zgxZer7eNHzhpv5pgsjXJsmZxkEwFa3),
  and the retry finalizes
  [3XJXQWye…](https://testnet.nearblocks.io/txns/3XJXQWyeKx1fedUfFeEwxH4RUprLsSGBRgqFpHAvm6iN).
  No test hook exists in the contracts.
- **Test 3 — PASS.** Same as sandbox. With the box unlisted and then listed,
  each of these was called and refused (`ERR_ONLY_BOX` /
  `ERR_APPROVALS_DISABLED`): the owner's `nft_transfer`
  [FvRpp3y2…](https://testnet.nearblocks.io/txns/FvRpp3y24e2DR824Qm7EQWaTs1WRKTMaJyqtz2ZtJKr),
  `nft_transfer_call`
  [9pqLhLVR…](https://testnet.nearblocks.io/txns/9pqLhLVRnnzP16M9xKrpGYtv7RzKhDQDNpCnNX3LJa6N),
  `nft_approve`
  [6pd5XXXE…](https://testnet.nearblocks.io/txns/6pd5XXXE8cPrEQobq8WQxuPcmRSqDSarxsREWDVueFvC),
  `nft_revoke`, `nft_revoke_all`, an approved-account style transfer with
  `approval_id`
  [5dY394sU…](https://testnet.nearblocks.io/txns/5dY394sU2W4AkwWVS9taDPDj996kQNnaMhbqUGrHBcBv),
  and the **collection deployer key's** own `nft_transfer`
  [DjogmXDo…](https://testnet.nearblocks.io/txns/DjogmXDoWWY4bAnMRcXH66rvSjashYw36EiWAhWNqwKC).
  Owner and pending balances were unchanged.
- **Test 4:** see below.
- **Test 5 — PASS, using real time.** Sandbox used fast-forward. Here the
  inspector attests a 45 s validity
  [AHhs1GAU…](https://testnet.nearblocks.io/txns/AHhs1GAUqS7t1ngUDE8aJCBKwCwgRuCEyJnZnYqLcoCR),
  the lease starts
  [AWRRJmq5…](https://testnet.nearblocks.io/txns/AWRRJmq5B7VvDKa3jGp998imorAw4taSF898fVeBB2sj),
  and the runner waits until the block timestamp is past `valid_until`.
  `pay_hire` still works
  [FHECUfiv…](https://testnet.nearblocks.io/txns/FHECUfiv2M2tYJ8ukVXG6AzDjZGi3UmxMpsEQRu9fnVG).
  After `off_hire` and the deposit withdrawal, `start_lease` is refused with
  `ERR_INSPECTION_LAPSED`
  [2e1KyQnH…](https://testnet.nearblocks.io/txns/2e1KyQnHGB9v6GprQwyygL2W6qHRBaQmDoa4PmHfyFaC).
  The owner's, buyer's and lessee's attests are refused
  ([2rt9hLWN…](https://testnet.nearblocks.io/txns/2rt9hLWNpJzNpXmvye2fsHTMGv1Y84cR4pBgSPLjdbXm),
  [7bMNjxQ8…](https://testnet.nearblocks.io/txns/7bMNjxQ8vTecE5FrU6XvayV3qztZhcCAAgKT4awdCKD2),
  [FLGwi31h…](https://testnet.nearblocks.io/txns/FLGwi31hJV2NZXqtbyV2KweViCyfjegSoQQTrk5FRjKw)).
  The inspector re-attests
  [12WPX2HS…](https://testnet.nearblocks.io/txns/12WPX2HSjozprKEVeUb2i6voiti6KFg98v5MbtZoJttX),
  and `start_lease` succeeds
  [Bkv7Wmbe…](https://testnet.nearblocks.io/txns/Bkv7WmbePnu366zSx8JMPsjNY1LXpcrUM8cDNy9WfVi9).
- **Test 6 — PASS.** Same as sandbox. The lessee pays X =
  123456789012345678901234 yocto
  [235xmRBF…](https://testnet.nearblocks.io/txns/235xmRBFwBoos2bhBi7cKChwkHa73Wt9JneZ4aV9wExT).
  The reserve grows by exactly 18518518351851851835185 (X·1500/10000), and
  104938270660493827066049 accrues to the owner. Each of these 10 methods is
  called by the owner, the lessee and the buyer, and all 30 calls fail with
  `MethodNotFound`: `withdraw_reserve`, `reserve_withdraw`, `claim_reserve`,
  `add_key`, `add_full_access_key`, `delete_account`, `update`, `upgrade`,
  `deploy`, `migrate`. Example:
  [Gj2eo8ed…](https://testnet.nearblocks.io/txns/Gj2eo8edAgX7V6GRywJb7KHX7fkRAL8TBJ79EKpFXz3p).
  The owner's withdraw takes only the lease payments
  [7UD5FNa6…](https://testnet.nearblocks.io/txns/7UD5FNa63wrC7Ag5a2URvchB8M1YazQWdcfEHJpd8uRx),
  then every party's withdraw is refused with `ERR_NOTHING_TO_WITHDRAW`. The
  reserve is unchanged.

### Test 4 on testnet: what it does and does not prove

The spec's test 4 sets up a *stuck* lock by **sandbox state patching**
(4.1–4.3): a current lock with no transfer in flight, or a lock where the
collection already shows the buyer. A live chain has no
`sandbox_patch_state`, and the box account has no keys. On an unmodified
deployment the box's own callback always lands (see the timing section
below), so a stuck lock cannot be created honestly on testnet. **Steps
4.1–4.3 are not claimed on testnet.** They remain proven in sandbox
(`docs/SANDBOX-PROOF.md`, test 4: 20 steps PASS, global and local code modes).

What testnet test 4 does prove, against real in-flight sales on
`box-0022.onhire.testnet`:

1. **Round F, failed transfer with concurrent rescues.** The collection is
   drained as in test 2. A fresh buyer buys
   ([BShUEd5d…](https://testnet.nearblocks.io/txns/BShUEd5dJYEtfXWjMyzs4Ww6yi3CFFYy1cPsrfaUyYKY)),
   and four accounts fire `rescue_sale(sid)` 150–1200 ms after submission.
   Result: the callback undid the sale exactly once. The rescues were
   refused as stale or returned `noop`. The buyer was refunded exactly once
   and withdrew the price
   [3RFrbvng…](https://testnet.nearblocks.io/txns/3RFrbvngFbfEA9em6ukyRwXBPB5Vii6FJ6hAMEmft7kt).
   The seller was credited nothing. After the undo, rescue is stale and an
   injected `on_nft_transfer` is refused as private, with box STATE
   byte-identical before and after.
2. **Rounds S0–S3, four successful sales** alternating owner ⇄ buyer
   (e.g. [BU4AZ4C9…](https://testnet.nearblocks.io/txns/BU4AZ4C9tAeTpvtSfaziUnXRhUdA5z4Jo8UPXWQZ5a12)),
   each raced by four `rescue_sale` calls. Every sale finalized exactly once.
   The seller got `price + accrued lease payments`, the buyer got no refund, and no
   rescue ever undid or double-resolved a sale.
3. **Rescue outcomes in run B** (20 in-flight rescues): 16 returned `noop`
   (the rescue's `on_rescue_owner` arrived after the callback had already
   resolved the lock), and 4 were refused `ERR_STALE_SALE_ID` (they reached
   the box before `buy` created the lock). Across all testnet runs (84
   in-flight rescues: 69 `noop`, 15 stale), no rescue ever got as far as the
   undo-delay check, because its `owner_of` reply always arrives after the
   callback.
4. **4.4, stale ids.** `rescue_sale` for every earlier `sale_id` is refused
   `ERR_STALE_SALE_ID` (e.g.
   [3zVSKEvF…](https://testnet.nearblocks.io/txns/3zVSKEvFX1UXCFuQEHxKZom7xKgAVSJWGyLv37ANA5ck)).
   Injected `on_nft_transfer` and `on_rescue_owner` are refused as private.
   Box STATE is byte-identical before and after.
5. A lease payment made after the last sale accrues to the current owner, and both
   parties withdraw exactly their pending balances.

## `SALE_UNDO_DELAY_BLOCKS`: re-measured on testnet

**Unchanged at 32.** Not raised, not lowered.

The runner reads block heights from each `buy`'s receipt outcomes: the lock
block from the `sale_locked` event's `started_at_block`, the block of the
collection's `nft_transfer` receipt, and the block of the box callback.

| Sample | `nft_transfer` lands | callback lands | Outcomes |
| --- | --- | --- | --- |
| Run B (8 buys: tests 1, 2, 4) | lock **+1** every time | lock **+2** every time | 6 finalized, 2 undone (failed transfer) |
| All testnet runs (28 buys, every attempt) | lock **+1** every time | lock **+2** every time | 23 finalized, 5 undone |

Rescue receipts fired during in-flight sales landed between lock+0 and
lock+4. Every one resolved as stale or no-op, because the callback had
already landed at lock+2. Testnet blocks were about 0.6 s apart, so 32
blocks is about 19 s, against an observed worst case of 2 blocks. That is a
margin of 30 blocks, so no raise is needed. Under heavy congestion a delayed
receipt could land later. The undo path stays safe in that case: rescue only
undoes when the collection *still shows the seller* and the lock is at least
32 blocks old, and it finalizes for the buyer at any time.

## What went wrong along the way

- **Run 1 (test 1, `box-0001`):** `rpc.testnet.near.org` returned "this
  client has exceeded the rate limit" after step 11. All steps before that
  were green. Fix: use the FastNEAR testnet RPC.
- **Run 2 (test 1, `box-0002`): duplicate transactions.** `near-workspaces`'
  `transact()` uses `broadcast_tx_commit` and re-signs and resends with a new
  nonce when the RPC returns an error, even if the first transaction already
  landed. A chunk scan found `attest_inspection`
  [6C8a3Yyy…](https://testnet.nearblocks.io/txns/6C8a3YyyNY8pCuj59kAMpWfi2YdqwXY64TUDAvBzTgQ4)
  and `start_lease`
  [CVv1i7TZ…](https://testnet.nearblocks.io/txns/CVv1i7TZ8jrzTZ74QJ2se7Td3TJ3KKkLTzjArkWek2Jz)
  on chain alongside the recorded resends, so the recorded `start_lease`
  failed with `ERR_LEASE_ACTIVE`. Nothing was lost; the box stayed solvent.
  Fix: the runner now broadcasts once (`broadcast_tx_async`) and polls, a
  single-instance lock was added, and `audit-nonces.sh` cross-checks every
  signer. Both duplicates are in `logs/testnet-records.jsonl`, marked
  `"source":"audit"`.
- **First version of test 4:** it required catching a rescue while "lock
  current, collection shows seller". Across 21 races that window never came
  up, for the reason above, so it was replaced by the assertions described in
  the test 4 section.
- **Run A (commit `2575cc9`, log `logs/testnet-proof-runA-2575cc9.log`):**
  tests 3–6 passed, but 1 and 2 failed. Test 1's `start_lease` was accepted by
  the RPC and then dropped before inclusion: it is unknown on two RPCs, and
  the owner's nonce later moved past it, so it can never land. Test 2's
  precondition was not met because `buyer2` already owned a token from the
  earlier test 2 run. Fix (`7057b06`): a dropped tx is resent once, and tests
  2 and 4 create a fresh sub-account buyer each run. Run B then passed all six.
- Two drain transfers in early runs (`52w5F5ko…`, `ENacDHQf…`) were rejected
  at validation for overshooting the storage stake. No balance moved, but the
  nonce was consumed. They are recorded so the audit balances.
- Boxes from incomplete attempts (`box-0001` rate limit, `box-0002`
  duplicate txs, `box-0007` first test 4 version, which still holds owner and
  buyer pending balances, and `box-0011`, `box-0012`, `box-0013` from run A)
  are left as they are. Every check on them up to the failure printed
  `INVARIANT OK, ZERO KEYS`. Boxes 0003–0006, 0008–0010 and 0014–0017 are
  earlier passing runs of individual tests.

## Appendix: every recorded transaction of run B

Generated with `scripts/testnet/proof-links.sh 2794240 2795156 2797535 2798560 2803325 2804582`.
"after step" is the checked-step counter at the time the tx was sent.
"refused (expected)" means the test asserted that the call fails with the
named error.

#### testnet-t1 (pid 2794240)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [B2gYe66g…](https://testnet.nearblocks.io/txns/B2gYe66gKVVt3H4dvMGjrfXkdYdS9jt38rxsgPR7dpvH) |
| 1 | `attest_inspection` → `box-0018.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [BVcvzf9K…](https://testnet.nearblocks.io/txns/BVcvzf9KjJBiW9VH5ngGPNahFwMTTXq2Sub14pbhqoQ7) |
| 2 | `start_lease` → `box-0018.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [4jWwSwJX…](https://testnet.nearblocks.io/txns/4jWwSwJXxgU4JXQ5ZRKgmyX4MPNSZbsRHspRsfFC5iuB) |
| 3 | `pay_hire` → `box-0018.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [4zSpJeeG…](https://testnet.nearblocks.io/txns/4zSpJeeGgr2VZBaknJBZBLKKmJMsNExUDp8qb7YMnwB9) |
| 4 | `list` → `box-0018.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [4A2w4ZsN…](https://testnet.nearblocks.io/txns/4A2w4ZsNwu8iP9USayRUCeXpca8nUHu1bQoSmRJ6Kas3) |
| 5 | `buy` → `box-0018.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [TgjXtb2w…](https://testnet.nearblocks.io/txns/TgjXtb2wfSGgnBZ5oBufh9NjxR2fsSA62syBEeQdi9g) |
| 6 | `buy` → `box-0018.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [6TKMmgYC…](https://testnet.nearblocks.io/txns/6TKMmgYCNhhBmv9pDZrBXS9D91BbdsWdcZdKkK7SjEKU) |
| 8 | `pay_hire` → `box-0018.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [KbD8yPXg…](https://testnet.nearblocks.io/txns/KbD8yPXgFgBhRuR2Bf9DjAqZkoY7DgjZvTju1rCiJhh) |
| 9 | `withdraw` → `box-0018.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [BsQyMzgi…](https://testnet.nearblocks.io/txns/BsQyMzgiGjbJWi2jGDcpP8EFeLnSytdQagKBXnSKTEQC) |
| 10 | `withdraw` → `box-0018.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [2MXaDHhN…](https://testnet.nearblocks.io/txns/2MXaDHhNvjB23QKpJk8GbxQTEWyyUKc1HhDPPE4vJ3DD) |
| 11 | `withdraw` → `box-0018.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [HPgjnWSB…](https://testnet.nearblocks.io/txns/HPgjnWSB5JaQEb1HS3khgcUcXDf62AJc1ek72LHyTMUK) |
| 12 | `withdraw_reserve` → `box-0018.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [2eybh3dx…](https://testnet.nearblocks.io/txns/2eybh3dx1Jjmw3rW5ZXCJ8YrihByS6EC2t7uN8tLKUuj) |

#### testnet-t2 (pid 2795156)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [9UQSqVRa…](https://testnet.nearblocks.io/txns/9UQSqVRaRvyXCYptxr9sGmjLG8E2LVaefeyAVihmbh9U) |
| 1 | `attest_inspection` → `box-0019.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [68NoFkya…](https://testnet.nearblocks.io/txns/68NoFkyaHaks4bGBH7TSyZk6Ufrs1LYJuLR4zuL1yX8R) |
| 2 | `start_lease` → `box-0019.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [4vsginC7…](https://testnet.nearblocks.io/txns/4vsginC7dedmSUHSA7WsDKjDCJfdbmv9aMYRZDS1Nvef) |
| 3 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [FFfMz4sc…](https://testnet.nearblocks.io/txns/FFfMz4sceSH7koJLMn2yMibAtozTitZQ6voRAP92PFy) |
| 4 | `pay_hire` → `box-0019.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [9wJux5MQ…](https://testnet.nearblocks.io/txns/9wJux5MQDQRvC4trFt1WF3TnNNf6cw2wWf3C2yaB1uYJ) |
| 5 | `list` → `box-0019.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [6jXFFzfm…](https://testnet.nearblocks.io/txns/6jXFFzfmwBmmXaxC8Vzr87SvMu9YZ5syd5Mkf4xzBxt4) |
| 6 | `CreateAccount` → `t2b1794479639.onhire-outsider-n12.testnet` | `onhire-outsider-n12.testnet` | ok | [FCvUnUrB…](https://testnet.nearblocks.io/txns/FCvUnUrBtUbT274kdtDkcBNgtva4m5SbuTVXB8vVUB53) |
| 6 | `Transfer` → `onhire-outsider-n12.testnet` | `onhire.testnet` | ok | [EZfdJq23…](https://testnet.nearblocks.io/txns/EZfdJq23hV1B3sQpn1SuGbXpZmgPL6XDZCbMJVRYpH6j) |
| null | `Transfer` → `onhire-outsider-n12.testnet` | `onhire.testnet` | rejected (not executed) | [3E1A3Mp3…](https://testnet.nearblocks.io/txns/3E1A3Mp3EX4pypZ85cTx2qpS3WFm92ymAWmAabusEt2Q) |
| 6 | `Transfer` → `onhire-outsider-n12.testnet` | `onhire.testnet` | ok | [9vYxLGvi…](https://testnet.nearblocks.io/txns/9vYxLGviuPKdY3w2E6viir5BsqXB9idWH29PdbejERKk) |
| 6 | `buy` → `box-0019.onhire.testnet` | `t2b1794479639.onhire-outsider-n12.testnet` | ok | [9HiDGNQc…](https://testnet.nearblocks.io/txns/9HiDGNQcFwEwibNiEtT43yut1z3GPqzCGxTwZSYyPdqX) |
| 6 | `Transfer` → `onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [HLYMEjEE…](https://testnet.nearblocks.io/txns/HLYMEjEEM6Hq3NgRaMUF8AUdgb861dJEbcBdQ6eqRnLs) |
| 8 | `withdraw` → `box-0019.onhire.testnet` | `t2b1794479639.onhire-outsider-n12.testnet` | ok | [ApyJQqhd…](https://testnet.nearblocks.io/txns/ApyJQqhdYNj36zgxZer7eNHzhpv5pgsjXJsmZxkEwFa3) |
| 9 | `buy` → `box-0019.onhire.testnet` | `t2b1794479639.onhire-outsider-n12.testnet` | ok | [3XJXQWye…](https://testnet.nearblocks.io/txns/3XJXQWyeKx1fedUfFeEwxH4RUprLsSGBRgqFpHAvm6iN) |

#### testnet-t3 (pid 2797535)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [EKVV4Mmr…](https://testnet.nearblocks.io/txns/EKVV4MmrwKg3sRKihVgUxZuo7RW5dSrB8BY7FidaCdTX) |
| 1 | `nft_transfer` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [FvRpp3y2…](https://testnet.nearblocks.io/txns/FvRpp3y24e2DR824Qm7EQWaTs1WRKTMaJyqtz2ZtJKr) |
| 2 | `nft_transfer_call` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [9pqLhLVR…](https://testnet.nearblocks.io/txns/9pqLhLVRnnzP16M9xKrpGYtv7RzKhDQDNpCnNX3LJa6N) |
| 3 | `nft_approve` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [6pd5XXXE…](https://testnet.nearblocks.io/txns/6pd5XXXE8cPrEQobq8WQxuPcmRSqDSarxsREWDVueFvC) |
| 4 | `nft_revoke` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [GUv4NNxD…](https://testnet.nearblocks.io/txns/GUv4NNxDATFpoPWxKx9RqHWcyQJtN9AWQT4FAH5TMQ95) |
| 4 | `nft_revoke_all` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [5N8iyYHj…](https://testnet.nearblocks.io/txns/5N8iyYHjS8V78y7i5Bu7z3KgBXSb9SFH1C8xbydtxLYD) |
| 5 | `nft_transfer` → `onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [5dY394sU…](https://testnet.nearblocks.io/txns/5dY394sU2W4AkwWVS9taDPDj996kQNnaMhbqUGrHBcBv) |
| 6 | `nft_transfer` → `onhire.testnet` | `onhire.testnet` | refused (expected) | [DjogmXDo…](https://testnet.nearblocks.io/txns/DjogmXDoWWY4bAnMRcXH66rvSjashYw36EiWAhWNqwKC) |
| 8 | `list` → `box-0021.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [6kRC1DHE…](https://testnet.nearblocks.io/txns/6kRC1DHEiRySsfCtBSZmU9LkhXxsTDuqPk3kr7zeW243) |
| 9 | `nft_transfer` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [2BxVZNV5…](https://testnet.nearblocks.io/txns/2BxVZNV5KsF343XcRDTAYMfGgkf3p4mbnezZvsnywqtE) |
| 10 | `nft_transfer_call` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [8myL7cbo…](https://testnet.nearblocks.io/txns/8myL7cbokBaKhJASjYYUMHWrjCPwGwYkdz21hKZvCyK3) |
| 11 | `nft_approve` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [J4CKvvBd…](https://testnet.nearblocks.io/txns/J4CKvvBdU881z6DtyYU3t1s9AwNRtRT6GexbbPm6B2V) |
| 12 | `nft_revoke` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [CgTPVZoN…](https://testnet.nearblocks.io/txns/CgTPVZoNUAkBZEQCw1AhwmHh91MCgCDzi2zNLeH22vTk) |
| 12 | `nft_revoke_all` → `onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [3PuUDr4C…](https://testnet.nearblocks.io/txns/3PuUDr4CaProf9howMn6Q9zp8p1E6cwzEJE3wj1YLN2s) |
| 13 | `nft_transfer` → `onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [9cuZPfnf…](https://testnet.nearblocks.io/txns/9cuZPfnfsPkMNjPyitmDb9RzSor7kXvPeZHXVr4KoKKm) |
| 14 | `nft_transfer` → `onhire.testnet` | `onhire.testnet` | refused (expected) | [xqAYctPj…](https://testnet.nearblocks.io/txns/xqAYctPjhT3iFWBudmeN6mBA3MZgd4QLrjhKHZKfK4c) |

#### testnet-t4 (pid 2798560)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [Hjv1yBFP…](https://testnet.nearblocks.io/txns/Hjv1yBFPew1pRiHZJQg1JuUGtpS3XXvbKo1sngTwHQqt) |
| 1 | `attest_inspection` → `box-0022.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [2uJRWzq4…](https://testnet.nearblocks.io/txns/2uJRWzq48o7dNdagRkqhchPTK7hpwpZizDBuBAr6aJUZ) |
| 2 | `start_lease` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [EiNBRaDf…](https://testnet.nearblocks.io/txns/EiNBRaDfJCo6iQvDTNv8w13wtyMkGAACkvo77Ho4sbU1) |
| 3 | `pay_hire` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [4wZdfGc7…](https://testnet.nearblocks.io/txns/4wZdfGc7uswZ84J9JMEH9dFrVm8tyLM2aHY3cXs86Yp8) |
| 4 | `CreateAccount` → `t4b1794479881.onhire-outsider-n12.testnet` | `onhire-outsider-n12.testnet` | ok | [4DJAHMau…](https://testnet.nearblocks.io/txns/4DJAHMauGrkFDRAHB2skxGkdvCLhdmNPy6GwcqkXdZfq) |
| 4 | `list` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [EbMFTSrG…](https://testnet.nearblocks.io/txns/EbMFTSrGZwdAVVMCWnkQK118wFxp3eUyoVbRpyNdb2JU) |
| 5 | `Transfer` → `onhire-racer1-n12.testnet` | `onhire.testnet` | ok | [G7Gsu1fv…](https://testnet.nearblocks.io/txns/G7Gsu1fvnXuXfD9b5QdawJFKxjcMDxRx6bmWBABCN1NJ) |
| null | `Transfer` → `onhire-racer1-n12.testnet` | `onhire.testnet` | rejected (not executed) | [HdS7xh8c…](https://testnet.nearblocks.io/txns/HdS7xh8cEmZow8c79DzdyaauCex4VXzauHbWf1HquFg6) |
| 5 | `Transfer` → `onhire-racer1-n12.testnet` | `onhire.testnet` | ok | [BeeJwLc8…](https://testnet.nearblocks.io/txns/BeeJwLc88P1sCjZox4RoG98n7cDn4jtTCgWWSUG5PdUY) |
| 5 | `buy` → `box-0022.onhire.testnet` | `t4b1794479881.onhire-outsider-n12.testnet` | ok | [BShUEd5d…](https://testnet.nearblocks.io/txns/BShUEd5dJYEtfXWjMyzs4Ww6yi3CFFYy1cPsrfaUyYKY) |
| 5 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-racer1-n12.testnet` | refused (expected) | [BDThK1Yj…](https://testnet.nearblocks.io/txns/BDThK1YjxKtkcN3soVc2DKh5SpoMqLXdidpHjgf2LKnT) |
| 5 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-inspector-n12.testnet` | refused (expected) | [AiBqK4jZ…](https://testnet.nearblocks.io/txns/AiBqK4jZh62DkAsoTtLQUgVfihULHHuaxShkDnENnc7W) |
| 5 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [6d99UKAV…](https://testnet.nearblocks.io/txns/6d99UKAVEuy1LLTZ89ccVsrkTdgnDTidjQjMSEjP8t5d) |
| 5 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [DGq1p8XP…](https://testnet.nearblocks.io/txns/DGq1p8XPbQDs2VjdNfkXSBDd9tLMLK7CsYkc4Fs9j8C) |
| 5 | `Transfer` → `onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [3FqMwnF7…](https://testnet.nearblocks.io/txns/3FqMwnF7edTJmC9CUSaw2ERvMFjGeHxNEgj9BL3YML91) |
| 6 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [6Ps7BV1C…](https://testnet.nearblocks.io/txns/6Ps7BV1CVvsacZD1vmLo1SZMMDHfnuJTaos1CuMSsuvd) |
| 6 | `on_nft_transfer` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [9eWRJ9f7…](https://testnet.nearblocks.io/txns/9eWRJ9f7MXKd4oTxqhaCYgtP1tUeqdX8c17pLwxu4yTR) |
| 7 | `withdraw` → `box-0022.onhire.testnet` | `t4b1794479881.onhire-outsider-n12.testnet` | ok | [3RFrbvng…](https://testnet.nearblocks.io/txns/3RFrbvngFbfEA9em6ukyRwXBPB5Vii6FJ6hAMEmft7kt) |
| 8 | `list` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [J6tfcQvq…](https://testnet.nearblocks.io/txns/J6tfcQvq3PTkdNBJiN6kKXdYmrXS53sgZBNzUj8f1KaG) |
| 9 | `buy` → `box-0022.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [BU4AZ4C9…](https://testnet.nearblocks.io/txns/BU4AZ4C9tAeTpvtSfaziUnXRhUdA5z4Jo8UPXWQZ5a12) |
| 9 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [BvMKCVqX…](https://testnet.nearblocks.io/txns/BvMKCVqXiZPVHs5gcFbxdWR2iUmh3iqzqTGCz6aFvqsC) |
| 9 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [ER6mbg3i…](https://testnet.nearblocks.io/txns/ER6mbg3iXWFxERWdEXfpgfbZKExmFXKazMehhK2UjmpQ) |
| 9 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [Fp1otbwS…](https://testnet.nearblocks.io/txns/Fp1otbwSrAfdxRmczWbLbgHgXXEshCaMhojruobDPCup) |
| 9 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [CttYJ63D…](https://testnet.nearblocks.io/txns/CttYJ63D6n533twHdGFCEM3STHQ1ZwL7aCKp4NNufVTV) |
| 10 | `list` → `box-0022.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [A92euq2g…](https://testnet.nearblocks.io/txns/A92euq2gLfusr2V3C7Z9CgdSq8gvZk6Dt9gamTRr8def) |
| 11 | `buy` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [E6FXGHDE…](https://testnet.nearblocks.io/txns/E6FXGHDEwcTYS3BUcNYJXjQdDz1ExM6TDbfVS7DcqFkN) |
| 11 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [GAK8GTRb…](https://testnet.nearblocks.io/txns/GAK8GTRbGWCmCoNrfF5WhbyKKB2118uZCE8TioPc9JA6) |
| 11 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [9VKVZJaG…](https://testnet.nearblocks.io/txns/9VKVZJaGjqpUMf8Bu2C98SVwN8nB9CYVQsocvJfj5K7N) |
| 11 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [J7TEkPkh…](https://testnet.nearblocks.io/txns/J7TEkPkh5cwLtDX4mSSunsig6zBeEcQND9vWRDRcpAyB) |
| 11 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [2V2TAPJ7…](https://testnet.nearblocks.io/txns/2V2TAPJ7QnBx2BnRjC9D1TvWMQxGebDstTvGUmTzraBS) |
| 12 | `list` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [A3rnHjmC…](https://testnet.nearblocks.io/txns/A3rnHjmCDrVvxiJ31SZf4zR45UAD85pu5QQfwpMuCj35) |
| 13 | `buy` → `box-0022.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [8V2ACtzX…](https://testnet.nearblocks.io/txns/8V2ACtzXW1ciwzoe2pGLNPY4z13pnZHahaPNjQwvbADr) |
| 13 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [FiLSi4XN…](https://testnet.nearblocks.io/txns/FiLSi4XN5qzieHUNBmLH4sAVhQKPkGyrDeicXBHfhtdF) |
| 13 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [4BEPAgWW…](https://testnet.nearblocks.io/txns/4BEPAgWWZTXNNgzrZfpmpFSPikZMctcMwtsgK9RmMf6Z) |
| 13 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [GdqZnc4e…](https://testnet.nearblocks.io/txns/GdqZnc4eTfx1qKCn3jGuo16EA5vHtpPcDpNnDgh3h9by) |
| 13 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [8HRFbwgb…](https://testnet.nearblocks.io/txns/8HRFbwgbphmjWM3X7L868REske15wjKvkcuNBp237zSE) |
| 14 | `list` → `box-0022.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [7KHaNmvz…](https://testnet.nearblocks.io/txns/7KHaNmvzhTDCJ3M1wPJjEuAXRYRwvxFAcakNJ9secWEc) |
| 15 | `buy` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [21RciF32…](https://testnet.nearblocks.io/txns/21RciF32eBc52Np4d1QnPopJ79jffFGuB4FKDAtrnB2f) |
| 15 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [D3tSVGXi…](https://testnet.nearblocks.io/txns/D3tSVGXi86aAdyL5VgPvZghWi4YJsE81uiSU8wgKcqki) |
| 15 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [4dueZKTq…](https://testnet.nearblocks.io/txns/4dueZKTqmMewrwP5RCTcT1NWvzNoS79swwo15bXZ2sm5) |
| 15 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [En8Y6SoS…](https://testnet.nearblocks.io/txns/En8Y6SoSgaQD3f5ta94SqQdL2dZTvhnhAXqwxB4giY2p) |
| 15 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [CeMmMNEE…](https://testnet.nearblocks.io/txns/CeMmMNEEC79GFsHQGqd1gSmbsG37fxbv56Hfnn9eiM7H) |
| 16 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [3zVSKEvF…](https://testnet.nearblocks.io/txns/3zVSKEvFX1UXCFuQEHxKZom7xKgAVSJWGyLv37ANA5ck) |
| 16 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [FyttJCuY…](https://testnet.nearblocks.io/txns/FyttJCuYYDDhFCRAfN6vB2wkDT2qeuK6hb8aQNVF1BnW) |
| 16 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [2j76JcuN…](https://testnet.nearblocks.io/txns/2j76JcuN16fSu7eMvbGGoc1AXtYrN4BsFURtRBsiog69) |
| 16 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [95eAo63t…](https://testnet.nearblocks.io/txns/95eAo63tk5Z98fFPLh9Sk2tfBEgPsdrkBcMKfXJD7C8U) |
| 16 | `rescue_sale` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [597j4RfP…](https://testnet.nearblocks.io/txns/597j4RfPfMKYkJgcBZaXtqnWMSEBLhNEEgHZU6QBPKfy) |
| 16 | `on_nft_transfer` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [78ugBLhS…](https://testnet.nearblocks.io/txns/78ugBLhS6DwLJifWHoDiqr8XVs2VBPdbWZvS8Mn4s8SR) |
| 16 | `on_rescue_owner` → `box-0022.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [HhXyozvk…](https://testnet.nearblocks.io/txns/HhXyozvkXjGxLTSgaTSF7B5VszgMFMcRiVqt8AjG1dc4) |
| 17 | `pay_hire` → `box-0022.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [BdBY8bBh…](https://testnet.nearblocks.io/txns/BdBY8bBhvtTUFBEjePQrJntTRHpXhKhEfuea4769EVni) |
| 18 | `withdraw` → `box-0022.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [3umtBKeW…](https://testnet.nearblocks.io/txns/3umtBKeWmfBhNads1ymWexq4mUuePr2yWd59FXMU5GGQ) |
| 19 | `withdraw` → `box-0022.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [CvShBGxL…](https://testnet.nearblocks.io/txns/CvShBGxLMDdvBCczPjC3UjYugkSUKbeczGvsYYZfPdmY) |

#### testnet-t5 (pid 2803325)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [A241rcR8…](https://testnet.nearblocks.io/txns/A241rcR83ds1UU5oCJ9KRerT9S9N59gEJJqwmDj3JLmS) |
| 1 | `attest_inspection` → `box-0023.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [AHhs1GAU…](https://testnet.nearblocks.io/txns/AHhs1GAUqS7t1ngUDE8aJCBKwCwgRuCEyJnZnYqLcoCR) |
| 2 | `start_lease` → `box-0023.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [AWRRJmq5…](https://testnet.nearblocks.io/txns/AWRRJmq5B7VvDKa3jGp998imorAw4taSF898fVeBB2sj) |
| 4 | `pay_hire` → `box-0023.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [FHECUfiv…](https://testnet.nearblocks.io/txns/FHECUfiv2M2tYJ8ukVXG6AzDjZGi3UmxMpsEQRu9fnVG) |
| 5 | `off_hire` → `box-0023.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [CioCCgYm…](https://testnet.nearblocks.io/txns/CioCCgYmjxm9vmcqEu7DJSbUr1ePfE8ZnmorfifjfDiK) |
| 6 | `withdraw` → `box-0023.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [xjzLcmSQ…](https://testnet.nearblocks.io/txns/xjzLcmSQG6qagn3kEVn5zvjUXkGmt4Amm3od7ARcR1C) |
| 7 | `start_lease` → `box-0023.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [2e1KyQnH…](https://testnet.nearblocks.io/txns/2e1KyQnHGB9v6GprQwyygL2W6qHRBaQmDoa4PmHfyFaC) |
| 8 | `attest_inspection` → `box-0023.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [2rt9hLWN…](https://testnet.nearblocks.io/txns/2rt9hLWNpJzNpXmvye2fsHTMGv1Y84cR4pBgSPLjdbXm) |
| 8 | `attest_inspection` → `box-0023.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [7bMNjxQ8…](https://testnet.nearblocks.io/txns/7bMNjxQ8vTecE5FrU6XvayV3qztZhcCAAgKT4awdCKD2) |
| 8 | `attest_inspection` → `box-0023.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [FLGwi31h…](https://testnet.nearblocks.io/txns/FLGwi31hJV2NZXqtbyV2KweViCyfjegSoQQTrk5FRjKw) |
| 9 | `attest_inspection` → `box-0023.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [12WPX2HS…](https://testnet.nearblocks.io/txns/12WPX2HSjozprKEVeUb2i6voiti6KFg98v5MbtZoJttX) |
| 10 | `start_lease` → `box-0023.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [Bkv7Wmbe…](https://testnet.nearblocks.io/txns/Bkv7WmbePnu366zSx8JMPsjNY1LXpcrUM8cDNy9WfVi9) |

#### testnet-t6 (pid 2804582)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `onhire.testnet` | `onhire.testnet` | ok | [FR1HsxHX…](https://testnet.nearblocks.io/txns/FR1HsxHXa6ZJinQmWAMYanLSgt3gReLMiRXCw4csS36m) |
| 1 | `attest_inspection` → `box-0024.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [6apDrsFN…](https://testnet.nearblocks.io/txns/6apDrsFNCmSAcW3ZrD4Cy3LtPLCy5xAVWPQ3PmzrYb6d) |
| 2 | `start_lease` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [3KaoRvDq…](https://testnet.nearblocks.io/txns/3KaoRvDqMhhneGpwADHYQYP4YpxUfkifHGxEgsUnFAR6) |
| 3 | `pay_hire` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [235xmRBF…](https://testnet.nearblocks.io/txns/235xmRBFwBoos2bhBi7cKChwkHa73Wt9JneZ4aV9wExT) |
| 4 | `withdraw_reserve` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [Gj2eo8ed…](https://testnet.nearblocks.io/txns/Gj2eo8edAgX7V6GRywJb7KHX7fkRAL8TBJ79EKpFXz3p) |
| 4 | `withdraw_reserve` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [Bt4JeG5m…](https://testnet.nearblocks.io/txns/Bt4JeG5mDbHbt88wmHfawrVcu9K4yMmnX8hz4iwuBpMk) |
| 4 | `withdraw_reserve` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [6Mtq6yhh…](https://testnet.nearblocks.io/txns/6Mtq6yhhbK1NsGQW1ZVG2h61MqZzBLFNhBDDegt7wtt1) |
| 4 | `reserve_withdraw` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [BSUX6gyL…](https://testnet.nearblocks.io/txns/BSUX6gyLriksMJsHaZYQHGRTyRNnQi9LGKKUF5aC5cN7) |
| 4 | `reserve_withdraw` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [87NQWwcD…](https://testnet.nearblocks.io/txns/87NQWwcD1SQsigRyWVe2CXPho8JjNYyfuQPW26g3zLhv) |
| 4 | `reserve_withdraw` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [9XQk8kGu…](https://testnet.nearblocks.io/txns/9XQk8kGub2pwNfdMWMWUJAqeKaUhyKWqPfZQdHa78qxq) |
| 4 | `claim_reserve` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [2SeVDUzj…](https://testnet.nearblocks.io/txns/2SeVDUzjnyYwdQR5NHUT3LDwK9AtinmXhAYp44fEMSfb) |
| 4 | `claim_reserve` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [3ycsyHEE…](https://testnet.nearblocks.io/txns/3ycsyHEE9VRAbGVcrdfQrwRwx1wANzWRxRWgq9ukdcQp) |
| 4 | `claim_reserve` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [B4YDddVQ…](https://testnet.nearblocks.io/txns/B4YDddVQCfJ1uJ9uX2zEJvfJj5JryeEuKqUmH6xE8GC) |
| 4 | `add_key` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [4bvXw7GP…](https://testnet.nearblocks.io/txns/4bvXw7GP1qjYdQuS2Pa3TiKZbTUCmsLbo2gazuvYEVfh) |
| 4 | `add_key` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [6ePGbRBK…](https://testnet.nearblocks.io/txns/6ePGbRBKZTv4F9L9rT71VEVtEVy3AdNk6r8mWPTYQfyF) |
| 4 | `add_key` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [FFdacFkE…](https://testnet.nearblocks.io/txns/FFdacFkEDtg9F7EmNWycoLGj4AC3jGo7qcqVPwiGfFSt) |
| 4 | `add_full_access_key` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [8FSpdWy5…](https://testnet.nearblocks.io/txns/8FSpdWy55TmWESi1jdNwcgEa5QY3wu5bTzcSYwiyNkyo) |
| 4 | `add_full_access_key` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [69DXTpoN…](https://testnet.nearblocks.io/txns/69DXTpoNx7AAEcpSr6bB3JS8topqQKuSkN1JMHmJnLJP) |
| 4 | `add_full_access_key` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [HooBpnpY…](https://testnet.nearblocks.io/txns/HooBpnpYdS9we5nCvafG27Zty2bRUyyoBzfGgM1GBoUE) |
| 4 | `delete_account` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [HwHhBbsy…](https://testnet.nearblocks.io/txns/HwHhBbsyMYPFJqDXJrauaxwd5gbXmzEbqDiwWLd9SNEn) |
| 4 | `delete_account` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [CCBJrHCB…](https://testnet.nearblocks.io/txns/CCBJrHCBnKZnBuFEHNvRwp8ND6dyxNzRih7es3wcVatH) |
| 4 | `delete_account` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [2vVNjbFt…](https://testnet.nearblocks.io/txns/2vVNjbFt6Kzdbn377b4R2CiLcNKYaq7SY8TS9udJrsX5) |
| 4 | `update` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [AcMQzc7A…](https://testnet.nearblocks.io/txns/AcMQzc7AtQUjgD6HWCjRa1Qrj4eAjKrohf1ffj9rgF3p) |
| 4 | `update` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [FKSyRRHZ…](https://testnet.nearblocks.io/txns/FKSyRRHZKiFN8bRMxTzKgD4p3UnaKjezbBCdEinhPDhM) |
| 4 | `update` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [82qW7RqN…](https://testnet.nearblocks.io/txns/82qW7RqNwGwJ2DiXKG1yicY1wWoAmV7s38Nqk7BsFfUh) |
| 4 | `upgrade` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [DyGjaqGL…](https://testnet.nearblocks.io/txns/DyGjaqGLotsr8S8yeLnGbxsbSgDKJu5Dqofzxb7wHuKz) |
| 4 | `upgrade` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [BoWLw5Bx…](https://testnet.nearblocks.io/txns/BoWLw5Bxuwyf3qMXsYKQ39dFYyVR9SJFH8BkXQBe7vn2) |
| 4 | `upgrade` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [ERB94tHa…](https://testnet.nearblocks.io/txns/ERB94tHa47ie88QscGB5rLv5eH8DHWQgA58UrXi8sfW) |
| 4 | `deploy` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [436Yn8Vx…](https://testnet.nearblocks.io/txns/436Yn8VxFy5YztjLqkuysBCYxz3jNpzZgoyt9NfGXhpV) |
| 4 | `deploy` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [DwAxy5JZ…](https://testnet.nearblocks.io/txns/DwAxy5JZYAMnuH7v73qUoy6rZhKketb7EZnq9FCH8WXL) |
| 4 | `deploy` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [2WYxH8ye…](https://testnet.nearblocks.io/txns/2WYxH8ye5TR2LiW2uy2qGm5rnxbiESMgc8bJQXiagkFy) |
| 4 | `migrate` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [FCv6RtWe…](https://testnet.nearblocks.io/txns/FCv6RtWeypnqMxE4yBmRESDKspSvrKKQ5WY3CUxHoBMs) |
| 4 | `migrate` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [2ZQz6Kfb…](https://testnet.nearblocks.io/txns/2ZQz6KfbpAWwXbfWa2B4qz1ArGxjQ3eSFarY8KyhQ7D6) |
| 4 | `migrate` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [AY89XuiQ…](https://testnet.nearblocks.io/txns/AY89XuiQs2gNbj2suiHsyVCtG17JXAwjgLGdyohGxV6S) |
| 5 | `withdraw` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [7UD5FNa6…](https://testnet.nearblocks.io/txns/7UD5FNa63wrC7Ag5a2URvchB8M1YazQWdcfEHJpd8uRx) |
| 6 | `withdraw` → `box-0024.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [AhNNRJvv…](https://testnet.nearblocks.io/txns/AhNNRJvv679gZPRykpoZV5pydBfGx6Y4oagJRY4VDP8v) |
| 6 | `withdraw` → `box-0024.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [D2Az2Jkr…](https://testnet.nearblocks.io/txns/D2Az2JkrQsmsCH4KSv8RQp1KvtWcAQMFSsntKvyJKiVk) |
| 6 | `withdraw` → `box-0024.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [96zan32P…](https://testnet.nearblocks.io/txns/96zan32PrttYs7BDaP3sqex4MvCgpDkUoFeAAVX8tAjh) |
| 6 | `withdraw` → `box-0024.onhire.testnet` | `onhire-inspector-n12.testnet` | refused (expected) | [BiVKuLMN…](https://testnet.nearblocks.io/txns/BiVKuLMNbWaBcizTwmrdwr7EosCaDhCvTmLUNoatArhp) |

