# Onhire — 1% protocol fee proof (sandbox + NEAR testnet)

## Current: lease-fix build on `v3.onhire.testnet` (2026-11-13)

Run Friday 2026-11-13, 15:52–16:24 ET (US Eastern). **NEAR testnet only.**
No mainnet, no real funds. All accounts are throwaways funded from the
public testnet faucet.

This deployment carries the two-step lease fix: the owner offers with
1 yocto, and the named lessee accepts and posts the deposit. The fee
logic is unchanged: `FEE_BPS = 100`, taken after the 15% reserve split, out
of the owner's or seller's share. Full deployment, accounts, commands and
the complete tx list are in `docs/TESTNET-PROOF.md` (section "Current").

| | |
| --- | --- |
| Collection | `v3.onhire.testnet` |
| Fee recipient (throwaway) | [`onhire-fee-n13.testnet`](https://testnet.nearblocks.io/address/onhire-fee-n13.testnet), created via the `testnet` registrar [8LHTLurG…](https://testnet.nearblocks.io/txns/8LHTLurGFyRofxyj4JjbeefWKpNwmai97tfzodZJeRfh) |
| `new(minter, fee_account)` | [37fiicAe…](https://testnet.nearblocks.io/txns/37fiicAessmHVepmD2AxKkufzGWSeYcqzS77sxybUNLT) |
| Item code (global, NEP-591) | sha256 `5e726984173c57c5d04f3c1e72624c94f6fdd235872e28063d769ce9301f9671`, `set_item_code` [HrMNaVk1…](https://testnet.nearblocks.io/txns/HrMNaVk1gSx8v17qZwiXeLci77hTdMSg5rxeMt8mmgR5) |
| Collection code | sha256 `e1036df87063267aea6d1daf0e9ccdbf9511911e2e240e6036d76dd4f64d2fe7` |
| Sandbox for the same wasm | 27 unit tests, 7 sandbox tests passed (`docs/SANDBOX-PROOF.md`) |

### t7 on testnet (pid 884475, `box-0008.v3.onhire.testnet`): PASS, 9 steps, 29 s

| Step | Check | Tx |
| --- | --- | --- |
| Lease | owner offer (1 yocto), lessee accept (0.1 NEAR deposit) | [58nEHVfY…](https://testnet.nearblocks.io/txns/58nEHVfYrdTkeQ1x5yniSTFs6xmMXfiagu8gB5G1GLqc), [FJ68gXLh…](https://testnet.nearblocks.io/txns/FJ68gXLh7N6aYceGDEXfnih2ofgVMKqjy6BKZL53W735) |
| Constants | `get_constants`: `FEE_BPS` 100, `RESERVE_BPS` 1500, `SALE_UNDO_DELAY_BLOCKS` 32, `FEE_ACCOUNT` `onhire-fee-n13.testnet` | — |
| `pay_hire` 100 yocto | reserve +15, fee account +1 yocto (exact), owner accrues 84 | [3EPGFHx4…](https://testnet.nearblocks.io/txns/3EPGFHx4EMLc4FBg2xeWvcHpmLdu3QraAXYCgFRSqKHp) |
| `list` + `buy` at 100 yocto | fee account +1 yocto (exact), seller credited 99, reserve unchanged; callback at lock + 2 blocks, finalized | [8cAc69VR…](https://testnet.nearblocks.io/txns/8cAc69VRnEyRtz2crt79mVEdpkZ62CodauGZiFWYPHsc), [CsKkfgd2…](https://testnet.nearblocks.io/txns/CsKkfgd2caANfbLy6AEEQs7Tgt1QZtpUCuzgA928rrMr) |
| Seller `withdraw` | exactly 99 + 84 yocto | [AtJU5Yk3…](https://testnet.nearblocks.io/txns/AtJU5Yk3dzp6US9M6q9CatHzp44BmP4YFBQvaSsh7C1p) |

Log lines:

```
[testnet-t7]   FEE OK [hire 100 yocto -> reserve 15 / fee 1 / owner 84]: onhire-fee-n13.testnet received exactly 1 yocto (0.000000 NEAR)
[testnet-t7]   FEE OK [sale 100 yocto -> fee 1 / seller 99]: onhire-fee-n13.testnet received exactly 1 yocto (0.000000 NEAR)
[testnet-t7] RESULT: PASS (9 checked steps, 29 s)
```

### Fee checks in the other tests (exact fee-account deltas)

| Test | Fee checks |
| --- | --- |
| t1 (PASS, 14 steps) | hire 0.1 → 0.001; sale 1 → 0.01; hire 0.2 → 0.002 NEAR |
| t2 (PASS, 11 steps) | undone sale → 0; retry sale 0.5 → 0.005 NEAR |
| t3 (PASS, 16 steps) | blocked transfers pay no fee (`get_fees_paid` 0) |
| t4 (PASS, 21 steps) | undone sale → 0; four sales 0.5 → 0.005 NEAR each, each charged once; hire 0.1 → 0.001 NEAR |
| t5 (PASS, 15 steps) | hire after the inspection lapsed 0.1 → 0.001 NEAR |
| t6 (PASS, 8 steps) | hire X = 123456789012345678901234 yocto → exactly 1234567890123456789012 yocto |

The table lists the runner's per-payment fee assertions. The box counters
also include one 0.1 NEAR hire payment each in t2 and t4 (0.001 NEAR fee)
that those tests make before their sales without a separate fee-account
check. So `get_fees_paid` is 0.006 NEAR on `box-0002` and 0.022 NEAR on
`box-0005`.

The fee account was created with 0.1 NEAR and never signs anything. Over
the whole run, including the failed and earlier passing t5 attempts
(`box-0006`, `box-0009` to `box-0011`), it gained exactly
**44,234,567,890,123,456,789,014 yocto**. That equals the sum of
`get_fees_paid` over all twelve v3 boxes, to the yocto. Every checked step
printed `INVARIANT OK, ZERO KEYS`.

Not shown on testnet: the fail-closed revert (`ERR_FEE_EXCEEDS_OWNER_SHARE`)
needs state patching. It is proven in unit tests and sandbox t7.

---

## Earlier proof: fee-only build on `v2.onhire.testnet` (2026-11-12)

This section is kept as a historical record. `v2.onhire.testnet` boxes run the
earlier lease code, in which the owner posted the deposit.


Commit IDs in this document refer to earlier development history, which is kept private. Fee commits: `ae2e0c4` (contracts + sandbox),
`a6fd509` (testnet runner), `7665554` (README/spec). The runner fixes for the
t4/t5 retries are in the commit that adds this file. Run date: 2026-11-12
(times are US Eastern).

**NEAR testnet only.** No mainnet, no real funds. Every testnet account below
is a throwaway funded from the public testnet faucet.

### Summary

| Test | Sandbox (global code) | Sandbox (local deploy) | Testnet first run | Testnet retry |
| --- | --- | --- | --- | --- |
| 1 — Mid-lease sale (+1% on lease payment and sale) | PASS | PASS | **PASS** (13 steps) | — |
| 2 — Failed transfer undoes the sale (undone sale pays no fee) | PASS | PASS | **PASS** (10 steps) | — |
| 3 — Direct transfer is blocked (no fee) | PASS | PASS | **PASS** (16 steps) | — |
| 4 — Stuck lock / rescue (fee charged exactly once) | PASS | PASS | **FAIL**: a dropped tx (see below) | **PASS** (20 steps) |
| 5 — Lapsed inspection (+1% on lease payment after lapse) | PASS | PASS | **FAIL**: `valid_now` read (see below) | **PASS** (11 steps) |
| 6 — Reserve cut; owner cannot take it (+1% of X) | PASS | PASS | **PASS** (7 steps) | — |
| 7 — Exact 100-yocto fee splits | PASS | PASS | **PASS** (8 steps) | — |

Every testnet test now has a PASS. Every checked step printed
`INVARIANT OK, ZERO KEYS`. That means rule 14 held both from the box's
`get_invariant_parts` and from the RPC account view, and the box account had
zero access keys.

Testnet t1–t3 and t6–t7 come from the first run. Testnet t4 and t5 come from
the retry run. The retry ran the same deployed contracts; only the
test-runner binary changed.

### The fee

- `FEE_BPS = 100` (1%), `BPS_DENOMINATOR = 10000`, `RESERVE_BPS = 1500` (15%).
- **Lease payment** `P`: reserve `15%` (unchanged), fee `1%` sent to the fee
  account at once, owner gets the rest. Proven exactly at **100 yocto → 15 / 1 / 84**
  (reserve / fee / owner).
- **Sale** price `P`: fee `1%`, seller gets `99%`, reserve untouched. Proven
  exactly at **100 yocto → 1 / 99** (fee / seller).
- An undone sale and a blocked transfer pay no fee. A finalized sale pays
  the fee exactly once, even when rescues race it (t2, t3, t4).
- Fail-closed: if the owner's share cannot cover the fee, `pay_hire` and
  `buy` revert with `ERR_FEE_EXCEEDS_OWNER_SHARE` and nothing is split.
  Proven in unit tests and in sandbox t7 by state patching. Testnet has no
  state patching, so this is not shown on testnet.
- `SALE_UNDO_DELAY_BLOCKS = 32` (unchanged). Testnet t7 read it from the box's
  `get_constants`. On testnet, the largest gap seen between a sale's lock and
  its callback was **2 blocks** in both runs.

#### Fee account

| | Account |
| --- | --- |
| Testnet fee recipient used for this proof | [`onhire-fee-n12.testnet`](https://testnet.nearblocks.io/address/onhire-fee-n12.testnet), a throwaway created through the testnet registrar ([5Xuk7Tu9…](https://testnet.nearblocks.io/txns/5Xuk7Tu9qAQEwoe6AfYZusGWcYx4KcVUhmZ9dVpMh4k1)) |

The fee account is passed to the collection's `new(minter, fee_account)` at
deploy time, and each box gets it at init. A testnet contract cannot pay a
`.near` account, so the testnet deploy passes `onhire-fee-n12.testnet`.
No mainnet fee account is named or used, and nothing has been deployed to
mainnet. Nobody else sends to the fee
account, so the runner checks that its balance grew by **exactly** the
expected yocto after every lease payment and sale.

### Sandbox (near-workspaces)

Run before the testnet deploy, on the contracts committed in `ae2e0c4`. The
contracts and `tests/tests/sandbox.rs` have not changed since then, so
sandbox was **not** rerun. Only the testnet runner binary changed for the
later retry.

```bash
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include
scripts/test-sandbox.sh                          # global item code (NEP-591)  -> logs/fee-sandbox-global.log
ONHIRE_CODE_MODE=local scripts/test-sandbox.sh   # local-deploy fallback       -> logs/fee-sandbox-local.log
```

Each mode built both contracts, then ran:

- `onhire-item` unit tests: **16 passed**. These include
  `split_hire_100_is_15_1_84`, `split_sale_100_is_1_99`,
  `pay_hire_100_yocto_sends_1_to_fee_account`,
  `sale_100_yocto_credits_99_and_fee_1_reserve_untouched`,
  `pay_hire_reverts_when_owner_share_cannot_cover_fee`,
  `buy_reverts_before_locking_when_fee_exceeds_price` and `fee_constants`.
- Sandbox `t1_mid_lease_sale` … `t7_protocol_fee`: **7 passed, 0 failed** in
  each mode (93.1 s global, 88.2 s local).

### Testnet

#### Wasm (sha256)

| Contract | sha256 (hex) | Deployed on testnet |
| --- | --- | --- |
| `target/near/onhire_item/onhire_item.wasm` (179,030 B) | `623920fef27fa29c40c43ae220e7cd5bdc33b63533fb149576cb8e134bf0c174` | Global contract (NEP-591), by hash. The collection's `get_item_code_info` returns `Yjkg/vJ/opxAxDriIOfNW9wztjUz+xSVdsuOE0vwwXQ=`, the base64 form of the same hash |
| `target/near/onhire_collection/onhire_collection.wasm` (222,074 B) | `6d855a51da73fdc1b1b1fb2d2e944bce806ddeb1151d6913de0e7c12afd3c192` | `v2.onhire.testnet`. The account's `code_hash` `8NXNs67zN371hjbxSPn4A1tfWqdxHtmsQXH5Xnwk9rD7` is the base58 form of the same hash |

Both hashes were rechecked against the chain on 2026-11-12 after the retry.
Builds: `cargo near build non-reproducible-wasm --no-abi`. The binaries are
not reproducible across machines.

#### Accounts (all throwaway, testnet)

| Role | Account |
| --- | --- |
| Fee build collection / factory / minter | [`v2.onhire.testnet`](https://testnet.nearblocks.io/address/v2.onhire.testnet) (sub-account of `onhire.testnet`; created [6GdVu5xv…](https://testnet.nearblocks.io/txns/6GdVu5xvdasFkUGvWpaHXnPwGTYE1tDKdH13goyra1wg)) |
| Fee recipient | [`onhire-fee-n12.testnet`](https://testnet.nearblocks.io/address/onhire-fee-n12.testnet) |
| Owner / lessee / buyer / inspector | `onhire-owner-n12.testnet`, `onhire-lessee-n12.testnet`, `onhire-buyer-n12.testnet`, `onhire-inspector-n12.testnet` |
| Rescue callers | `onhire-outsider-n12.testnet`, `onhire-racer1-n12.testnet` |
| Fresh buyers (sub-accounts of the outsider, never owned a token) | `t2b1794499522.…` (first-run t2), `t4b1794499813.…` (first-run t4), `t4b1794513637.onhire-outsider-n12.testnet` (retry t4) |
| Funding donors (testnet NEAR only) | inspector, racer1, owner, buyer2 → collection (fee-setup) |

Keys live in `~/.onhire-testnet` (mode 700, outside the repo).

#### Setup and deploy (once, before the first run)

| Step | Tx |
| --- | --- |
| create `onhire-fee-n12.testnet` | [5Xuk7Tu9…](https://testnet.nearblocks.io/txns/5Xuk7Tu9qAQEwoe6AfYZusGWcYx4KcVUhmZ9dVpMh4k1) |
| create `v2.onhire.testnet` | [6GdVu5xv…](https://testnet.nearblocks.io/txns/6GdVu5xvdasFkUGvWpaHXnPwGTYE1tDKdH13goyra1wg) |
| fund collection: inspector 8, racer1 8, owner 4, buyer2 3 test NEAR | [47WRs5LC…](https://testnet.nearblocks.io/txns/47WRs5LCnDvWPMxpK3ZR9J85bLjJFGPt46hyvNW2vbgQ), [EZfrePQR…](https://testnet.nearblocks.io/txns/EZfrePQRhguhFC47LmvZd5PmWbKsehkFSCnvRLeeM2qv), [D9AyoHWn…](https://testnet.nearblocks.io/txns/D9AyoHWnK1id3c5PmRSGJqqpf8yjzf2ThfidwgT9ALTh), [F1suAmGo…](https://testnet.nearblocks.io/txns/F1suAmGodbeVAidHckJfYzfprbwbaFEVGQtooy3N98Tz) |
| (racer1's first 8-NEAR move, `6Lf2zibq…`, was dropped before inclusion and never landed. Both RPCs report it unknown. fee-setup was rerun.) | — |
| deploy collection wasm | [Dfco7exE…](https://testnet.nearblocks.io/txns/Dfco7exEneszfbpebdcnsdbPKiCY5WcFqhhwKgNQc6Rp) |
| `new(minter, fee_account = onhire-fee-n12.testnet)` | [B6LHVi9W…](https://testnet.nearblocks.io/txns/B6LHVi9WyiVf6MJ3y3cv64Q3kYa1cqZa5a6PaAcZDFH6) |
| `set_item_code(global = true)` (17.88 test NEAR) | [AZNM8AtT…](https://testnet.nearblocks.io/txns/AZNM8AtTbWL43aY71N2Q2LpyeBBYon23zAMuCrwfH3om) |

#### Commands

```bash
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include
export NEAR_TESTNET_RPC=https://rpc.testnet.fastnear.com

# once
cargo run -p onhire-tests --bin onhire-testnet -- fee-setup   # fee account + v2 collection + funding
cargo run -p onhire-tests --bin onhire-testnet -- deploy      # collection wasm, new(..., fee_account), set_item_code(global)

# first run (after the 11:04 ET deploy; finished 11:36 ET), output in logs/fee-testnet-proof.log
scripts/test-testnet.sh                      # t1 … t7

# retry (15:00:23–15:05:26 ET), output in logs/fee-testnet-retry.log, exit 0
scripts/test-testnet.sh t4 t5

# checks
scripts/testnet/audit-nonces.sh              # -> logs/fee-testnet-audit.log
scripts/testnet/proof-links.sh <pid> …       # appendix below
```

The runner only accepts account IDs ending in `.testnet`. Its worker is
`near_workspaces::testnet()`, and it contains no mainnet configuration.

#### Testnet results, per test

| Test | Run (pid) | Box | Result | Fee checks (fee account delta, exact) |
| --- | --- | --- | --- | --- |
| t1 | first (3083535) | `box-0001.v2.onhire.testnet` | PASS, 13 steps, 45 s | hire 0.1 → 0.001; sale 1 → 0.01; hire 0.2 → 0.002 NEAR |
| t2 | first (3084444) | `box-0002`, `box-0003` | PASS, 10 steps, 241 s | undone sale → 0; retry sale 0.5 → 0.005 |
| t3 | first (3087797) | `box-0004` | PASS, 16 steps, 57 s | blocked transfers → 0 (`get_fees_paid` = 0) |
| t4 | first (3088783) | `box-0005` | **FAIL** after 5 steps | — |
| t5 | first (3093691) | `box-0006` | **FAIL** after 3 steps | — |
| t6 | first (3095625) | `box-0007` | PASS, 7 steps, 103 s | lease payment X = 123456789012345678901234 yocto → exactly X/100 |
| t7 | first (3096945) | `box-0008` | PASS, 8 steps, 26 s | **lease payment 100 yocto → 15 / 1 / 84; sale 100 yocto → 1 / 99** |
| t4 | **retry** (3136943) | `box-0009` | **PASS, 20 steps, 224 s** | 4.F undone sale → 0; 4.S0–S3 four sales 0.5 → 0.005 each, once each; lease payment 0.1 → 0.001 |
| t5 | **retry** (3141046) | `box-0010` | **PASS, 11 steps, 78 s** | lease payment after lapse 0.1 → 0.001 |

Key t7 txs (exact splits): `pay_hire` 100 yocto
[2RJuTMxU…](https://testnet.nearblocks.io/txns/2RJuTMxUsu9f2CzBNxVjm2qagcXCpxCgUdEczhJpBNRa);
`buy` 100 yocto
[7WGKReGR…](https://testnet.nearblocks.io/txns/7WGKReGRaJNSKo7XopKiSY87yVB1dCN5GWYTx3QzqGPt)
(lock@277388516, transfer +1, callback +2, finalized). The seller then
withdrew 99 + 84 yocto
([4k5zYxpk…](https://testnet.nearblocks.io/txns/4k5zYxpkx1VuaUNNDSGmWFBTFvazz88bc7HwPNyEPqxH)).
`get_constants` returned
`{"RESERVE_BPS":1500,"SALE_UNDO_DELAY_BLOCKS":32,"MIN_RESERVE":"0","BPS_DENOMINATOR":"10000","FEE_BPS":100,"FEE_ACCOUNT":"onhire-fee-n12.testnet"}`.

#### First-run failures and how they were retried

**t4 (first run): FAIL, dropped transaction, not a contract failure.**
Round 4.F ran to the end on chain: the collection was drained, the sale's
transfer failed, the sale was undone via the callback, and the collection
was restored. Then the runner collected the concurrent `rescue_sale`
racers. Three of them landed (`ok:noop`). The fourth racer's tx
`14pjkL367vAsfHioP4hA3TMdRa2EQpzsrR5xcpY1iwXW` (by output order, signed by
`onhire-buyer-n12.testnet`) was "not final after 90 s (never landed)". The
racers used raw `send_once` with no dropped-tx handling, so the test
aborted.

- Before retrying, I checked whether it had landed later. `tx` returns
  `UNKNOWN_TRANSACTION` from both `rpc.testnet.fastnear.com` and
  `archival-rpc.testnet.near.org`, for every plausible signer. It never landed.
  No funds moved, and it was not resent.
- Runner fix in `race()`: when a racer's tx is "never landed", the runner
  confirms with two RPCs that it is unknown (the same `tx_unknown` check the
  existing resend logic uses), then records it as `dropped` with no resend.
  The moment it was racing has passed. Any other error still fails the
  test. The outcome checks accept `dropped`; it never counts as an undo or a
  finalize. A race now fails if *every* racer was dropped.
- Retry: **PASS, 20 checked steps.** It used a fresh buyer,
  `t4b1794513637.onhire-outsider-n12.testnet`
  ([AQRaRTJg…](https://testnet.nearblocks.io/txns/AQRaRTJgWAADkMKbBH4hHZ7uCGkWjiasaw6YphiuHoDw)).
  No racer was dropped in the retry, so the new path was not exercised.
  Rescue tally: failed-transfer `noop` ×2, `ERR_STALE_SALE_ID` ×2;
  successful-transfer `noop` ×11, `ERR_STALE_SALE_ID` ×5. There was exactly
  one undo (by callback) and exactly one finalize per sale. Max
  callback − lock was 2 blocks (delay = 32).

**t5 (first run): FAIL, `valid_now` read too early or from a lagging node.**
The runner attested a 45 s inspection and started the lease. It then
polled `view_block` until its timestamp was past `valid_until + 1`. Next
it called `get_inspection` **unpinned** (latest/optimistic, behind a
load-balanced RPC), which returned `valid_now: true`. The two reads are
independent and can be served by different nodes. The one serving the view
can still have a head inside the window.

- The contract was **not** changed. `valid_now = block_timestamp_secs <= valid_until`
  is the same code.
- Runner fix: after the window ends, the runner takes a block whose
  timestamp is past `valid_until`, then reads `get_inspection`
  **pinned to that block height** (`.block_height(h)`). It retries while a
  node hasn't got block `h` yet, for up to 300 s. If the pinned read says
  `valid_now: true`, the test still fails.
- Retry: **PASS, 11 checked steps.** Pinned read at block 277409973
  (ts 1794513897 > valid_until 1794513895):
  `{"inspector":"onhire-inspector-n12.testnet","valid_until":1794513895,"valid_now":false}`.
  The rest of t5 behaved as specified. `pay_hire` after the lapse
  succeeded with exactly 1% to the fee account
  ([3W3HMmxe…](https://testnet.nearblocks.io/txns/3W3HMmxeCUgDC3DTa1HL66k2bY6ahyujNf2hvb3NBPMu)).
  The lessee off-hired and withdrew 0.1. `start_lease` was refused with
  `ERR_INSPECTION_LAPSED`
  ([58xrq4GZ…](https://testnet.nearblocks.io/txns/58xrq4GZsnQtH98HMUQoQuDks2rHPkf1vKyFF812QFJP)).
  owner/buyer/lessee attests were refused with `ERR_NOT_INSPECTOR`. The
  inspector re-attested, and `start_lease` then succeeded
  ([5UsxekT7…](https://testnet.nearblocks.io/txns/5UsxekT7WnuLkUyko4si8hFMq6ph8e5cEpPQyfeoUMZn)).

#### Nonce audit: open discrepancy (not explained)

`scripts/testnet/audit-nonces.sh` (output in `logs/fee-testnet-audit.log`)
compares each signer's on-chain key nonce with the txs the runner recorded:

| Account | On chain | Recorded | Status |
| --- | --- | --- | --- |
| buyer | 102 | 101 | 1 unrecorded |
| inspector | 64 | 63 | 1 unrecorded |
| lessee | 118 | 114 | 4 unrecorded |
| owner | 182 | 180 | 2 unrecorded |
| racer1 | 33 | 34 | −1: the recorded `6Lf2zibq…` (fee-setup) was dropped and never landed, so it used no nonce. Explained. |
| all others (`onhire.testnet`, `v2.onhire.testnet`, outsider, buyer2, fee, fresh buyers) | | | OK |

The FastNear tx index lists each signer's on-chain txs. Comparing it with
the records finds exactly **8 unrecorded txs**. All of them went to
`box-0006.v2.onhire.testnet`, the abandoned **first-run t5** box, between
block 277386594 and 277386632 (11:17:40–11:18:03 ET). That is about 2.5 min
after the first-run t5 started its lease (block 277386335) and about
16 min before first-run t6 minted (block 277388314):

| Tx | Signer | Call | Outcome |
| --- | --- | --- | --- |
| [DPbedaDB…](https://testnet.nearblocks.io/txns/DPbedaDB5NMFXn2raNqQTcQ4Lj17uNLQwGUwtE8sDCLe) | lessee | `pay_hire` 0.1 NEAR | success |
| [C2q5X8ao…](https://testnet.nearblocks.io/txns/C2q5X8aoadunCwyJhS9XEY9Pu7tLM4VkJqfD27kGk1pa) | lessee | `off_hire` | success |
| [A6mcDwMR…](https://testnet.nearblocks.io/txns/A6mcDwMRQLJqjTnziz9TGs98XTNN7jzFSkZyy2AKkWsz) | lessee | `withdraw` | success |
| [2BEGdPmh…](https://testnet.nearblocks.io/txns/2BEGdPmhzTMHyvnMy2M9zicHgJB7ZA2og5ef9Wyv4G21) | owner | `start_lease` | failure (refused) |
| [8XmvjnTC…](https://testnet.nearblocks.io/txns/8XmvjnTCJ2GU7nZgQCPq21KotNXFaLX24HFkgG3gQZHF) | owner | `attest_inspection` | failure (refused) |
| [9o1FZAbC…](https://testnet.nearblocks.io/txns/9o1FZAbCRbqhH5GQ8bPckaSm2ZAdsQTMk9iFHH71kioX) | buyer | `attest_inspection` | failure (refused) |
| [FAHkvKfJ…](https://testnet.nearblocks.io/txns/FAHkvKfJ2vU3TciWG5TtGX1p4iEfX8ib1KXdvsBTKXYL) | lessee | `attest_inspection` | failure (refused) |
| [D724eV1A…](https://testnet.nearblocks.io/txns/D724eV1A9BSTg14e3XQXvq1m4pmQZHhe5zTP2Bo4cioV) | inspector | `attest_inspection` `valid_until` 1797092274 (+30 d) | success |

This sequence is exactly the remainder of the runner's t5 code after the
lapse check, minus the final `start_lease`. It was signed with the
throwaway testnet keys. I could not find what sent it:

- `logs/testnet-records.jsonl` has no record of it.
- `logs/fee-testnet-proof.log` shows that box's t5 run (pid 3093691)
  stopping at step 3.
- The previous run's session log (kept locally, not committed) does not
  mention these txs.
- No other records file or runner checkout exists on the machine.

The txs touched only `box-0006`, a box no passing test uses, and moved
only testnet NEAR. They do not change any result above. They do mean the
audit is **not** clean for this run. Treat their origin as unknown until
someone explains it.

### Appendix: every recorded testnet tx, by run

Generated with `scripts/testnet/proof-links.sh 3083535 3084444 3087797 3088783 3093691 3095625 3096945 3136943 3141046`.
The first seven pids are the first run (t1–t7). The last two are the retry
(t4, t5). "refused (expected)" marks a call the test expects the contract
to reject.

#### testnet-t1 (pid 3083535)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [G5b74ALL…](https://testnet.nearblocks.io/txns/G5b74ALLjbyNVaY2DHaopm1hfkpf6zjmkmFyLkAJMknv) |
| 1 | `attest_inspection` → `box-0001.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [5MjuanuG…](https://testnet.nearblocks.io/txns/5MjuanuG2gj7tJiLVvKTcMiSK33xdxuRs959rQQZTgW8) |
| 2 | `start_lease` → `box-0001.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [5C8BwgrX…](https://testnet.nearblocks.io/txns/5C8BwgrXkDs51CnedNoWDEWHf62fgxQ6Bm4cPszArkMF) |
| 3 | `pay_hire` → `box-0001.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [3GZJSXxR…](https://testnet.nearblocks.io/txns/3GZJSXxRRMKTkH4wjfHivU55F2dQyBCmV77zLb85JjQs) |
| 4 | `list` → `box-0001.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [DGQe37Tq…](https://testnet.nearblocks.io/txns/DGQe37TqEmesWbzePobeJ4fvJvWA1s9gaDg5w6Ua7Ldo) |
| 5 | `buy` → `box-0001.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [4C43SLQX…](https://testnet.nearblocks.io/txns/4C43SLQXTiBjyYipXL5ttEeXMy7yuxRKsCzJeupkZDza) |
| 6 | `buy` → `box-0001.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [6RbCwoUX…](https://testnet.nearblocks.io/txns/6RbCwoUXcaJUviFnC7EjgFDJus8XEq3AzJBjmChqPPLm) |
| 8 | `pay_hire` → `box-0001.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [A92k7TJc…](https://testnet.nearblocks.io/txns/A92k7TJcfikt2JuGJEXB4iJwCsSPZ9ozN7WtpEzQspiz) |
| 9 | `withdraw` → `box-0001.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [4DZ2s3vv…](https://testnet.nearblocks.io/txns/4DZ2s3vvTi7Vqu3mrogWgEB2ipk689AMKVSeC47W6sVW) |
| 10 | `withdraw` → `box-0001.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [HVALr1B8…](https://testnet.nearblocks.io/txns/HVALr1B8YNEz96KBxBFvuuqNubwGrDsQHxaZWmKV7Sk5) |
| 11 | `withdraw` → `box-0001.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [GLYBC8zS…](https://testnet.nearblocks.io/txns/GLYBC8zSn6ECNLyC1b31ALmH53XSNWdkRVky1BrGwc8b) |
| 12 | `withdraw_reserve` → `box-0001.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [6JP7B859…](https://testnet.nearblocks.io/txns/6JP7B85913K5bmz4G8vPWUcEoo1GfdqFLLZtd5U8NfWM) |

#### testnet-t2 (pid 3084444)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [EcdE4Nqg…](https://testnet.nearblocks.io/txns/EcdE4NqgchCcLgAuyMggtS1T9y1BfToH5tP47G9nDKiH) |
| 1 | `attest_inspection` → `box-0002.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [GhAx1Ndo…](https://testnet.nearblocks.io/txns/GhAx1NdotxgGLF1KMMiXy9tCvYrnJsvHf7sbHVd2Pupm) |
| 2 | `start_lease` → `box-0002.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [54rMSJ5j…](https://testnet.nearblocks.io/txns/54rMSJ5jXdbjmLkgQJVA3BC7qi3zN1YQMS9chWbymTKB) |
| 3 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [DmYuYwJs…](https://testnet.nearblocks.io/txns/DmYuYwJsFH7LVdJyqZAFWzxrNaAPADAsc8smQjdSEYuY) |
| 4 | `pay_hire` → `box-0002.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [DnWRJoSj…](https://testnet.nearblocks.io/txns/DnWRJoSjJJeguEP9Wt3phRCRfEMFNrk5uTKHKGM73j2V) |
| 5 | `list` → `box-0002.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [G3sfGq3s…](https://testnet.nearblocks.io/txns/G3sfGq3sPcTM5paAxKvuGJgSDfc1dF995vagp9NLTDbt) |
| 6 | `CreateAccount` → `t2b1794499522.onhire-outsider-n12.testnet` | `onhire-outsider-n12.testnet` | ok | [61Vz8V1X…](https://testnet.nearblocks.io/txns/61Vz8V1XP5RmPxCWpGrq8QzxVVZ7wJibrEoLwghwuHCF) |
| 6 | `Transfer` → `onhire-outsider-n12.testnet` | `v2.onhire.testnet` | ok | [22Bxj8NJ…](https://testnet.nearblocks.io/txns/22Bxj8NJbM7NbfZ78xYLqTwxKMu4L9CihxVHz5626vND) |
| null | `Transfer` → `onhire-outsider-n12.testnet` | `v2.onhire.testnet` | rejected (not executed) | [2dW3S4JC…](https://testnet.nearblocks.io/txns/2dW3S4JCvRKEQyf2XCYuipUpcdPM9Ce958GbqfX6J7dN) |
| 6 | `Transfer` → `onhire-outsider-n12.testnet` | `v2.onhire.testnet` | ok | [FPNqiyrz…](https://testnet.nearblocks.io/txns/FPNqiyrzwsnkzW6gxY7giz6UV8hvHBq5jWvZd8N3op9H) |
| 6 | `buy` → `box-0002.v2.onhire.testnet` | `t2b1794499522.onhire-outsider-n12.testnet` | ok | [6m3qYsB2…](https://testnet.nearblocks.io/txns/6m3qYsB288dJ6i2E6cwCUJiYp1rsZUTPj3Yc4YeW1eLZ) |
| 6 | `Transfer` → `v2.onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [9AUxKbN7…](https://testnet.nearblocks.io/txns/9AUxKbN74ae5CrmTrBqFCe5jW4yocv7r5UKy1A6EqZ9r) |
| 8 | `withdraw` → `box-0002.v2.onhire.testnet` | `t2b1794499522.onhire-outsider-n12.testnet` | ok | [Fw5ohCo9…](https://testnet.nearblocks.io/txns/Fw5ohCo9qGko5h4vqWx47Ro5ghfEuBeV9oiZTqtTcRQF) |
| 9 | `buy` → `box-0002.v2.onhire.testnet` | `t2b1794499522.onhire-outsider-n12.testnet` | ok | [997RB3p5…](https://testnet.nearblocks.io/txns/997RB3p5t4J1gQaUMbBuPtW6u6MAGRgkdAhjVBhuGrbU) |

#### testnet-t3 (pid 3087797)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [E3gPQvSp…](https://testnet.nearblocks.io/txns/E3gPQvSp6KJsA2HvekQGbgbw2VBu2DR2k7EYEy6dZsWP) |
| 1 | `nft_transfer` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [39NL5yrh…](https://testnet.nearblocks.io/txns/39NL5yrhgc9bntao9NRbttdZ96ed5kbNheh4EwNTgNnc) |
| 2 | `nft_transfer_call` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [HnmxSQKS…](https://testnet.nearblocks.io/txns/HnmxSQKSe5u8mEpZg7AKmAoSK91J9SM3t2vPiQdaaLL9) |
| 3 | `nft_approve` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [8tDH39VC…](https://testnet.nearblocks.io/txns/8tDH39VCkvocX7RvL59ERhCDvM8ktiR7D5TFeGvHTV3d) |
| 4 | `nft_revoke` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [A9Si9MGZ…](https://testnet.nearblocks.io/txns/A9Si9MGZJTLsfmeMXwsasV3pCUj9ZZDsLKWJDE8sUvtQ) |
| 4 | `nft_revoke_all` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [BjwpRHh4…](https://testnet.nearblocks.io/txns/BjwpRHh4u8Wpw5f81nUdgPFyHZEx13gAMLjWY41sbQPg) |
| 5 | `nft_transfer` → `v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [D1dJ1haN…](https://testnet.nearblocks.io/txns/D1dJ1haNUR2XBrc89rNaZRtnjcG5MpQBStPfZDM9MLML) |
| 6 | `nft_transfer` → `v2.onhire.testnet` | `v2.onhire.testnet` | refused (expected) | [6HJyJBtW…](https://testnet.nearblocks.io/txns/6HJyJBtWsopB5jKchY9m8DM2yckj4m393vjuoCY7oQxP) |
| 8 | `list` → `box-0004.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [2tgZnNgm…](https://testnet.nearblocks.io/txns/2tgZnNgmgX9L2A2PVGNpEUopJXwK6jKrPZgPii5PBwDh) |
| 9 | `nft_transfer` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [8rNzD5Ku…](https://testnet.nearblocks.io/txns/8rNzD5KuSME5rDtmAYq9C8MEHvmzPrDwxWXsDHKkoQpU) |
| 10 | `nft_transfer_call` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [5rd3DMDi…](https://testnet.nearblocks.io/txns/5rd3DMDiMNnCH9otrTDLXjyvDfgh9LBk4SUrxt5UKiNP) |
| 11 | `nft_approve` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [E4osLcw9…](https://testnet.nearblocks.io/txns/E4osLcw9oXwKg1cnZ8fqvfjJcMtF5gx44UiCMnCAhpmv) |
| 12 | `nft_revoke` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [9QJdxuy2…](https://testnet.nearblocks.io/txns/9QJdxuy2LPD5CPnBXVQitCtbfxwQ9R79ftT61tnHaz2V) |
| 12 | `nft_revoke_all` → `v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [9haAVqk4…](https://testnet.nearblocks.io/txns/9haAVqk47jwWNpHQRqFe5gLYYkMwXQjQ8bFNkMQL1q4y) |
| 13 | `nft_transfer` → `v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [9GZyJ7aq…](https://testnet.nearblocks.io/txns/9GZyJ7aqeMSvgq86Hw6ha83q9nHEaTwJgeyykP9ySWAw) |
| 14 | `nft_transfer` → `v2.onhire.testnet` | `v2.onhire.testnet` | refused (expected) | [GJJXLou4…](https://testnet.nearblocks.io/txns/GJJXLou4v1fwcxGByF4LLvfBhxmJPhrujAe94zJNXjX9) |

#### testnet-t4 (pid 3088783)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [Div6CbXP…](https://testnet.nearblocks.io/txns/Div6CbXPqZ1ywDV45b6WYANap8tuRtcFVX9n1Jf94zKP) |
| 1 | `attest_inspection` → `box-0005.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [9bMByNsD…](https://testnet.nearblocks.io/txns/9bMByNsDxzDqrHax1SX3AKpZ8M7LEB1tv3UXJVteiWrR) |
| 2 | `start_lease` → `box-0005.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [EnD3S1bS…](https://testnet.nearblocks.io/txns/EnD3S1bSvGrWZ6JUC1ZyfhEDTdyAkVCLUiT4kkvjPvm1) |
| 3 | `pay_hire` → `box-0005.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [BBMvMBh9…](https://testnet.nearblocks.io/txns/BBMvMBh9X2wiuEZ3WHqe4syXxzXpW7RhqXwX3kB2wS4B) |
| 4 | `CreateAccount` → `t4b1794499813.onhire-outsider-n12.testnet` | `onhire-outsider-n12.testnet` | ok | [2SvEnceD…](https://testnet.nearblocks.io/txns/2SvEnceDL5mZCknobQk1bmo6ar14s6bBysVmkYioFa6a) |
| 4 | `list` → `box-0005.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [6nUPuDKn…](https://testnet.nearblocks.io/txns/6nUPuDKnGfRcm2bHkiyKNtGezH3s4UTm9JgCb18YSbme) |
| 5 | `Transfer` → `onhire-racer1-n12.testnet` | `v2.onhire.testnet` | ok | [FXMkkQwR…](https://testnet.nearblocks.io/txns/FXMkkQwRBaK5Kpynrke3FRHLw18cGvmtTAdgssSDjMua) |
| null | `Transfer` → `onhire-racer1-n12.testnet` | `v2.onhire.testnet` | rejected (not executed) | [6xSdDoxM…](https://testnet.nearblocks.io/txns/6xSdDoxMAvJqCE8THKuoW4zBt2UGMFWAPNsFzfUV9ATd) |
| 5 | `Transfer` → `onhire-racer1-n12.testnet` | `v2.onhire.testnet` | ok | [2pRrXEUF…](https://testnet.nearblocks.io/txns/2pRrXEUFnV34mWNn15hvKvTqV2ycesZZ2RBK6MLJsD16) |
| 5 | `buy` → `box-0005.v2.onhire.testnet` | `t4b1794499813.onhire-outsider-n12.testnet` | ok | [7y5WawB1…](https://testnet.nearblocks.io/txns/7y5WawB1Dobbc7ssD5iSkqgmnksxssQJ8o1hjEKa8R6v) |
| 5 | `rescue_sale` → `box-0005.v2.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [Cze1nxMZ…](https://testnet.nearblocks.io/txns/Cze1nxMZqaFoNuSLuAEBT8Uokuq9pDQiAewqbNA2U2Zc) |
| 5 | `rescue_sale` → `box-0005.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [D6GWXFRm…](https://testnet.nearblocks.io/txns/D6GWXFRmneJyxEBcMrkb4vVmwuBAs4UDGoHqbL5MBfix) |
| 5 | `rescue_sale` → `box-0005.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [7FNqhLGn…](https://testnet.nearblocks.io/txns/7FNqhLGnex7gipmQFyMjMmut7nhf5jgX9pVaNodGRbzh) |
| 5 | `Transfer` → `v2.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [CRfYVxW3…](https://testnet.nearblocks.io/txns/CRfYVxW33nYA1vbSJrdfb8aRGHDiPyqbAcG7kJ3P8Rit) |

#### testnet-t5 (pid 3093691)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [HsNJiCsz…](https://testnet.nearblocks.io/txns/HsNJiCszUx24wdTakmA8ZT8BRnqUkg2fpzDoEW7sDqU9) |
| 1 | `attest_inspection` → `box-0006.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [DGDphPgx…](https://testnet.nearblocks.io/txns/DGDphPgxby6DFpbNwc1nLiHFNk887RxDZHgM8kAqzxH8) |
| 2 | `start_lease` → `box-0006.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [3B2dq3fY…](https://testnet.nearblocks.io/txns/3B2dq3fYHDBC5789YS6VZYVhRHigcTNmhvtBYtYZ6DZJ) |

#### testnet-t6 (pid 3095625)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [8f8v5Crq…](https://testnet.nearblocks.io/txns/8f8v5CrqQWo3MB58iwAoTyFL24nCd7eBd5d4xYEcW1RC) |
| 1 | `attest_inspection` → `box-0007.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [CE7NKNfp…](https://testnet.nearblocks.io/txns/CE7NKNfpWUuiUWuZNpdgfnzinete3tZb5esVCss79bmw) |
| 2 | `start_lease` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [4Y5WBkBd…](https://testnet.nearblocks.io/txns/4Y5WBkBdufFSgrAK9ed9VjshoEjWrtJSuy5vgbfy7bEb) |
| 3 | `pay_hire` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [m1LGML8m…](https://testnet.nearblocks.io/txns/m1LGML8mw7kZBpTsVawXfsqomUyzNhWHxcWS969wLQH) |
| 4 | `withdraw_reserve` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [5WZsJJm9…](https://testnet.nearblocks.io/txns/5WZsJJm9te9wCc18jZK8gvHci2mm2paEsxPdGLapWLsB) |
| 4 | `withdraw_reserve` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [jArSftEN…](https://testnet.nearblocks.io/txns/jArSftEN7GVZHZjCj2yShg6YKQuJmmdD48QSKMv63y8) |
| 4 | `withdraw_reserve` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [G9Sti41t…](https://testnet.nearblocks.io/txns/G9Sti41tWUzqAxghLx83iLxDrJBP5BkoZsxhPwC8txiQ) |
| 4 | `reserve_withdraw` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [AauEeUjT…](https://testnet.nearblocks.io/txns/AauEeUjT1FtqQpCBrB2dFD5Vanoe2JtTW4YepMrx36Au) |
| 4 | `reserve_withdraw` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [FyWgiABJ…](https://testnet.nearblocks.io/txns/FyWgiABJbWHHN1HhSUscBXeW67idZiUdcKUfEo4FREzh) |
| 4 | `reserve_withdraw` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [CYftuu7K…](https://testnet.nearblocks.io/txns/CYftuu7KcqwnW4GU5r9HSw7iYKgNuDQCeMnZ6wUAEgw3) |
| 4 | `claim_reserve` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [Fj9tLrL5…](https://testnet.nearblocks.io/txns/Fj9tLrL56dHuBtL6GMahj35ggPSECkHnxY37RX9A8Hoa) |
| 4 | `claim_reserve` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [C51GWaFk…](https://testnet.nearblocks.io/txns/C51GWaFkeCX8M2RKD5xsK6ddxcrNxdgt8CnLMBEw552D) |
| 4 | `claim_reserve` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [E2169Z53…](https://testnet.nearblocks.io/txns/E2169Z53bGoeWMnDaUa5VTXrotjbyhD7CRjnuuag9LPx) |
| 4 | `add_key` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [BBVVtQHD…](https://testnet.nearblocks.io/txns/BBVVtQHDAMJRrqvDgCc6g1R5p221NnCA5kdcpJYzpqKP) |
| 4 | `add_key` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [CKbfGtwA…](https://testnet.nearblocks.io/txns/CKbfGtwAq9gXScoUBKN3izWRHSQEb7kfKTAMFAiR48Ht) |
| 4 | `add_key` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [A6zbyQqA…](https://testnet.nearblocks.io/txns/A6zbyQqAx8b3KG8sDWecogXpQrDcKWjhpqpwpx1UnUd) |
| 4 | `add_full_access_key` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [3p6T4ebL…](https://testnet.nearblocks.io/txns/3p6T4ebLLasbYBmdB8NLcfdaB8XhAorZ4shAqGxjFbY7) |
| 4 | `add_full_access_key` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [FiSUc8wr…](https://testnet.nearblocks.io/txns/FiSUc8wrv7hYsoqbX8DDj45PQKHbX5nD9NRrTHBq96zE) |
| 4 | `add_full_access_key` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [G3vDFW9w…](https://testnet.nearblocks.io/txns/G3vDFW9wHw5SncNMpmxnKnSbQvfUxQJLZotGEKreNYuz) |
| 4 | `delete_account` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [8CxDiJkU…](https://testnet.nearblocks.io/txns/8CxDiJkUnHadBsSSnhb6iNTpyuZguqNJRW86MmZa4ba4) |
| 4 | `delete_account` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [F9KjTZ7L…](https://testnet.nearblocks.io/txns/F9KjTZ7L48GQybnAxgqGQdhD6MXgeuTj3jurfUN2x42o) |
| 4 | `delete_account` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [9zWoo3NQ…](https://testnet.nearblocks.io/txns/9zWoo3NQnQuxEpuPhRF1ZRNgJUThdbjY67XrXyKofQrr) |
| 4 | `update` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [BuPz4HCk…](https://testnet.nearblocks.io/txns/BuPz4HCkhyqwSKFYoroGbTzNZCrF59pBsYB3ExPqwvV4) |
| 4 | `update` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [Hf1xp38Z…](https://testnet.nearblocks.io/txns/Hf1xp38ZNBgVLhQSdZar3CTns3KvQgwqFfBSzkAAye4G) |
| 4 | `update` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [8qycgnvA…](https://testnet.nearblocks.io/txns/8qycgnvAEgnEUxKN8cdYFFijsA2Fzizd17m9PaGRkYaq) |
| 4 | `upgrade` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [36m3AqfJ…](https://testnet.nearblocks.io/txns/36m3AqfJSD6P64qjdbHm17xP22wKtMnnGUs3PQ2tbhjF) |
| 4 | `upgrade` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [DVXc57Py…](https://testnet.nearblocks.io/txns/DVXc57PySk4GtiRowpG4jaz5Jqmjxxjr9MnvZeA98NNb) |
| 4 | `upgrade` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [5w1UDKJd…](https://testnet.nearblocks.io/txns/5w1UDKJdiBP9wg5qD92M4PnGFua7tkYJQ8gof4bdyjk4) |
| 4 | `deploy` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [5HBecn6W…](https://testnet.nearblocks.io/txns/5HBecn6WQtLmT8bWgnKSF9g7vfcqdh8z4uSA8GSZbzqk) |
| 4 | `deploy` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [9ResCmMK…](https://testnet.nearblocks.io/txns/9ResCmMKRkZf2oqM1WQQeP1rYTwu4MA1EUdRMfVowtdk) |
| 4 | `deploy` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [DY9Z4ggJ…](https://testnet.nearblocks.io/txns/DY9Z4ggJssnWN5VptHRRzt9gQp2Tnj9B3kBmMZV3VBR3) |
| 4 | `migrate` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [HBKkVEk3…](https://testnet.nearblocks.io/txns/HBKkVEk3rh9wBA1H9qQcsgQ7gqCBTvDodgwKYsf2q2JA) |
| 4 | `migrate` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [HauMt5SD…](https://testnet.nearblocks.io/txns/HauMt5SDwsoAygTyGSE2RUMCNMuMva2seyQ5XugdS9f6) |
| 4 | `migrate` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [8TFYdY2Z…](https://testnet.nearblocks.io/txns/8TFYdY2ZWESC7zDQv9WwoUvk77u1qGTkdnzmB9Vh1J68) |
| 5 | `withdraw` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [DPuSntqV…](https://testnet.nearblocks.io/txns/DPuSntqVuhsUKg6Swkpkn9AphBYwmdcKvf91ZNVA4fxu) |
| 6 | `withdraw` → `box-0007.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [8MnQubSR…](https://testnet.nearblocks.io/txns/8MnQubSRYwJY8vFNJcapxyjsKUKfABa5L3GEwfSDTc11) |
| 6 | `withdraw` → `box-0007.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [dGsMdeqr…](https://testnet.nearblocks.io/txns/dGsMdeqrrMcMs3jYimWWMCtMxrFxvb3oUavwauFLHYp) |
| 6 | `withdraw` → `box-0007.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [CjmnTh41…](https://testnet.nearblocks.io/txns/CjmnTh41RJ6iEgzQySg6NkCcKTnXPGQUYTiWGRctgKC9) |
| 6 | `withdraw` → `box-0007.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | refused (expected) | [J6QyHJQi…](https://testnet.nearblocks.io/txns/J6QyHJQiwEbEPDJd1V33DKXymJ8o5so7XXzMo8i5vjen) |

#### testnet-t7 (pid 3096945)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [36YExKER…](https://testnet.nearblocks.io/txns/36YExKERQUhcaPxbybqYt8QQfxnibAUyGoHAwFTDTJGU) |
| 1 | `attest_inspection` → `box-0008.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [3kR7TZbP…](https://testnet.nearblocks.io/txns/3kR7TZbPQJnMwcXEEd7cjQhCEFi9PaKSzgi8cXm2fwJL) |
| 2 | `start_lease` → `box-0008.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [2e5mD2JM…](https://testnet.nearblocks.io/txns/2e5mD2JMsuW3JZ1CQxJiHuqPPgNnDdeXaNZAC2PEUd5Y) |
| 4 | `pay_hire` → `box-0008.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [2RJuTMxU…](https://testnet.nearblocks.io/txns/2RJuTMxUsu9f2CzBNxVjm2qagcXCpxCgUdEczhJpBNRa) |
| 5 | `list` → `box-0008.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [9FnguvAa…](https://testnet.nearblocks.io/txns/9FnguvAaszG5hFYTmLYt3xprqKdkPxvMHQt5NrQtTM7G) |
| 6 | `buy` → `box-0008.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [7WGKReGR…](https://testnet.nearblocks.io/txns/7WGKReGRaJNSKo7XopKiSY87yVB1dCN5GWYTx3QzqGPt) |
| 7 | `withdraw` → `box-0008.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [4k5zYxpk…](https://testnet.nearblocks.io/txns/4k5zYxpkx1VuaUNNDSGmWFBTFvazz88bc7HwPNyEPqxH) |

#### testnet-t4 (pid 3136943)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [FhMdxarp…](https://testnet.nearblocks.io/txns/FhMdxarpn7LR8XhKmSnwaFKoEhD7ieevHQRnBqpPTCtm) |
| 1 | `attest_inspection` → `box-0009.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [8sNDd86j…](https://testnet.nearblocks.io/txns/8sNDd86jctTUyVbZM1fTfFLNcxHmxUf7brKP1pMJZB8T) |
| 2 | `start_lease` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [BdL3VCG6…](https://testnet.nearblocks.io/txns/BdL3VCG6Gyu94ZCqZgNFPSvjkzWKgeeKpgdcm9CkbJW5) |
| 3 | `pay_hire` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [JAUk8F6f…](https://testnet.nearblocks.io/txns/JAUk8F6fyqtXWW1gLxv25jBxV8op1VjRjvqwSxDJ9YwV) |
| 4 | `CreateAccount` → `t4b1794513637.onhire-outsider-n12.testnet` | `onhire-outsider-n12.testnet` | ok | [AQRaRTJg…](https://testnet.nearblocks.io/txns/AQRaRTJgWAADkMKbBH4hHZ7uCGkWjiasaw6YphiuHoDw) |
| 4 | `list` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [2DeQgFEC…](https://testnet.nearblocks.io/txns/2DeQgFECr7nyBKTa9arX915s3a2Nrr1tz3Kfgfb1LSc2) |
| 5 | `Transfer` → `onhire-racer1-n12.testnet` | `v2.onhire.testnet` | ok | [DkGo4PEo…](https://testnet.nearblocks.io/txns/DkGo4PEojvYV7ijnynE3okPJq27H6KUDcfuehFaerZ9J) |
| null | `Transfer` → `onhire-racer1-n12.testnet` | `v2.onhire.testnet` | rejected (not executed) | [6rUMdU5C…](https://testnet.nearblocks.io/txns/6rUMdU5CS8bBhzEfGaqnV1uTMZKtp1KhYQNG16P27nNS) |
| 5 | `Transfer` → `onhire-racer1-n12.testnet` | `v2.onhire.testnet` | ok | [C6kDr62n…](https://testnet.nearblocks.io/txns/C6kDr62nHJm6JB2XWDAXYBtex1eDH9nxCM1iDW6Z63yF) |
| 5 | `buy` → `box-0009.v2.onhire.testnet` | `t4b1794513637.onhire-outsider-n12.testnet` | ok | [nQ9Sfo8M…](https://testnet.nearblocks.io/txns/nQ9Sfo8MNSZsBCQPMkg5LPMEZRYjrAWzCuiwPXMrxts) |
| 5 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-racer1-n12.testnet` | refused (expected) | [5EnS1NzF…](https://testnet.nearblocks.io/txns/5EnS1NzFYdtNuAsNGpcT3DQQ2mBY8b9TAxfQcxYPDwni) |
| 5 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | refused (expected) | [AD8cPNAN…](https://testnet.nearblocks.io/txns/AD8cPNANXZyyUisX9nVPUuXj6LAS3oxta7o41iFGCGtm) |
| 5 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [76RWFepG…](https://testnet.nearblocks.io/txns/76RWFepGtiPLBuWsHYmis8ZwsECjYsFDUKtrZrivUdpq) |
| 5 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [7Gz83QH3…](https://testnet.nearblocks.io/txns/7Gz83QH3DVDCRsTKLwT26DzMYXZcGSqreBPcuqSSPLp5) |
| 5 | `Transfer` → `v2.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [EZ35fkHq…](https://testnet.nearblocks.io/txns/EZ35fkHqU9cr8RHXWPxszLxYtQ2KDJPNc6adFP5vkvwp) |
| 6 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [9eoCJKb9…](https://testnet.nearblocks.io/txns/9eoCJKb9BmsXFxCVq4KyfTohCE5GS74h25FhX4rhziHb) |
| 6 | `on_nft_transfer` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [B5V68xor…](https://testnet.nearblocks.io/txns/B5V68xorvdF9UPoeMwnKvZVvCtLrydqnzHwxvWoizUE2) |
| 7 | `withdraw` → `box-0009.v2.onhire.testnet` | `t4b1794513637.onhire-outsider-n12.testnet` | ok | [B2bsYqtA…](https://testnet.nearblocks.io/txns/B2bsYqtAh6vT1dtxzewwK1pKNBT7DVjxLLQkEm3SuTWM) |
| 8 | `list` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [HKsGQYCE…](https://testnet.nearblocks.io/txns/HKsGQYCE1KD58Yv4x9YkeotQXrHhAv8XSG4wSt4uBGh8) |
| 9 | `buy` → `box-0009.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [GDSo8pvz…](https://testnet.nearblocks.io/txns/GDSo8pvzo1ymS6AspaBgUdGkcRLaufEftTpJuXMafni6) |
| 9 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [HX8rbrXM…](https://testnet.nearblocks.io/txns/HX8rbrXMjpeYwFgpDrZdJpePhGpdQq3snqvtWx15SdY9) |
| 9 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-racer1-n12.testnet` | refused (expected) | [6bhGMpfZ…](https://testnet.nearblocks.io/txns/6bhGMpfZjxsdTh42LdQZSn45QbEF5RHNrLMbiEqKNL3t) |
| 9 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [CWfqKA7v…](https://testnet.nearblocks.io/txns/CWfqKA7vuAw9kb7vPHnq5tUYYy8qYfeEuU11FDLPKkkC) |
| 9 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [7VXzR5P6…](https://testnet.nearblocks.io/txns/7VXzR5P6vjfouk2nWN5EfLN8sRYn6ywwnbGSk1E8aZSf) |
| 10 | `list` → `box-0009.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [4kQSTyeb…](https://testnet.nearblocks.io/txns/4kQSTyebC6HQc59W4ALmU6yhZ88q8n6nJBcrneuon3Ls) |
| 11 | `buy` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [14ezissH…](https://testnet.nearblocks.io/txns/14ezissHnXzBJsmsNvaFdCUtSqrSP6QKgKX11UDjKwes) |
| 11 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [At9LuSmW…](https://testnet.nearblocks.io/txns/At9LuSmWet2JFYws9XNDjzLhwYzXggxCie67t74oq6co) |
| 11 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [AndshwxR…](https://testnet.nearblocks.io/txns/AndshwxRW3xTTZhu8bii5yH1LLvmpBJJ4drJRXK9zdY) |
| 11 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [DqJzDnpY…](https://testnet.nearblocks.io/txns/DqJzDnpYP2HBfsKs71oS72wwDgCo62JhunVMTaYNDQm6) |
| 11 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [4YWV3qdE…](https://testnet.nearblocks.io/txns/4YWV3qdER7Nc5ai4zMaLU5Y539HpMPgMk8GrXeHLrSLu) |
| 12 | `list` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [21wfo3ue…](https://testnet.nearblocks.io/txns/21wfo3ue4MpgwMjNAigVWhTeWa9M2Ew8cBQKCu8seRdo) |
| 13 | `buy` → `box-0009.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [GkmyDvz6…](https://testnet.nearblocks.io/txns/GkmyDvz6zB3WtiCPRpUdKc1rjxaQoyKk2JEFt1WnB5wY) |
| 13 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | ok | [VwGoQcLa…](https://testnet.nearblocks.io/txns/VwGoQcLaHMndNggJfo5KuZezt4Vt3dXxVWDKaDhK6G9) |
| 13 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-racer1-n12.testnet` | ok | [51tT5kSV…](https://testnet.nearblocks.io/txns/51tT5kSVbonor1CQR6Ckd8KDgSf6zZ5DP1wnyAgBLtFd) |
| 13 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [6HYRk42U…](https://testnet.nearblocks.io/txns/6HYRk42UkQyJ1JpEEiW5qrqjoXBp7MHXT7QPHmWb8Wxe) |
| 13 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [GT8ujRnW…](https://testnet.nearblocks.io/txns/GT8ujRnWitVM1TGr3j56BfbS4J1AcpdFj3rjgiBK8P3K) |
| 14 | `list` → `box-0009.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [4tX8g4Vr…](https://testnet.nearblocks.io/txns/4tX8g4VrginrkY3DJj2TjXVrXj9nqg2h2RxKu7SuEb1v) |
| 15 | `buy` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [CL4sWYP5…](https://testnet.nearblocks.io/txns/CL4sWYP5SwLHrC8aFJLtjxEuEQZR2u2r2961CeyyhTqj) |
| 15 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [6vx8ei5N…](https://testnet.nearblocks.io/txns/6vx8ei5Ne8UnXMyDNAKoyhfNv8Bke4ocPx6N4sJJv27g) |
| 15 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-racer1-n12.testnet` | refused (expected) | [HLfEx14H…](https://testnet.nearblocks.io/txns/HLfEx14Hx7oQ9yedP2SdYBTQEo4RUrodfo1TUpLvfxBW) |
| 15 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [8MvgGQLE…](https://testnet.nearblocks.io/txns/8MvgGQLEGY1kipFLJaF9AH8LdU8P9iMhGojzReSqwta7) |
| 15 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [4pX2JFxn…](https://testnet.nearblocks.io/txns/4pX2JFxnFdjLAMdjaiKJr7UFtrkSuNzRNNgi8mNusfTc) |
| 16 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [HztvqxwV…](https://testnet.nearblocks.io/txns/HztvqxwVLm4izgHECD817drVPBDBzUpkAJXLvx2EqgYm) |
| 16 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [7QeoGAkx…](https://testnet.nearblocks.io/txns/7QeoGAkxMSVpnciJrTj2w8pcm63oLKAgi2vXB7cPnHpB) |
| 16 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [9FWtf8j3…](https://testnet.nearblocks.io/txns/9FWtf8j3qtwh395HXS5EqEumzKxpGKcgCqsXPp8k5hmF) |
| 16 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [4ZSiV6nf…](https://testnet.nearblocks.io/txns/4ZSiV6nfiNXnXCGs8USPifADSwWEgkDnMLzetqAvpLNp) |
| 16 | `rescue_sale` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [9SdfiYXz…](https://testnet.nearblocks.io/txns/9SdfiYXzSPh4k7DdMnL5dzmRpYygrQMWJerzo5QFkDxA) |
| 16 | `on_nft_transfer` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [EjM1N7vb…](https://testnet.nearblocks.io/txns/EjM1N7vbn9UjedBhdp2LaZjUr1V97F1y1uH2UhXPJ2LH) |
| 16 | `on_rescue_owner` → `box-0009.v2.onhire.testnet` | `onhire-outsider-n12.testnet` | refused (expected) | [FFTbn6g9…](https://testnet.nearblocks.io/txns/FFTbn6g96y1kfAjovzS84BdQueLf6tuSmDhWuwxAge7j) |
| 17 | `pay_hire` → `box-0009.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [7Ybaducu…](https://testnet.nearblocks.io/txns/7YbaducuwBmM7Vx7qYTXnji3KW36mBNsM5BKRh1X4sEt) |
| 18 | `withdraw` → `box-0009.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [EuAQjkBs…](https://testnet.nearblocks.io/txns/EuAQjkBsZiEZ88Mc9Ejdp5G6NcoqBBbpZ1m1pqAvuAtG) |
| 19 | `withdraw` → `box-0009.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | ok | [9qicMakm…](https://testnet.nearblocks.io/txns/9qicMakmvMizKi9Dwxnd1zDugfsQGa5dg5QsnG1uSF54) |

#### testnet-t5 (pid 3141046)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v2.onhire.testnet` | `v2.onhire.testnet` | ok | [Hsn2ZNXM…](https://testnet.nearblocks.io/txns/Hsn2ZNXM8z4YZNjkkn8Kc7P4Maz711qhLoqUDQhpirG8) |
| 1 | `attest_inspection` → `box-0010.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [4LFt3Kyr…](https://testnet.nearblocks.io/txns/4LFt3KyrLNvx4Q3Yz9gX2WJMExaUxWhPxR8tMX5fjpBh) |
| 2 | `start_lease` → `box-0010.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [2VFwbths…](https://testnet.nearblocks.io/txns/2VFwbthsWJZsuDFP6nxLzz4ptRU5VVkRQmQupZcymwyK) |
| 4 | `pay_hire` → `box-0010.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [3W3HMmxe…](https://testnet.nearblocks.io/txns/3W3HMmxeCUgDC3DTa1HL66k2bY6ahyujNf2hvb3NBPMu) |
| 5 | `off_hire` → `box-0010.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [7ZG2ATwV…](https://testnet.nearblocks.io/txns/7ZG2ATwVj9vqf6sWiaz9M1fSDh8qeVXDjAtWeqPoNZU8) |
| 6 | `withdraw` → `box-0010.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | ok | [CAes32hA…](https://testnet.nearblocks.io/txns/CAes32hAkUHQ9msNpWuhJQTo14txWQRDqK4Ca5pFEpMF) |
| 7 | `start_lease` → `box-0010.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [58xrq4GZ…](https://testnet.nearblocks.io/txns/58xrq4GZsnQtH98HMUQoQuDks2rHPkf1vKyFF812QFJP) |
| 8 | `attest_inspection` → `box-0010.v2.onhire.testnet` | `onhire-owner-n12.testnet` | refused (expected) | [DCbvzKyX…](https://testnet.nearblocks.io/txns/DCbvzKyXnzXwT7Mx3XxBMxoMJ2sqkfs8z4qgPEvnxajW) |
| 8 | `attest_inspection` → `box-0010.v2.onhire.testnet` | `onhire-buyer-n12.testnet` | refused (expected) | [8aqoAMbj…](https://testnet.nearblocks.io/txns/8aqoAMbjVWQA7wKVQuWQVHXpEDg4SiEfpVQKnqBYcHB5) |
| 8 | `attest_inspection` → `box-0010.v2.onhire.testnet` | `onhire-lessee-n12.testnet` | refused (expected) | [6Aj2otLG…](https://testnet.nearblocks.io/txns/6Aj2otLGhPH87nqPuPdDHDgK89ZxvXzJJQi3oreDqq6y) |
| 9 | `attest_inspection` → `box-0010.v2.onhire.testnet` | `onhire-inspector-n12.testnet` | ok | [BR3T1RNa…](https://testnet.nearblocks.io/txns/BR3T1RNaixN1npHLrNZWBkydqup2HbQuK9fWpWCAV9rZ) |
| 10 | `start_lease` → `box-0010.v2.onhire.testnet` | `onhire-owner-n12.testnet` | ok | [5UsxekT7…](https://testnet.nearblocks.io/txns/5UsxekT7WnuLkUyko4si8hFMq6ph8e5cEpPQyfeoUMZn) |

