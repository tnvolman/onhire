# Onhire — NEAR testnet proofs

## Current: two-step lease on `v3.onhire.testnet` (2026-11-13)

Run Friday 2026-11-13, 15:50–16:24 ET, on **NEAR testnet only** (chain id
`testnet`). Native test NEAR only, from the public faucet. No mainnet, no
real funds.

This deployment carries the two-step lease. The owner's `start_lease`
(exactly 1 yocto) writes a pending offer. Only the named lessee's
`accept_lease`, with exactly the offered deposit, starts the lease. The
lessee posts the deposit, and `off_hire` returns it to the lessee. Source:
parent commit `043b612` plus that change in `contracts/item/src/lib.rs`;
the collection contract is unchanged. The earlier boxes on
`v2.onhire.testnet` and `onhire.testnet` are keyless and cannot be
upgraded, so this proof uses new item code, a new collection and new boxes.

### Result

| Test | Testnet result | Checked steps | Box | Runner pid |
| --- | --- | --- | --- | --- |
| 1 — Mid-lease sale | **PASS** (48 s) | 14 | `box-0001.v3.onhire.testnet` | 869500 |
| 2 — Failed collection transfer undoes the sale | **PASS** (149 s) | 11 | `box-0002`, `box-0003` | 870473 |
| 3 — Direct transfer is blocked | **PASS** (57 s) | 16 | `box-0004` | 872720 |
| 4 — Stuck lock / rescue | **PASS on what a live chain can show** (412 s). The *stuck*-lock steps 4.1–4.3 need state patching and are proven in sandbox only. | 21 | `box-0005` | 873685 |
| 5 — Lapsed inspection and the two-step lease | **PASS** (344 s), after failed attempts described below | 15 | `box-0012` | 895521 |
| 6 — Reserve cut; owner cannot take it | **PASS** (104 s) | 8 | `box-0007` | 882624 |
| 7 — Exact 100-yocto fee splits | **PASS** (29 s) | 9 | `box-0008` | 884475 |

Result lines, copied from the logs:

```
[testnet-t1] RESULT: PASS (14 checked steps, 48 s)
[testnet-t2] RESULT: PASS (11 checked steps, 149 s)
[testnet-t3] RESULT: PASS (16 checked steps, 57 s)
[testnet-t4] RESULT: PASS (21 checked steps, 412 s)
[testnet-t5] RESULT: PASS (15 checked steps, 344 s)
[testnet-t6] RESULT: PASS (8 checked steps, 104 s)
[testnet-t7] RESULT: PASS (9 checked steps, 29 s)
```

All 94 checked steps of these seven passing runs printed
`INVARIANT OK, ZERO KEYS`. After every step the runner checks rule 14 twice:
from the box's `get_invariant_parts`, and separately from the RPC account
view. It also checks that the box account has zero access keys. After the
run, all twelve boxes `box-0001` … `box-0012.v3.onhire.testnet` were
re-read from the chain. Each had **0 access keys**, `code_hash = 1111…`,
`global_contract_hash = 7MgWbdYK2R9Wyb4DZ4zFDcpwohBjz6rDPQnmExhPiL8g`,
and `get_invariant_parts.holds = true`.

The fee account gained exactly 44,234,567,890,123,456,789,014 yocto over the
run (balance minus its 0.1 NEAR creation balance). That equals the sum of
`get_fees_paid` over all twelve boxes, to the yocto.

### Lease-fix evidence (t5, box-0012)

| What | Expected | Result | Tx |
| --- | --- | --- | --- |
| Owner `start_lease(lessee, 0.01 NEAR/day, 10 d, deposit 0.1 NEAR)` with 1 yocto | offer written, no lease, no deposit held | ok; `get_lease` = null | [7VVgAJFa…](https://testnet.nearblocks.io/txns/7VVgAJFaRD9YZAAc8MJQr9A9at66paHGhH1vGnt5GhpD) |
| `accept_lease` by the owner (0.1 NEAR attached) | refused | `ERR_NOT_LESSEE` | [EUigqXxs…](https://testnet.nearblocks.io/txns/EUigqXxse25v9aUixGeeybQhH7mkb6bhQXazpZyfrWxd) |
| `accept_lease` by an outsider | refused | `ERR_NOT_LESSEE` | [9pvJ7mKv…](https://testnet.nearblocks.io/txns/9pvJ7mKvhAGhoqeSSpQYrSf286VLPPgUpZcipHj7Rfow) |
| `accept_lease` by the lessee with deposit − 1 yocto | refused | `ERR_WRONG_DEPOSIT` | [8kZwcNBr…](https://testnet.nearblocks.io/txns/8kZwcNBr1DUC4noDLzKfSupqZA7ePKX19ae1i29QsHJc) |
| `accept_lease` by the lessee with deposit + 1 yocto | refused | `ERR_WRONG_DEPOSIT` | [3h5Hw4zJ…](https://testnet.nearblocks.io/txns/3h5Hw4zJ9BTP1WPuDHMD1dynK4N4ijPqp6yRF6B7tGbw) |
| `accept_lease` by the lessee with exactly 0.1 NEAR | lease starts; box holds the lessee's deposit | ok; lessee balance −0.100181 NEAR (deposit + gas); `held_lessee_deposits` = 0.1 NEAR | [GyukYEe9…](https://testnet.nearblocks.io/txns/GyukYEe99mvkmHjYy4snZTa4gGqU1CLeRDATPAaBK5Ya) |
| `pay_hire` 0.1 NEAR after the inspection lapsed | ok, exactly 1% to the fee account | ok (+0.001 NEAR) | [9z3VWGtU…](https://testnet.nearblocks.io/txns/9z3VWGtULRBjYr7uyksTwGudK8ifsjx9yGP4uLLbaxL6) |
| Lessee `off_hire` | deposit credited to the lessee; owner credited nothing | ok; owner pending 0 | [7kFE4Syf…](https://testnet.nearblocks.io/txns/7kFE4SyfJZZGr55ES6D8RVAaMcwpB7o3EgZekHhUg7Tp) |
| Lessee `withdraw` | exactly the 0.1 NEAR the lessee posted | ok, 0.1 NEAR | [4vKU2bQC…](https://testnet.nearblocks.io/txns/4vKU2bQC3LJB414wtiEDFtkEALwuGequRx94Vf7iQrNy) |
| **Old attack:** owner `start_lease` with 0.1 NEAR attached | refused | `Requires attached deposit of exactly 1 yoctoNEAR` | [3efQyVGd…](https://testnet.nearblocks.io/txns/3efQyVGd8YygjTrpoCqKwznGV1FaqJfEo9LgJNmAvv6e) |
| Owner `start_lease` (1 yocto) after the lapse | refused | `ERR_INSPECTION_LAPSED` | [8UENtcu9…](https://testnet.nearblocks.io/txns/8UENtcu9GXRj1K5C452VQo7zUHLT7YLsD8NZow1DmWzC) |
| Inspector re-attests; new offer; lessee accepts | new lease | ok | [9z7ZUUah…](https://testnet.nearblocks.io/txns/9z7ZUUahvLkHmdEWVTEvwu1qRUfBWJrZrTACMB8DHnYT), [4KwbyPTe…](https://testnet.nearblocks.io/txns/4KwbyPTejoockfYywPsMfzNTJr2UYXn1wNqikayZiBqz) |

The lapse was read pinned to a block past `valid_until`: block 277564715
(ts 1794604988 > valid_until 1794604983) returned
`{"inspector":"onhire-inspector-n13.testnet","valid_until":1794604983,"valid_now":false}`.

Tests 1, 2, 4, 6 and 7 also lease through `start_lease` (1 yocto) +
`accept_lease` (lessee deposit). For example, t1's offer is
[CUo8U1n9…](https://testnet.nearblocks.io/txns/CUo8U1n9GthscNzntDdPaa2VTw8L31SnfwxL24u8muCV) and its accept is
[DjdUyJsT…](https://testnet.nearblocks.io/txns/DjdUyJsTj6mxDpxVFDqekEEjVLFWZzTsb4YdFh1scXdB). In t1 the lease and
its lessee deposit stayed with the box through the mid-lease sale.

### Commands and environment

```bash
export PATH="$HOME/.cargo/bin:$PATH" OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include
export NEAR_TESTNET_RPC=https://rpc.testnet.fastnear.com

# contracts were built by the sandbox run (scripts/test-sandbox.sh), see docs/SANDBOX-PROOF.md

# 1. fresh throwaway role accounts (faucet, 10 test NEAR each). ONHIRE_COLLECTION points
#    at the existing parent so the script skips it and only creates roles.
ONHIRE_COLLECTION=onhire.testnet scripts/testnet/create-accounts.sh n13

export ONHIRE_SUFFIX=n13 ONHIRE_COLLECTION=v3.onhire.testnet ONHIRE_FEE_ACCOUNT=onhire-fee-n13.testnet

# 2. top up the parent (it had ~0.19 NEAR free above its storage stake, too little to
#    fund the new sub-account): the runner's topup, pointed at the parent for this one call
ONHIRE_COLLECTION=onhire.testnet target/debug/onhire-testnet topup

# 3. fee account + v3 sub-account + donations (inspector 8, racer1 8, owner 4, buyer2 3)
target/debug/onhire-testnet fee-setup
# 4. collection wasm, new(minter, fee_account), set_item_code(global = true)
target/debug/onhire-testnet deploy

# 5. the seven tests (15:52–16:08 ET), then t5 again (final pass 16:18–16:24 ET)
scripts/test-testnet.sh t1 t2 t3 t4 t5 t6 t7
scripts/test-testnet.sh t5

# 6. checks
scripts/testnet/audit-nonces.sh
scripts/testnet/proof-links.sh <pid> …
```

The runner only accepts account ids that end in `.testnet`, and its worker
is `near_workspaces::testnet()`.

### Accounts (all throwaway, testnet)

| Role | Account |
| --- | --- |
| Collection / factory / minter (keeps its deployer key, as disclosed in the README) | [`v3.onhire.testnet`](https://testnet.nearblocks.io/address/v3.onhire.testnet), sub-account of `onhire.testnet`, created [7ona6pNh…](https://testnet.nearblocks.io/txns/7ona6pNhdfa4m2Hj9dftSvuXM1paX7GoJcGXq3gfg3ij) |
| Protocol fee recipient | [`onhire-fee-n13.testnet`](https://testnet.nearblocks.io/address/onhire-fee-n13.testnet), created through the `testnet` registrar [8LHTLurG…](https://testnet.nearblocks.io/txns/8LHTLurGFyRofxyj4JjbeefWKpNwmai97tfzodZJeRfh) |
| Owner | `onhire-owner-n13.testnet` ([ARE2vAwS…](https://testnet.nearblocks.io/txns/ARE2vAwSSWcW4Gh6NzofTd28BqocTWrRjZtqLMiMSHJw)) |
| Lessee | `onhire-lessee-n13.testnet` ([DNtj3ax2…](https://testnet.nearblocks.io/txns/DNtj3ax22DeKU26FbBXcRFkR77iRPpTtcTw8zYKR5TfB)) |
| Buyer | `onhire-buyer-n13.testnet` ([8VeuBSoe…](https://testnet.nearblocks.io/txns/8VeuBSoebyqrmwm6GbMVNA9KYFSauGR5oB7J5RhZL3e4)) |
| Inspector | `onhire-inspector-n13.testnet` ([7XL6prjX…](https://testnet.nearblocks.io/txns/7XL6prjXL1GPbpk2DkVb3gnuDxkAgZkfjkR2zuX9CPi7)) |
| Outsider (rescue caller; parent of the fresh buyers) | `onhire-outsider-n13.testnet` ([6k9FE3BE…](https://testnet.nearblocks.io/txns/6k9FE3BEde2DeQ6hGvtXpR15bDjYweqFucF4j5xmyGDE)) |
| Racer (rescue caller; drain sink in t4) | `onhire-racer1-n13.testnet` ([HbQP56CJ…](https://testnet.nearblocks.io/txns/HbQP56CJbwN9myXBncHqNNcuFgv7MyEMzAj4gus6aUNp)) |
| buyer2 (top-up and funding donor) | `onhire-buyer2-n13.testnet` ([8ivqo37u…](https://testnet.nearblocks.io/txns/8ivqo37uAy5kvLHjJsszmX9jhdfAHDCu6FbXQxngsZAd)) |
| Fresh buyers (sub-accounts of the outsider, never owned a token) | `t2b1794603213.onhire-outsider-n13.testnet` (t2), `t4b1794603511.onhire-outsider-n13.testnet` (t4) |
| Created, unused | `onhire-racer2-n13`, `onhire-racer3-n13`, `onhire-funder1-n13` (`.testnet`) |

The faucet refused the eleventh creation, `onhire-funder2-n13.testnet`,
with `Rate limit exceeded, retry in 12 hours`, so `funder2` and `funder3`
do not exist. The runner does not use them.

### Setup and deploy

| Step | Tx |
| --- | --- |
| Top-up 6 test NEAR `onhire-buyer2-n13` → `onhire.testnet` | [2pAbZxfJ…](https://testnet.nearblocks.io/txns/2pAbZxfJTCReA5Y5FgX2GonM3E1yUpZNtvUX6B75mW81) |
| Create `onhire-fee-n13.testnet` | [8LHTLurG…](https://testnet.nearblocks.io/txns/8LHTLurGFyRofxyj4JjbeefWKpNwmai97tfzodZJeRfh) |
| Create `v3.onhire.testnet` (initial balance: the parent's free balance minus 0.2 NEAR) | [7ona6pNh…](https://testnet.nearblocks.io/txns/7ona6pNhdfa4m2Hj9dftSvuXM1paX7GoJcGXq3gfg3ij) |
| Fund collection: inspector 8, racer1 8, owner 4, buyer2 3 test NEAR (collection then 28.99 NEAR) | [R4H1aCmd…](https://testnet.nearblocks.io/txns/R4H1aCmdjSnJ7cb8fUneSYXnoAhbkmqKxKwkFwxYLh6), [4jXmWZHg…](https://testnet.nearblocks.io/txns/4jXmWZHgJFmp1aC5L8b3P4y3cQEip2WB9LDpHBFuCQCb), [9pmSruNw…](https://testnet.nearblocks.io/txns/9pmSruNwWMYZusQTrJUi5CzTLyzdbcBfBgpvwVwzwJ17), [5v4uS8j2…](https://testnet.nearblocks.io/txns/5v4uS8j2eiP7wmXf6ReRDfHhxQeSWDu7SAxJRQyFY6tB) |
| Deploy `onhire_collection.wasm` | [DX8EvbUG…](https://testnet.nearblocks.io/txns/DX8EvbUG5DnzVkQGZhvcja38jfyMRp39sXofPMEsdPcm) |
| `new(minter, fee_account = onhire-fee-n13.testnet)` | [37fiicAe…](https://testnet.nearblocks.io/txns/37fiicAessmHVepmD2AxKkufzGWSeYcqzS77sxybUNLT) |
| `set_item_code(global = true)`, NEP-591 global contract, cost 18.242731 test NEAR for 182,699 bytes | [HrMNaVk1…](https://testnet.nearblocks.io/txns/HrMNaVk1gSx8v17qZwiXeLci77hTdMSg5rxeMt8mmgR5) |

After deploy the collection held 10.72 test NEAR. At the end it held
4.62 test NEAR (about 2.32 NEAR above its storage stake).

### Wasm hashes (sha256)

| File | sha256 | On chain |
| --- | --- | --- |
| `onhire_item.wasm` (182,699 B) | `5e726984173c57c5d04f3c1e72624c94f6fdd235872e28063d769ce9301f9671` | `get_item_code_info` → `["XnJphBc8V8XQTzwecmJMlPb90jWHLigGPXac6TAflnE=","Global"]` (base64), and every box's `global_contract_hash` `7MgWbdYK2R9Wyb4DZ4zFDcpwohBjz6rDPQnmExhPiL8g` (base58). Both decode to this hash. |
| `onhire_collection.wasm` (222,098 B) | `e1036df87063267aea6d1daf0e9ccdbf9511911e2e240e6036d76dd4f64d2fe7` | `v3.onhire.testnet` `code_hash` `G9MmNtWYaQgTZyTzVmNfu8mK4cvmbYfHsaSmzJ8iKdUz` (base58) decodes to this hash. |

These are the same files the sandbox run in `docs/SANDBOX-PROOF.md` used.
They were built with `cargo near build non-reproducible-wasm --no-abi`, so
another machine is not expected to reproduce them byte for byte.

### t5: failed attempts and runner changes

The contracts were not changed for any retry. Only the testnet runner
(`tests/src/bin/onhire-testnet.rs`, test 5) changed.

1. **First run (pid 880482, `box-0006`): FAIL after 1 step.** The
   inspector's attest [AaP7PNcy…](https://testnet.nearblocks.io/txns/AaP7PNcynJ8vuTLj7VkHhwNaTeuqotj9UuftwWQfNWuH)
   was dropped before inclusion (unknown to two RPCs). The runner waits
   90 s before it resends a dropped tx. The resend
   [Eqpd3APS…](https://testnet.nearblocks.io/txns/Eqpd3APSTEiQVq5QxNsQABC6RKzryXnsvRVEJ6exsvF1) executed at block
   time 1794603953, after its `valid_until` 1794603873 (now + 60 s), so the
   box correctly refused it with `ERR_VALID_UNTIL_NOT_IN_FUTURE`. The
   appendix's generic "refused (expected)" label on that tx is wrong: the
   runner did not expect the refusal.
2. **Rerun, unchanged (pid 885685, `box-0009`): FAIL after 1 step**, the
   same way (dropped [HqQGHUgw…](https://testnet.nearblocks.io/txns/HqQGHUgwqdV8sUFJL9mfC3ss74eFewMBgXsh2aGPytG2),
   late resend [3ECP8Ueo…](https://testnet.nearblocks.io/txns/3ECP8Ueo6ZD6SgH4A1w74EgrWBePqsjLDP4m29bEKkY6)).
   Runner change: the test-only inspection window went from 60 s to
   300 s, and the wait-for-lapse limit from 300 s to 600 s. A dropped and
   resent attest, offer or accept now still lands inside the window.
3. **Rerun (pid 888814, `box-0010`): FAIL after 3 steps**, on the runner's
   own balance check. The lessee's balance dropped 0.2509 NEAR on a
   0.1 NEAR accept, because unused prepaid gas had not been refunded yet
   when the runner read it. The runner allowed only 0.1 NEAR of gas.
   Runner change: wait 5 s, then allow up to 0.25 NEAR of gas above the
   deposit, and print the drop. The exact check is box-side
   (`held_lessee_deposits` = deposit).
   `box-0010` was left with that lease active and the lessee's 0.1 NEAR
   deposit held. The lessee can still `off_hire` it and `withdraw` it.
4. **Rerun (pid 889249, `box-0011`): PASS, 14 steps.** Lessee drop on
   accept after the wait: 0.100181 NEAR.
5. Runner change: the wrong-accept refusals (owner, outsider, deposit
   ±1 yocto) were added to testnet t5, as in sandbox t5. The first version
   left them out to fit the 60 s window.
6. **Final run (pid 895521, `box-0012`): PASS, 15 steps.** This is the
   run in the result table.

Logs from failed attempts are kept separately and were not overwritten.

### Limitations (honest notes)

- **Test 4** cannot create a *stuck* lock on a live chain, since there is
  no state patching. On testnet it races `rescue_sale` against real
  in-flight sales and forces a real failed transfer. Steps 4.1–4.3 (stuck
  locks) are proven in sandbox only. The largest lock-to-callback gap seen
  in t1, t2, t4 and t7 was **2 blocks** (`SALE_UNDO_DELAY_BLOCKS` = 32).
- **Test 2** forces the collection transfer to fail by having the
  collection (deployer key) send its free balance to the outsider,
  then restoring it. Test 4 does the same with `racer1` for its
  failed-transfer round.
- The fail-closed fee revert (`ERR_FEE_EXCEEDS_OWNER_SHARE`) needs state
  patching. It is proven in unit tests and sandbox t7, not on testnet.
- The stale-offer refusal (`ERR_STALE_OFFER`), refusals with an offer
  already pending, and cancel (`cancel_lease_offer`) are proven in unit
  tests only. Accept and cancel refusals under a sale lock are proven in
  unit tests and sandbox t4. None of these were run on testnet.
- **Nonce audit:** `scripts/testnet/audit-nonces.sh` shows **OK** for every
  account this run created or signed with: all `onhire-*-n13.testnet`
  roles, `onhire-fee-n13.testnet`, `v3.onhire.testnet` and both fresh
  buyers. On-chain counts match the recorded txs, for example owner 53/53,
  lessee 43/43, inspector 22/22, buyer 26/26, `v3` 23/23. The two dropped
  inspector txs are in the record. The audit is nonce-based, so a dropped
  tx's skipped nonce also counts. The script checks every key in the key
  directory against this checkout's record file, which holds only this
  run. So the earlier `n12` accounts, `v2.onhire.testnet`, and
  `onhire.testnet` (50 on chain, 1 recorded here: the `v3` creation) show
  mismatches. Those earlier txs are recorded with the earlier proofs, not
  in this record file.
- Unaudited prototype. Testnet only.

### Appendix: every recorded transaction of this run

Generated with `scripts/testnet/proof-links.sh`. "refused (expected)" marks
a call that failed in the contract, which the tests expect except where
noted above. "rejected (not executed)" marks a tx that was dropped before
inclusion.

#### testnet-topup (pid 868452)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `Transfer` → `onhire.testnet` | `onhire-buyer2-n13.testnet` | ok | [2pAbZxfJ…](https://testnet.nearblocks.io/txns/2pAbZxfJTCReA5Y5FgX2GonM3E1yUpZNtvUX6B75mW81) |

#### testnet-fee-setup (pid 868507)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| null | `create_account` → `testnet` | `onhire-buyer2-n13.testnet` | ok | [8LHTLurG…](https://testnet.nearblocks.io/txns/8LHTLurGFyRofxyj4JjbeefWKpNwmai97tfzodZJeRfh) |
| null | `CreateAccount` → `v3.onhire.testnet` | `onhire.testnet` | ok | [7ona6pNh…](https://testnet.nearblocks.io/txns/7ona6pNhdfa4m2Hj9dftSvuXM1paX7GoJcGXq3gfg3ij) |
| null | `Transfer` → `v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [R4H1aCmd…](https://testnet.nearblocks.io/txns/R4H1aCmdjSnJ7cb8fUneSYXnoAhbkmqKxKwkFwxYLh6) |
| null | `Transfer` → `v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [4jXmWZHg…](https://testnet.nearblocks.io/txns/4jXmWZHgJFmp1aC5L8b3P4y3cQEip2WB9LDpHBFuCQCb) |
| null | `Transfer` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [9pmSruNw…](https://testnet.nearblocks.io/txns/9pmSruNwWMYZusQTrJUi5CzTLyzdbcBfBgpvwVwzwJ17) |
| null | `Transfer` → `v3.onhire.testnet` | `onhire-buyer2-n13.testnet` | ok | [5v4uS8j2…](https://testnet.nearblocks.io/txns/5v4uS8j2eiP7wmXf6ReRDfHhxQeSWDu7SAxJRQyFY6tB) |

#### testnet-deploy (pid 868708)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `DeployContract` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [DX8EvbUG…](https://testnet.nearblocks.io/txns/DX8EvbUG5DnzVkQGZhvcja38jfyMRp39sXofPMEsdPcm) |
| 0 | `new` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [37fiicAe…](https://testnet.nearblocks.io/txns/37fiicAessmHVepmD2AxKkufzGWSeYcqzS77sxybUNLT) |
| 0 | `set_item_code` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [HrMNaVk1…](https://testnet.nearblocks.io/txns/HrMNaVk1gSx8v17qZwiXeLci77hTdMSg5rxeMt8mmgR5) |

#### testnet-t1 (pid 869500)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [CSqjRKvK…](https://testnet.nearblocks.io/txns/CSqjRKvKRK47uxGH6uPyXKTgULccgcHKQ1D992rgFwr3) |
| 1 | `attest_inspection` → `box-0001.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [7ggXDLkp…](https://testnet.nearblocks.io/txns/7ggXDLkpso3CHHo6EQ5YB2nHzkmctgc5iaQZj8CKHmHQ) |
| 2 | `start_lease` → `box-0001.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [CUo8U1n9…](https://testnet.nearblocks.io/txns/CUo8U1n9GthscNzntDdPaa2VTw8L31SnfwxL24u8muCV) |
| 3 | `accept_lease` → `box-0001.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [DjdUyJsT…](https://testnet.nearblocks.io/txns/DjdUyJsTj6mxDpxVFDqekEEjVLFWZzTsb4YdFh1scXdB) |
| 4 | `pay_hire` → `box-0001.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [FGSC5mje…](https://testnet.nearblocks.io/txns/FGSC5mjeKgWTMSFWYx2GbYwV1UsuvkJ3CjUNMUuhVSu1) |
| 5 | `list` → `box-0001.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [4MGch628…](https://testnet.nearblocks.io/txns/4MGch6289bu1yAT1RQZNxychgS4ucjhCmDMzEvz574yC) |
| 6 | `buy` → `box-0001.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [3k8qxauF…](https://testnet.nearblocks.io/txns/3k8qxauFrpJXeFwopxF9CoBA5bCyCPeHaURhM4Ngs3GB) |
| 7 | `buy` → `box-0001.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [DukTwPev…](https://testnet.nearblocks.io/txns/DukTwPev6PPVWNRedXeqijYMuHbpkkC8vnH1r2xXtCTP) |
| 9 | `pay_hire` → `box-0001.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [HvJntAnw…](https://testnet.nearblocks.io/txns/HvJntAnwJcmZ2nPyPNG4Gef7B6pXWZu3WYbBcjHVUcGL) |
| 10 | `withdraw` → `box-0001.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [FmwLWv1n…](https://testnet.nearblocks.io/txns/FmwLWv1nE7qky7gGxvcxwfTFdSWJs8dEkhXeN1GUtnFN) |
| 11 | `withdraw` → `box-0001.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [Eesbo6d8…](https://testnet.nearblocks.io/txns/Eesbo6d8QWBpDJmaj8vgHcabjWMUY93LHBK1VQzP9FsV) |
| 12 | `withdraw` → `box-0001.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [64pH4oyu…](https://testnet.nearblocks.io/txns/64pH4oyuDHYzPB8qpw61M224YRFp9Fq6rF4Xh5oSca4J) |
| 13 | `withdraw_reserve` → `box-0001.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [CjgMDmzm…](https://testnet.nearblocks.io/txns/CjgMDmzmzv3jraRrKVEuBeKo1gs1wXqEU1zKKt9jsWBt) |

#### testnet-t2 (pid 870473)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [9mRRLwj5…](https://testnet.nearblocks.io/txns/9mRRLwj5Gku9LguZPYZaQmq7aeoZ9FK2kckEAqgn4Ju3) |
| 1 | `attest_inspection` → `box-0002.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [DMQCYLME…](https://testnet.nearblocks.io/txns/DMQCYLMEx6QLNHA62ASFJ9arYu4izNG9ot18Cn2DBUU6) |
| 2 | `start_lease` → `box-0002.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [J1XZ5how…](https://testnet.nearblocks.io/txns/J1XZ5howPLW8PVEkpsftZkoRGJYatiWTsXEdPQMeygEX) |
| 3 | `accept_lease` → `box-0002.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [DC5hWAQ5…](https://testnet.nearblocks.io/txns/DC5hWAQ5vowscTFfHZm8FNVVsYSywjac1wKLEVRnCy2s) |
| 4 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [5zZ7Fn8w…](https://testnet.nearblocks.io/txns/5zZ7Fn8wrrny7raEyHd5PTKc6seVYgQe1iaz2Dab8GQZ) |
| 5 | `pay_hire` → `box-0002.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [F9wsx9wH…](https://testnet.nearblocks.io/txns/F9wsx9wHVsaV9Nj8Focw9Q4gYEP4hS1AnG3Bpc7u3pte) |
| 6 | `list` → `box-0002.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [DFXGDvo2…](https://testnet.nearblocks.io/txns/DFXGDvo2CdqUC69qb2zEdpriGN79UxV5Ca2zutce9XwP) |
| 7 | `CreateAccount` → `t2b1794603213.onhire-outsider-n13.testnet` | `onhire-outsider-n13.testnet` | ok | [9LCZBJaG…](https://testnet.nearblocks.io/txns/9LCZBJaG3wuZwbse8k1mYd9ptJYW7MRmF3Yeo43L8K3G) |
| 7 | `Transfer` → `onhire-outsider-n13.testnet` | `v3.onhire.testnet` | ok | [vQNKhpjH…](https://testnet.nearblocks.io/txns/vQNKhpjHgDFYMdzXpWQg46DwTQoUSXrTKQmcu1NwEx7) |
| null | `Transfer` → `onhire-outsider-n13.testnet` | `v3.onhire.testnet` | rejected (not executed) | [Hf9TRMNQ…](https://testnet.nearblocks.io/txns/Hf9TRMNQKWJeahq8xAbzYf3rpWrZYEsP1xRnP2cZmQJH) |
| 7 | `Transfer` → `onhire-outsider-n13.testnet` | `v3.onhire.testnet` | ok | [8dKB4t4w…](https://testnet.nearblocks.io/txns/8dKB4t4wdjPgJUG63TeEwR8dfhWGGaWKRbiAL7PihUmB) |
| 7 | `buy` → `box-0002.v3.onhire.testnet` | `t2b1794603213.onhire-outsider-n13.testnet` | ok | [7wgE5wjq…](https://testnet.nearblocks.io/txns/7wgE5wjqw7nveYLFPmFtiDLzdb6wfJ6RPWmyAA8zo4hF) |
| 7 | `Transfer` → `v3.onhire.testnet` | `onhire-outsider-n13.testnet` | ok | [8mCuznA7…](https://testnet.nearblocks.io/txns/8mCuznA7RQvopiou4CW78weAEQWTVL6ELJuWNzARiyJ4) |
| 9 | `withdraw` → `box-0002.v3.onhire.testnet` | `t2b1794603213.onhire-outsider-n13.testnet` | ok | [2SC2bb7w…](https://testnet.nearblocks.io/txns/2SC2bb7wHrF21UiTerC3iBpNciibxkMuFRGe9Vsm6y3P) |
| 10 | `buy` → `box-0002.v3.onhire.testnet` | `t2b1794603213.onhire-outsider-n13.testnet` | ok | [Ho3RMZGr…](https://testnet.nearblocks.io/txns/Ho3RMZGrU8Sudfc2uSWhVKbBkAP6xrpU7Yx4YnRNSVJ4) |

#### testnet-t3 (pid 872720)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [HidyDEq8…](https://testnet.nearblocks.io/txns/HidyDEq8ZMTXrw8NakpfSvhWYkYiHCNwKJdggPFeuMfG) |
| 1 | `nft_transfer` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [BvdbPCgL…](https://testnet.nearblocks.io/txns/BvdbPCgLQtRq2cdg94bDTveW68FqvAz9G3jigtpd2ETi) |
| 2 | `nft_transfer_call` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [4Fz9XPpc…](https://testnet.nearblocks.io/txns/4Fz9XPpc6RBepAUfFKXJXMZTNJE7APy45oXq1hniQd32) |
| 3 | `nft_approve` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [CJaRbNft…](https://testnet.nearblocks.io/txns/CJaRbNftrTQLRCA2yNUaFg8wuBhHBJKCXC9XkqHSXxLq) |
| 4 | `nft_revoke` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [HtgVeqVo…](https://testnet.nearblocks.io/txns/HtgVeqVoDEzWLmgVMibpyc188CbD4BB9FVanucWvctSB) |
| 4 | `nft_revoke_all` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [Er71vuYT…](https://testnet.nearblocks.io/txns/Er71vuYTFECWKEQs5U1hy16bcG4eaaQnvao7ctDDZA6i) |
| 5 | `nft_transfer` → `v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [5S2yQjHE…](https://testnet.nearblocks.io/txns/5S2yQjHE1tCvqTAbyNhFJAXZs9HDMNExVAf7Fvre4kAC) |
| 6 | `nft_transfer` → `v3.onhire.testnet` | `v3.onhire.testnet` | refused (expected) | [844Hi5pi…](https://testnet.nearblocks.io/txns/844Hi5pihgGL88vbcBV61yVrPmNDmaFeTW1u13Menj2) |
| 8 | `list` → `box-0004.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [2gi8737q…](https://testnet.nearblocks.io/txns/2gi8737qRSHK2W44i5FGe9j5RaZdER7tu5TMqdGGoaY9) |
| 9 | `nft_transfer` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [ARYmu45u…](https://testnet.nearblocks.io/txns/ARYmu45uAd1aNbZ1wotJRgDZmzSRMAEWtY6n5h6QPtJv) |
| 10 | `nft_transfer_call` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [D4fSGz9F…](https://testnet.nearblocks.io/txns/D4fSGz9FMnjViRJTJZo3GvRRssSEm8VmF2ULXej797MX) |
| 11 | `nft_approve` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [2bi7CF2G…](https://testnet.nearblocks.io/txns/2bi7CF2G2Ss2L6JTg9J6REFpj4W4GffGMjJ4bm4GueoH) |
| 12 | `nft_revoke` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [6uxf3g2n…](https://testnet.nearblocks.io/txns/6uxf3g2npDKyyNFLSBJQSZaFxtEhNBRnxarHyCd1g8Tp) |
| 12 | `nft_revoke_all` → `v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [6tu9baZi…](https://testnet.nearblocks.io/txns/6tu9baZi1RHBsh5DTMkGSshg2mcwUz1hDrg1zrtpLQxy) |
| 13 | `nft_transfer` → `v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [HdZg1NCv…](https://testnet.nearblocks.io/txns/HdZg1NCvwoMvuWQmYmMN52d3j5fuNGvjBX9o1uL9pjXq) |
| 14 | `nft_transfer` → `v3.onhire.testnet` | `v3.onhire.testnet` | refused (expected) | [AyNmAJ1U…](https://testnet.nearblocks.io/txns/AyNmAJ1UYnqdbHDdvtK74dk5CVF7zxnxqmeEM4xD4M8X) |

#### testnet-t4 (pid 873685)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [BQwTRtxW…](https://testnet.nearblocks.io/txns/BQwTRtxWSz85sEfdNY4cjjf6LvEbF7ss5uA6QqvCa3rb) |
| 1 | `attest_inspection` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | rejected (not executed) | [HNG2Mzer…](https://testnet.nearblocks.io/txns/HNG2MzerPHyfYK7SAp549N3zxb1VZsJkMhCm7VHKDkJd) |
| 1 | `attest_inspection` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [6n4ZAwLa…](https://testnet.nearblocks.io/txns/6n4ZAwLafNZWYgBCVi8NWhuJUtBCeDDN4XK571hgMDPr) |
| 2 | `start_lease` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [2z3RRia3…](https://testnet.nearblocks.io/txns/2z3RRia35htz6dhzbi2sr31GRUPx5yCUJvoXsn9hXpvY) |
| 3 | `accept_lease` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [2bzastXH…](https://testnet.nearblocks.io/txns/2bzastXH5p2e8bxNGdgdGdo4L8fDpwbJCgCHmUu5sbu1) |
| 4 | `pay_hire` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [71sgxCAk…](https://testnet.nearblocks.io/txns/71sgxCAkyvv8LRPyqGUMWQHBphyhTqB22dVwatu9kQqN) |
| 5 | `CreateAccount` → `t4b1794603511.onhire-outsider-n13.testnet` | `onhire-outsider-n13.testnet` | ok | [2tYD8do9…](https://testnet.nearblocks.io/txns/2tYD8do9Kj8ErXRUrNPLcmH4KtXyqsiwUfNzW6q1fFJN) |
| 5 | `list` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [C2Bz74q4…](https://testnet.nearblocks.io/txns/C2Bz74q4oZnQ2aoJMrPBD911a7WUTRC8cKXsBDErVdvN) |
| 6 | `Transfer` → `onhire-racer1-n13.testnet` | `v3.onhire.testnet` | ok | [ycFw9pgg…](https://testnet.nearblocks.io/txns/ycFw9pggc25oa1Rbwx6DNNFuxfGWwuGxAzQkLcZbUzy) |
| null | `Transfer` → `onhire-racer1-n13.testnet` | `v3.onhire.testnet` | rejected (not executed) | [4GnEJYPU…](https://testnet.nearblocks.io/txns/4GnEJYPUTXdw1896FWXD741XCBVa22ZLDm9AzKmxxPFt) |
| 6 | `Transfer` → `onhire-racer1-n13.testnet` | `v3.onhire.testnet` | ok | [6Q1ZWArN…](https://testnet.nearblocks.io/txns/6Q1ZWArN7qXSzyxj298XoYY9mSU32unJ21k8qsMPEKJv) |
| 6 | `buy` → `box-0005.v3.onhire.testnet` | `t4b1794603511.onhire-outsider-n13.testnet` | ok | [BpnwcM5U…](https://testnet.nearblocks.io/txns/BpnwcM5UFoVF2GQTzHFiPAJ5ms2Qu1R9VgPbQWAtjYJS) |
| 6 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [92gndKZL…](https://testnet.nearblocks.io/txns/92gndKZLfAzvZJecu2h6gLkSk5WEJ2Aje5XwhUh7oHbu) |
| 6 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [BtZR2F2b…](https://testnet.nearblocks.io/txns/BtZR2F2b6YGPt5zCn5JxThHiRHW6n4j63ai2WXnKKPNA) |
| 6 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [iRfdLTCL…](https://testnet.nearblocks.io/txns/iRfdLTCLf966yW2UMpLtHGmcUarp4dGRNGmjA3UCxTX) |
| 6 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [8n2HqEqN…](https://testnet.nearblocks.io/txns/8n2HqEqNRDWUt2s8NtSs1DryyZLP47fsuStxYCjjnAkJ) |
| 6 | `Transfer` → `v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [4EAEkJrs…](https://testnet.nearblocks.io/txns/4EAEkJrscFHUJEEg4mHc2BAGVUeqbsJi7XXAtDCQpWGF) |
| 7 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [HzYAjrPt…](https://testnet.nearblocks.io/txns/HzYAjrPti3ir1ggMc1w3fWRj5kFijido6pxnaNTqASPm) |
| 7 | `on_nft_transfer` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [3ycdkwXQ…](https://testnet.nearblocks.io/txns/3ycdkwXQrG3eJDAMLFvsYt9Cc3iRwv1xuoBQWTER2sHx) |
| 8 | `withdraw` → `box-0005.v3.onhire.testnet` | `t4b1794603511.onhire-outsider-n13.testnet` | ok | [7mSKwN42…](https://testnet.nearblocks.io/txns/7mSKwN425W3egmnaEQsX7G5DpTDpJKFw9JL35aUYkdkq) |
| 9 | `list` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [9AsZvQuL…](https://testnet.nearblocks.io/txns/9AsZvQuLGrgwVoE1CAbpsbdsT7RDAZYPHVmZUUhR3Fwm) |
| 10 | `buy` → `box-0005.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [rVGfQyaE…](https://testnet.nearblocks.io/txns/rVGfQyaE2Fr7mmGTCxzs9fZ25f5FCZxg3fG7n9dDt4n) |
| 10 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | ok | [BXdjD26j…](https://testnet.nearblocks.io/txns/BXdjD26jrSq6hAtbmouL1WrR2Zhu6GimVorWhiCxabKd) |
| 10 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [twA583jJ…](https://testnet.nearblocks.io/txns/twA583jJdZ2XGvwdMMYn9xFWNWEPZDBDgj47EeM7zhB) |
| 10 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [4BmA6qBo…](https://testnet.nearblocks.io/txns/4BmA6qBoDoRbruuexEcauBnybMFUANgcQShaGUcLEQz4) |
| 10 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [FsFvPfjd…](https://testnet.nearblocks.io/txns/FsFvPfjdAXqV9WpLQCaecMdgp9BYujKK1DF6avqAtRXm) |
| 11 | `list` → `box-0005.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [EpKacLTw…](https://testnet.nearblocks.io/txns/EpKacLTwvTrQuqJjMZ7K8xwnihBZ5MtQWBcRwb9A9ZLa) |
| 12 | `buy` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [BpZBskvQ…](https://testnet.nearblocks.io/txns/BpZBskvQKdbSus9DzfxsYGGppoQTkV54EQ6Puv9FKcLi) |
| 12 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | ok | [9V8PnT2D…](https://testnet.nearblocks.io/txns/9V8PnT2DBsr2i3644TkK77fVtB45WHkuiYkFviffNEdM) |
| 12 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [ACgu6nUB…](https://testnet.nearblocks.io/txns/ACgu6nUBMqCoXUYirAXzpq4NEBfijer8YKBQ7DgtNDEa) |
| 12 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [5ZwKAicS…](https://testnet.nearblocks.io/txns/5ZwKAicSn5qULB9AFNftwRS1CHoqNGcZrRZa6q1Cs4M6) |
| 12 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [CPLHDzgv…](https://testnet.nearblocks.io/txns/CPLHDzgvHwUiycfrHpaNjTukt4QDqGd4BUJGr5ywpudQ) |
| 13 | `list` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [EcUsR3ip…](https://testnet.nearblocks.io/txns/EcUsR3ipvXTLiepw4MeUj1aKWvUT36mmKyqD5DBHZgAY) |
| 14 | `buy` → `box-0005.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [FHHEWv5R…](https://testnet.nearblocks.io/txns/FHHEWv5RAHdFvkoswbBnTP8tRrcEEXkG64EjqnVfrrXQ) |
| 14 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | ok | [GdPxPAzA…](https://testnet.nearblocks.io/txns/GdPxPAzAgbqxqReJrWEYWvhhssdod2hmVZsWM3jjZ3FD) |
| 14 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [64z5iwdF…](https://testnet.nearblocks.io/txns/64z5iwdFBMfGBXwxL7uLiBoDTV3PQiXzWxNH4wRk6EqJ) |
| 14 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | refused (expected) | [Emao3Y4d…](https://testnet.nearblocks.io/txns/Emao3Y4drMSbHAqNVFfWwtj48ZARGAtf8kWxdqpSxfUF) |
| 14 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [2cNErYvy…](https://testnet.nearblocks.io/txns/2cNErYvyGrEWdWnQKGTKd6gnLsAvch1JYZ96ojnD16hX) |
| 15 | `list` → `box-0005.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [BXtKj2iW…](https://testnet.nearblocks.io/txns/BXtKj2iW3QScurTmKiDUeNwAd2KyCatNfU7kYLh49956) |
| 16 | `buy` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [CEG5KDxw…](https://testnet.nearblocks.io/txns/CEG5KDxw8NDWrPJ6Y5bTYoB5KyveUJWHhXoBB2cyCxD8) |
| 16 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | ok | [5fWqcWQo…](https://testnet.nearblocks.io/txns/5fWqcWQorRBCSv4k4pGxD2EDuBYe5UAJDfbodLBqFtLt) |
| 16 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-racer1-n13.testnet` | ok | [9Q6z3ZhP…](https://testnet.nearblocks.io/txns/9Q6z3ZhPNz1qo1REBJjFNgZXLswSXm2hVg4sVyKzpsCa) |
| 16 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [5XM2CPVr…](https://testnet.nearblocks.io/txns/5XM2CPVr9PanDWaXuNt1YQRDBCaPk1LFriH2yUbEoXJ) |
| 16 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [GYG49Hi2…](https://testnet.nearblocks.io/txns/GYG49Hi2hAe5xrXYfa3AyFzhibCcijCfoVcEPUZP61Zc) |
| 17 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [45TLPMZt…](https://testnet.nearblocks.io/txns/45TLPMZtxBmDEdMvkVQTaNxmZxZW29gTZMAtcqwC1Nwu) |
| 17 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [5RMjwa7X…](https://testnet.nearblocks.io/txns/5RMjwa7XaGFgt1DmqfQBRyyLd6WsyaBtxtZowkFfbGeg) |
| 17 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [FboD4wed…](https://testnet.nearblocks.io/txns/FboD4wed4Aj5VfZhuKLGvR7gC3LXmPBkcB2kARU5UQF8) |
| 17 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [BCVfFpPj…](https://testnet.nearblocks.io/txns/BCVfFpPjMUa6vJtx7eez12AJ9Vqm1MPgrsqEkGQr5DYv) |
| 17 | `rescue_sale` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [7RSrvkGm…](https://testnet.nearblocks.io/txns/7RSrvkGm8oxqmLMAaRL2pXgS6Zp27mQERMk5LSCYeXhH) |
| 17 | `on_nft_transfer` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [BpNkyLvx…](https://testnet.nearblocks.io/txns/BpNkyLvxubuvEjoQnCq5gq6ut3QSSVrFUxUXcJk27fG7) |
| 17 | `on_rescue_owner` → `box-0005.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [89NK2vBE…](https://testnet.nearblocks.io/txns/89NK2vBE8s4XLgyaWwbxieNaqVzzXzvbPm1Mdg82vygU) |
| 18 | `pay_hire` → `box-0005.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [9HvdRKX8…](https://testnet.nearblocks.io/txns/9HvdRKX8y6Qx4apqkAXV9MWqsntkhK39RLMufZvbnEuy) |
| 19 | `withdraw` → `box-0005.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [8LwUo8ay…](https://testnet.nearblocks.io/txns/8LwUo8ayZ4RXZ4T6ZvY5PgTAcNZo1Dcek226pzSn1fdv) |
| 20 | `withdraw` → `box-0005.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [AAV2qRJA…](https://testnet.nearblocks.io/txns/AAV2qRJA7NuNGYZAwD2knhjFqpDtJHhbwWs78vhRgAX7) |

#### testnet-t5 (pid 895521)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [6PfJbYEk…](https://testnet.nearblocks.io/txns/6PfJbYEkn4EUgfCjxZXP2iHmetSyNTNU3UD5fNtygFxg) |
| 1 | `attest_inspection` → `box-0012.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [3pZp85aC…](https://testnet.nearblocks.io/txns/3pZp85aCUwKLN1TG7WeTbiYFCWwg97twHV5XLYqeYgx7) |
| 2 | `start_lease` → `box-0012.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [7VVgAJFa…](https://testnet.nearblocks.io/txns/7VVgAJFaRD9YZAAc8MJQr9A9at66paHGhH1vGnt5GhpD) |
| 3 | `accept_lease` → `box-0012.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [EUigqXxs…](https://testnet.nearblocks.io/txns/EUigqXxse25v9aUixGeeybQhH7mkb6bhQXazpZyfrWxd) |
| 3 | `accept_lease` → `box-0012.v3.onhire.testnet` | `onhire-outsider-n13.testnet` | refused (expected) | [9pvJ7mKv…](https://testnet.nearblocks.io/txns/9pvJ7mKvhAGhoqeSSpQYrSf286VLPPgUpZcipHj7Rfow) |
| 3 | `accept_lease` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [8kZwcNBr…](https://testnet.nearblocks.io/txns/8kZwcNBr1DUC4noDLzKfSupqZA7ePKX19ae1i29QsHJc) |
| 3 | `accept_lease` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [3h5Hw4zJ…](https://testnet.nearblocks.io/txns/3h5Hw4zJ9BTP1WPuDHMD1dynK4N4ijPqp6yRF6B7tGbw) |
| 4 | `accept_lease` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [GyukYEe9…](https://testnet.nearblocks.io/txns/GyukYEe99mvkmHjYy4snZTa4gGqU1CLeRDATPAaBK5Ya) |
| 6 | `pay_hire` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [9z3VWGtU…](https://testnet.nearblocks.io/txns/9z3VWGtULRBjYr7uyksTwGudK8ifsjx9yGP4uLLbaxL6) |
| 7 | `off_hire` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [7kFE4Syf…](https://testnet.nearblocks.io/txns/7kFE4SyfJZZGr55ES6D8RVAaMcwpB7o3EgZekHhUg7Tp) |
| 8 | `withdraw` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [4vKU2bQC…](https://testnet.nearblocks.io/txns/4vKU2bQC3LJB414wtiEDFtkEALwuGequRx94Vf7iQrNy) |
| 9 | `start_lease` → `box-0012.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [3efQyVGd…](https://testnet.nearblocks.io/txns/3efQyVGd8YygjTrpoCqKwznGV1FaqJfEo9LgJNmAvv6e) |
| 10 | `start_lease` → `box-0012.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [8UENtcu9…](https://testnet.nearblocks.io/txns/8UENtcu9GXRj1K5C452VQo7zUHLT7YLsD8NZow1DmWzC) |
| 11 | `attest_inspection` → `box-0012.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [EE2t9jp2…](https://testnet.nearblocks.io/txns/EE2t9jp2JFVKVRU7jtc8FShdbL2DMcw3kK7BCvuGRAC1) |
| 11 | `attest_inspection` → `box-0012.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [5YUVWdeB…](https://testnet.nearblocks.io/txns/5YUVWdeBp39mYraFRTpQcmkZ3GckvGxBbQV5ygqv7Vsz) |
| 11 | `attest_inspection` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [DV7dAPBr…](https://testnet.nearblocks.io/txns/DV7dAPBrvyB16Q5Gb4jpE46DFgFEWod3TBErFT8wULbC) |
| 12 | `attest_inspection` → `box-0012.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [FLgasxCf…](https://testnet.nearblocks.io/txns/FLgasxCfkjnuGpgwWcEhcQFcarc3fLw98N4kQU3pK8KP) |
| 13 | `start_lease` → `box-0012.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [9z7ZUUah…](https://testnet.nearblocks.io/txns/9z7ZUUahvLkHmdEWVTEvwu1qRUfBWJrZrTACMB8DHnYT) |
| 14 | `accept_lease` → `box-0012.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [4KwbyPTe…](https://testnet.nearblocks.io/txns/4KwbyPTejoockfYywPsMfzNTJr2UYXn1wNqikayZiBqz) |

#### testnet-t6 (pid 882624)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [6jooC81F…](https://testnet.nearblocks.io/txns/6jooC81FX1erRRzB83vnqQFNdXFp11TgAH6mQLYFj3gN) |
| 1 | `attest_inspection` → `box-0007.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [ArvPvA5A…](https://testnet.nearblocks.io/txns/ArvPvA5ANRPVfc5T3LCwyMw9qoiRuZpgwbUhNxUXA1os) |
| 2 | `start_lease` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [3uGijodZ…](https://testnet.nearblocks.io/txns/3uGijodZkxHNRCJTFfWW1PkaVLuGjN2fRkjSxTacuhwt) |
| 3 | `accept_lease` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [3ufqgrFz…](https://testnet.nearblocks.io/txns/3ufqgrFz3JeaTfogmfDKRc4dNgkGhJiishfautCPG2Wu) |
| 4 | `pay_hire` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [6PMtHB8M…](https://testnet.nearblocks.io/txns/6PMtHB8MrC823Q7vzFo39Ksp8fBJJwrutKyjgxnL5398) |
| 5 | `withdraw_reserve` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [Gs5rHmoH…](https://testnet.nearblocks.io/txns/Gs5rHmoHcXXXkUKJBw9qb5tveh1NXZwNsVzoAmm27Hz) |
| 5 | `withdraw_reserve` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [2YffP3ny…](https://testnet.nearblocks.io/txns/2YffP3nycWpGem77mH7MmT3TTbRaPSQGVTZ7Kzr13GqF) |
| 5 | `withdraw_reserve` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [HVGjXkKC…](https://testnet.nearblocks.io/txns/HVGjXkKCG4ELLt2hHxsMSEqW2htfBkKLq2KcQRyycyTc) |
| 5 | `reserve_withdraw` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [2d7PhRUo…](https://testnet.nearblocks.io/txns/2d7PhRUosxK2gUDjva6Rqq96fYT2u1QZYHyhTrnu2y6A) |
| 5 | `reserve_withdraw` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [Hb4AavY4…](https://testnet.nearblocks.io/txns/Hb4AavY42JESi4n8JowPSnRoFBqKSPg5eTVQd2yughpD) |
| 5 | `reserve_withdraw` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [GaYjgXKY…](https://testnet.nearblocks.io/txns/GaYjgXKYqZH2xZrfk7P9V8Qz3A73foiR5R65h2yFDmjU) |
| 5 | `claim_reserve` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [DRFzspLX…](https://testnet.nearblocks.io/txns/DRFzspLXtXC4eLzHoqJqgDxU2TziokafHZ4vGE8nGQ3e) |
| 5 | `claim_reserve` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [9iHmL2QQ…](https://testnet.nearblocks.io/txns/9iHmL2QQPzAdQYf6hj9ndjQtpUirXvz8AA8kQQYaGrbM) |
| 5 | `claim_reserve` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [DaVHcXSk…](https://testnet.nearblocks.io/txns/DaVHcXSk87NDA6aGLLwbhCKw2n7kV24SaiJ65rTLmDqe) |
| 5 | `add_key` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [5cifPrsz…](https://testnet.nearblocks.io/txns/5cifPrszvrzHHmTAvn4w7FnVERnZBni664rutbwzxirB) |
| 5 | `add_key` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [6p7a67vF…](https://testnet.nearblocks.io/txns/6p7a67vFFUtJ3GE5CUATZodEaaSHTczKqhWimYQ4LGro) |
| 5 | `add_key` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [BqRBhf4z…](https://testnet.nearblocks.io/txns/BqRBhf4zi4rBHXxgtjdtVG3Lqyscc6QCympWZojvJ8wY) |
| 5 | `add_full_access_key` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [EwBgtxE3…](https://testnet.nearblocks.io/txns/EwBgtxE3E2oPCimtcRF1TRkrWhJqsJWfQ38EFT3nAxsi) |
| 5 | `add_full_access_key` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [Ayfztamq…](https://testnet.nearblocks.io/txns/AyfztamqzCUhZG3hHDBbNGpSB96imfvKzmPK1Hv2wP86) |
| 5 | `add_full_access_key` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [7f7KahUb…](https://testnet.nearblocks.io/txns/7f7KahUbQcsPZbTA6uSm8QEP1DZ6Cz3nAN9BY33w9HdA) |
| 5 | `delete_account` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [AFE5Ad4G…](https://testnet.nearblocks.io/txns/AFE5Ad4GtAr8opiJGDT2xr5hJmBsmfhM6c2sp63tMebo) |
| 5 | `delete_account` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [35saTKWW…](https://testnet.nearblocks.io/txns/35saTKWWToPb85qCjjZG7u71sCrLuwoUUDUxRMJEPNDh) |
| 5 | `delete_account` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [HcjoHaJF…](https://testnet.nearblocks.io/txns/HcjoHaJFJJUhagpkjLNPzDCnTMvKkZ2xBC3Px9ZLM2kr) |
| 5 | `update` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [4NYwi4wv…](https://testnet.nearblocks.io/txns/4NYwi4wvE8zggrG5ZJyYrJwXiVvhejbuwjgctB7UXJ3Z) |
| 5 | `update` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [GWo9tfWE…](https://testnet.nearblocks.io/txns/GWo9tfWE6znG6L9JQT1wYi5fphJayqi2uit4fuuqgVBK) |
| 5 | `update` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [3XrLHbNA…](https://testnet.nearblocks.io/txns/3XrLHbNAvovtcALDXmzLccYreTMvZ2mWGHHiS7r8gtPM) |
| 5 | `upgrade` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [5Q1M4szt…](https://testnet.nearblocks.io/txns/5Q1M4szt2LkACWgzsZjARj7RavEAMoBvsRxJjYAsrwiu) |
| 5 | `upgrade` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [7KdsqwPE…](https://testnet.nearblocks.io/txns/7KdsqwPEQLtqfGTvsEK2VGp4PN5VTkpeV9yBbf14Fgg6) |
| 5 | `upgrade` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [3pWtZZ3y…](https://testnet.nearblocks.io/txns/3pWtZZ3yprGod2RqbgcAgK5qfguWML3JiABmEYvtc4Qu) |
| 5 | `deploy` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [EZG4MXhY…](https://testnet.nearblocks.io/txns/EZG4MXhYToyNenpm5dxwcqd4YVpXZht22QXr6w73Eogf) |
| 5 | `deploy` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [4yVYPbht…](https://testnet.nearblocks.io/txns/4yVYPbhtzr7DrhzMKpbvnFkt9CRYnCnDPtMrBDVxYCYK) |
| 5 | `deploy` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [CARdM7zR…](https://testnet.nearblocks.io/txns/CARdM7zRu9RNRqMdgT6G7r381vDM7fY2KvktLvKmaVpp) |
| 5 | `migrate` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [GNni9K8G…](https://testnet.nearblocks.io/txns/GNni9K8GtDfdfrUgw2XL9Kgo2vxWH3S95LRHtkHZf53M) |
| 5 | `migrate` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [G44awvXQ…](https://testnet.nearblocks.io/txns/G44awvXQ33TvdGXii48JUCHpS4UpUCpHtLDY7VKNQByz) |
| 5 | `migrate` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [8XoSYv2o…](https://testnet.nearblocks.io/txns/8XoSYv2otqzyNqWwtChHsyDNpY4qHwezZAEtkN8M56tw) |
| 6 | `withdraw` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [AXmbNRuU…](https://testnet.nearblocks.io/txns/AXmbNRuUG4mEsBjrZ968hae6uiAVkvB6UXodzqLiw4e5) |
| 7 | `withdraw` → `box-0007.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [21JqxBoV…](https://testnet.nearblocks.io/txns/21JqxBoVgorvBddDzTZdoyRBZfofVWr5TQp4MGbJ9PjN) |
| 7 | `withdraw` → `box-0007.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [2vf9aioS…](https://testnet.nearblocks.io/txns/2vf9aioSgtJkHd4WVFz2ySXJvkS9qGvdQFkuv9AwGrjM) |
| 7 | `withdraw` → `box-0007.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [2poAjoWT…](https://testnet.nearblocks.io/txns/2poAjoWTh9PvcCeaxht7JggDdmwLWPYWzCJ7iAzY7XWj) |
| 7 | `withdraw` → `box-0007.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | refused (expected) | [DhWLaiyk…](https://testnet.nearblocks.io/txns/DhWLaiykK8bCMfrVLq9GBn1dYeUG3N9REjDVtjiCnqZA) |

#### testnet-t7 (pid 884475)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [29TZtj89…](https://testnet.nearblocks.io/txns/29TZtj89E1wSEJXQqWLqzkLruYFAH7N3UPJNztJQbNav) |
| 1 | `attest_inspection` → `box-0008.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [2KnBVVVU…](https://testnet.nearblocks.io/txns/2KnBVVVUzf4L5hcrUNnp858Vt6jXWY3ZoKP4JLFWY6AD) |
| 2 | `start_lease` → `box-0008.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [58nEHVfY…](https://testnet.nearblocks.io/txns/58nEHVfYrdTkeQ1x5yniSTFs6xmMXfiagu8gB5G1GLqc) |
| 3 | `accept_lease` → `box-0008.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [FJ68gXLh…](https://testnet.nearblocks.io/txns/FJ68gXLh7N6aYceGDEXfnih2ofgVMKqjy6BKZL53W735) |
| 5 | `pay_hire` → `box-0008.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [3EPGFHx4…](https://testnet.nearblocks.io/txns/3EPGFHx4EMLc4FBg2xeWvcHpmLdu3QraAXYCgFRSqKHp) |
| 6 | `list` → `box-0008.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [8cAc69VR…](https://testnet.nearblocks.io/txns/8cAc69VRnEyRtz2crt79mVEdpkZ62CodauGZiFWYPHsc) |
| 7 | `buy` → `box-0008.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | ok | [CsKkfgd2…](https://testnet.nearblocks.io/txns/CsKkfgd2caANfbLy6AEEQs7Tgt1QZtpUCuzgA928rrMr) |
| 8 | `withdraw` → `box-0008.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [AtJU5Yk3…](https://testnet.nearblocks.io/txns/AtJU5Yk3dzp6US9M6q9CatHzp44BmP4YFBQvaSsh7C1p) |

Failed t5 attempts (see above):

#### testnet-t5 (pid 880482)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [5HbHm9PS…](https://testnet.nearblocks.io/txns/5HbHm9PSZtcoisU14K2c4eH5CAaCnVAgMDFRwDu4UJjF) |
| 1 | `attest_inspection` → `box-0006.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | rejected (not executed) | [AaP7PNcy…](https://testnet.nearblocks.io/txns/AaP7PNcynJ8vuTLj7VkHhwNaTeuqotj9UuftwWQfNWuH) |
| 1 | `attest_inspection` → `box-0006.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | refused (expected) | [Eqpd3APS…](https://testnet.nearblocks.io/txns/Eqpd3APSTEiQVq5QxNsQABC6RKzryXnsvRVEJ6exsvF1) |

#### testnet-t5 (pid 885685)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [7FPB4AjR…](https://testnet.nearblocks.io/txns/7FPB4AjRjgeFadxra96nAX5Qdzoys8seyU5XRff6b9Um) |
| 1 | `attest_inspection` → `box-0009.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | rejected (not executed) | [HqQGHUgw…](https://testnet.nearblocks.io/txns/HqQGHUgwqdV8sUFJL9mfC3ss74eFewMBgXsh2aGPytG2) |
| 1 | `attest_inspection` → `box-0009.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | refused (expected) | [3ECP8Ueo…](https://testnet.nearblocks.io/txns/3ECP8Ueo6ZD6SgH4A1w74EgrWBePqsjLDP4m29bEKkY6) |

#### testnet-t5 (pid 888814)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [HL122NVG…](https://testnet.nearblocks.io/txns/HL122NVGq5uvnKoDjuXTtbEdbBwKH4ZqTPNuXeTnCCsH) |
| 1 | `attest_inspection` → `box-0010.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [7ULSaFEE…](https://testnet.nearblocks.io/txns/7ULSaFEEfMQ75gz73MwKqbDEmyFgqAQkNkdfKVvgroNN) |
| 2 | `start_lease` → `box-0010.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [F3b9BaRT…](https://testnet.nearblocks.io/txns/F3b9BaRTdSDkBrsBdqkch1J5Pbvt7S6W9HPLqDXDk7B8) |
| 3 | `accept_lease` → `box-0010.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [5ncremdb…](https://testnet.nearblocks.io/txns/5ncremdb34Gibep25Z7hLDUFBvaaDo2EKuFGLxVd9m3c) |

#### testnet-t5 (pid 889249)

| after step | call | signer | result | tx |
| --- | --- | --- | --- | --- |
| 0 | `mint` → `v3.onhire.testnet` | `v3.onhire.testnet` | ok | [2uSyqyKv…](https://testnet.nearblocks.io/txns/2uSyqyKvjfAjfz8gWYqwu3XDaC9GgeeNYPKsq6HzJ4nB) |
| 1 | `attest_inspection` → `box-0011.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [APWFwWCy…](https://testnet.nearblocks.io/txns/APWFwWCy4Uco89wqrEbgvsiaX9MmBS6BYRgsA8atqek4) |
| 2 | `start_lease` → `box-0011.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [5FWU8KKH…](https://testnet.nearblocks.io/txns/5FWU8KKHq4zhBSWFkaYobwF63NZrkU8wQMtCqomCchbL) |
| 3 | `accept_lease` → `box-0011.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [5Ng4mqui…](https://testnet.nearblocks.io/txns/5Ng4mquiW7cF49ZDJULZ6xFZTJbcCaM448jZbrwvUzZx) |
| 5 | `pay_hire` → `box-0011.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [D2WseDUL…](https://testnet.nearblocks.io/txns/D2WseDUL3wX2r8ndQasCsErwGHbF4o6nsHuVLWFxgTgP) |
| 6 | `off_hire` → `box-0011.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [77xFKijd…](https://testnet.nearblocks.io/txns/77xFKijdY9CWYvrWy9WKnJa8Ecm4MecjNKb5ns38jg6i) |
| 7 | `withdraw` → `box-0011.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [C21YchoB…](https://testnet.nearblocks.io/txns/C21YchoBR8SVyS4FnRXdxPyJwL3w5ivVTd5ZX8Y4kbzb) |
| 8 | `start_lease` → `box-0011.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [BDsTjbBB…](https://testnet.nearblocks.io/txns/BDsTjbBBDoaCwErzvG1F6foebLbpze7e55SQmkyy7Sbj) |
| 9 | `start_lease` → `box-0011.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [AJQ5ci1t…](https://testnet.nearblocks.io/txns/AJQ5ci1thNC48eqTXHUNU54iPtwmfGnqT79SmoB7uy4n) |
| 10 | `attest_inspection` → `box-0011.v3.onhire.testnet` | `onhire-owner-n13.testnet` | refused (expected) | [8iiTxuZx…](https://testnet.nearblocks.io/txns/8iiTxuZxUy7Vmhdp5TBZ7FBToF58a61Y9qeB5hWDKoTJ) |
| 10 | `attest_inspection` → `box-0011.v3.onhire.testnet` | `onhire-buyer-n13.testnet` | refused (expected) | [Arb8vSSw…](https://testnet.nearblocks.io/txns/Arb8vSSwWBzmmumbRh45y3Dz89R4LHVoM8ioprdSwkHE) |
| 10 | `attest_inspection` → `box-0011.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | refused (expected) | [jvHiLVPp…](https://testnet.nearblocks.io/txns/jvHiLVPpHhoReesSNkdM3tS16QDChEak1fthdDWCkxT) |
| 11 | `attest_inspection` → `box-0011.v3.onhire.testnet` | `onhire-inspector-n13.testnet` | ok | [GHAQ35AJ…](https://testnet.nearblocks.io/txns/GHAQ35AJ5m1Fam1zGHiTS6nzuRqZ7gv2Vt2X2TVmJ1f4) |
| 12 | `start_lease` → `box-0011.v3.onhire.testnet` | `onhire-owner-n13.testnet` | ok | [ESdpNvQ9…](https://testnet.nearblocks.io/txns/ESdpNvQ9UeFnB9UaSo8Bj3ZtvKgn3nj1kuSMey6mNKnZ) |
| 13 | `accept_lease` → `box-0011.v3.onhire.testnet` | `onhire-lessee-n13.testnet` | ok | [4VJdskUc…](https://testnet.nearblocks.io/txns/4VJdskUcb51ow1AVi6QqEtWPKUvqMQAgPQYRxvLZYPYq) |

---

## Historical: tests 1–6 on `onhire.testnet`, before the protocol fee (2026-11-12)


Commit IDs in this document refer to earlier development history, which is kept private. Spec `docs/ONHIRE-SPEC-v1.1.md`. Run 2026-11-12
(UTC) on **NEAR testnet only** (chain id `testnet`, protocol 88, nearcore
2.14.1). Native test NEAR only, from the public faucet. No mainnet, no real
funds, no USDC.

This run predates the 1% protocol fee. There is no fee in this proof. The fee was added later and is proven in `docs/FEE-PROOF.md`.

### Result

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

### Commands

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

### Accounts (all throwaway, testnet)

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

### Deploy

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

#### Wasm hashes (sha256)

| File | sha256 | On chain |
| --- | --- | --- |
| `onhire_item.wasm` (176,231 B) | `0826451ccbd0d4512f4d02f04a675c0a89bbf39258da66ae5589a69c07987f79` | Global code hash: `get_item_code_info` → `CCZFHMvQ…` (base64) and box `global_contract_hash` `Yp7FS19k…` (base58). Both decode to this hash. |
| `onhire_collection.wasm` (221,209 B) | `3de6249e3e1f14877a9b89b2656f3e26281c8bd4dc19d3be829ea5c5743e6c88` | `onhire.testnet` `code_hash` `5AdQjfXh…` (base58) decodes to this hash. |

These are byte-identical to the wasm in `docs/SANDBOX-PROOF.md`. The
contract source was unchanged for this run.

### How each test runs on testnet

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

#### Test 4 on testnet: what it does and does not prove

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

### `SALE_UNDO_DELAY_BLOCKS`: re-measured on testnet

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

### What went wrong along the way

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

### Appendix: every recorded transaction of run B

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

