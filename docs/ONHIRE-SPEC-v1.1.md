# Onhire — Technical Specification v1.1

**Status:** v1.1, frozen 2026-11-11, with the 1% protocol fee added 2026-11-12 and the two-step lease (owner offers, named lessee accepts and posts the deposit) added 2026-11-13. The two-step lease replaces the earlier owner-funded deposit, which let the named lessee end the lease at once and withdraw the owner's NEAR.

**Author / copyright:** see `LICENSE`. License: proprietary, all rights reserved, with a limited NEAR HACKATHON evaluation grant.

**Event:** NEAR HACKATHON, second independent entry. Deadline Saturday, Nov 14, 2026, 6:59 PM ET (`2026-11-14T23:59:00Z`). Primary track: `nfts` (NFTs That Do Something). Do not add `realworld` unless this spec's must-ship tests are green and the README and demo genuinely centre a lessor's workflow. Do not add other tracks.

**Sister project (Q-015):** [openLID](https://github.com/tnvolman/openlid) is an independent project entered in the same hackathon. Onhire must be independent — not a wrapper, fork, or reskin of openLID. Generic NEAR patterns may overlap; the product, contract, and repository must not. Each README must name the other project. Both submissions use the same contact email.

**Network:** NEAR testnet only. Currency: native NEAR only. No USDC, not even test NEP-141. Unaudited. Not for real funds. $0 spend.

---

## 1. What this is

Onhire is a testnet prototype of container leasing. Each shipping container is an NFT that is also its own NEAR account. A lessee makes lease payments into the box. A cut of every lease payment is locked in a repair reserve the owner can never withdraw. The box refuses a new lease when its inspection has lapsed. A separate inspector attests inspections; the owner cannot attest their own box. The owner can list the box for a price, and a buyer can purchase it mid-lease. The lease, the reserve, and future lease payments move with the box. Already-earned lease payments and the sale price are credited to the seller as a pull balance. Each payout goes only to the account it is owed to, and the tests check this.

The demo opens on the mid-lease sale, then a payment that goes to the new owner, a refused lease after inspection lapses, and (stretch) a depot paid from the reserve.

---

## 2. Frozen rules

1. Seller lists a price. Seller may cancel a listing that is not locked. Buyer calls `buy` with that exact amount of native NEAR attached. That call locks the box and starts the collection transfer. A wrong attached amount is rejected and nothing locks.
2. Lease payments and sale payments are native NEAR. No tokens.
3. Accrued lease payments and the sale price are pull balances. Do not push NEAR to the seller in the same step as the NFT transfer. A failed seller `withdraw` cannot unsell the box or move the lease.
4. The box contract is the source of truth for `owner` during normal operation. Do not read the collection from `pay_hire` or other ordinary calls. Only `rescue_sale` (and the post-transfer callback that already expects a result) may query the collection.
5. `rescue_sale` may finalize a sale any time the collection already shows the intended buyer. `rescue_sale` may undo a sale only after the lock is `SALE_UNDO_DELAY_BLOCKS` old, and only if the collection still shows the seller. That delay must be long enough that the original transfer promise has either landed or failed. A sale ID does not cancel an in-flight collection transfer.
6. Each lock has a `sale_id`. A callback or rescue acting on a stale `sale_id` is a no-op and must not change state.
7. Box accounts are created with no access keys, and there is no path to add a key, upgrade the contract, or delete the account. Do not add lessee function-call keys in this version. The lessee calls from their own account. The README must disclose that the collection account on testnet still has the deployer's key.
8. Every collection transfer path is gated to the box account for that token: `nft_transfer`, `nft_transfer_call`, and all approval paths (`nft_approve`, `nft_revoke`, `nft_revoke_all`, and any transfer-by-approved-account). A direct owner or marketplace call must fail. There is no marketplace in this version.
9. A lease payment belongs to whoever the box records as owner at the moment `pay_hire` succeeds. No proration.
10. `pay_hire` during a sale lock is rejected. The lessee retries after the sale finalizes or is undone. After a successful sale, the next payment goes to the buyer. After an undone sale, it goes to the original owner.
11. A lapsed inspection blocks starting a lease only (`start_lease` and `accept_lease`). An active lease continues.
12. The owner can never withdraw the reserve. There is no owner-facing reserve withdrawal method.
13. The owner can never attest their own box. Only the inspector account can attest. The inspector is set at mint or on the collection and is not changeable by the owner.
14. After every test step, this invariant holds: `box_native_balance >= storage_staking + reserve + sum(pending_withdrawals) + held_lessee_deposits + locked_buyer_payments`.

### 2.1 Constants (document in code and README)

| Name | Value | Notes |
| --- | --- | --- |
| `RESERVE_BPS` | `1500` | 15% of each lease payment goes to the reserve |
| `SALE_UNDO_DELAY_BLOCKS` | `32` | Undo is allowed only at `lock.started_at_block + 32` or later. Re-measure on testnet day one; if receipts can land later than this, raise it and update the README. Do not lower it to make a test pass. |
| `MIN_RESERVE` | `0` | `start_lease` and `accept_lease` also fail when `reserve < MIN_RESERVE`. Zero keeps first-lease tests unblocked. |
| `BPS_DENOMINATOR` | `10_000` | |
| `FEE_BPS` | `100` | Added 2026-11-12 (author). 1% protocol fee on each lease payment and each sale. Taken from the owner's/seller's proceeds after the reserve split, never from the reserve: lease payment 100 → reserve 15, fee 1, owner 84; sale 100 → fee 1, seller 99. If the owner's share cannot cover the fee, the whole call reverts. Sent to the box's immutable `fee_account` (a throwaway `.testnet` account on the testnet deploy). Proof: `docs/FEE-PROOF.md`. |

---

## 3. Architecture

### 3.1 Accounts

- Collection / factory: `onhire.testnet` (exact name may be `onhire-<suffix>.testnet` if `onhire.testnet` is taken; record the live name in the README).
- Each container: `box-{id}.onhire.testnet` (or a sub-account of the chosen collection), e.g. `box-0042.onhire.testnet`. `{id}` is a zero-padded decimal token id.
- Inspector: a dedicated testnet account, not the owner, not the lessee, not the buyer. Named in the README as standing in for a certified surveyor or depot inspector.
- People in the demo: owner, lessee, buyer, inspector — four separate testnet accounts.

### 3.2 Contracts

Two contracts only for must-ship:

1. **Collection** — NEP-171 (and metadata standards as needed for a valid NFT: NEP-177 / NEP-181 if straightforward). Also the factory that mints a box account, deploys the item contract, deletes every access key on the new account, and initializes state. Prefer a global contract (NEP-591) so every box runs the same code; if testnet global-contract deploy fails on day one, fall back to a normal deploy paid with free testnet NEAR, and document the fallback.
2. **Item (box)** — holds lease, reserve, inspection, listing, sale lock, and pull balances. Holds the native NEAR.

The collection is the NFT registry. The box is the account that owns the money and enforces lease payment, sale, and inspection rules.

### 3.3 Sale sequence (must implement exactly)

1. Owner calls `list(price_yocto)` on the box. Box is not locked. `price_yocto > 0`.
2. Buyer calls `buy` on the box with attached NEAR exactly equal to `price_yocto`. Box is not locked and is listed. Buyer is not the current owner.
3. `buy` credits `locked_buyer_payments[sale_id] += attached`, increments `sale_id`, writes the lock `{sale_id, seller, buyer, price, started_at_block}`, and issues `nft_transfer` to the collection for this token, predecessor-to-be the box, receiver = buyer, with enough gas for the transfer and the callback.
4. Callback `on_nft_transfer(sale_id, success)`:
   - If `sale_id` is stale or the box is not locked for that id: no-op.
   - If success: **finalize** (section 3.4).
   - If failure: **undo immediately** (section 3.5). A failed transfer does not wait for the undo delay.
5. `rescue_sale(sale_id)` (public):
   - If `sale_id` is stale or not locked: no-op or fail with a clear error; no state change.
   - Query the collection's owner of this token (this is the only intended collection read).
   - If collection owner == lock.buyer: **finalize**.
   - If collection owner == lock.seller AND `current_block >= started_at_block + SALE_UNDO_DELAY_BLOCKS`: **undo**.
   - If collection owner == lock.seller AND the delay has not elapsed: fail, leave the lock in place.
   - If collection owner is anyone else: fail, leave the lock, do not guess.
6. After finalize, seller calls `withdraw` to receive `price + lease payments earned before the sale`. After undo, buyer calls `withdraw` to receive the attached purchase price. Those withdrawals are pull-only.

### 3.4 Finalize

- Set box `owner = lock.buyer`.
- Add `lock.price + accrued_hire` to `pending_withdrawals[lock.seller]`.
- Set `accrued_hire = 0`.
- Clear the listing.
- Move `locked_buyer_payments[sale_id]` out of the locked bucket (it is now part of the seller's pending credit, together with accrued lease payments already on the box).
- Clear the lock.
- Leave lease, reserve, inspection, and lessee deposit unchanged.
- Emit an event.

### 3.5 Undo

- Leave `owner`, lease, reserve, inspection, accrued lease payments, and listing unchanged. After a failed `buy` transfer, the listing remains so the buyer (or another buyer) can retry; after a timed-out rescue undo, same rule.
- Add `locked_buyer_payments[sale_id]` to `pending_withdrawals[lock.buyer]`.
- Clear that locked payment and clear the lock.
- Do not credit the seller.
- Emit an event.

---

## 4. Box state

```
owner: AccountId                  // source of truth except during rescue, which may correct it
accrued_hire: u128                // unpaid lease payments already belonging to current owner
reserve: u128
pending_withdrawals: Map<AccountId, u128>
locked_buyer_payments: Map<u64, u128>   // sale_id -> attached purchase NEAR
listing: Option<{ price: u128 }>
lock: Option<{
  sale_id: u64,
  seller: AccountId,
  buyer: AccountId,
  price: u128,
  started_at_block: u64
}>
next_sale_id: u64
lease: Option<{
  lessee: AccountId,
  rate_per_day: u128,             // yoctoNEAR per day, paid explicitly via pay_hire
  start_block_or_ts: u64,
  end_block_or_ts: u64,
  deposit: u128                   // posted by the lessee in accept_lease
}>
lease_offer: Option<{             // pending, unaccepted offer; holds no NEAR
  lessee: AccountId,
  rate_per_day: u128,
  duration_secs: u64,             // counted from acceptance
  deposit: u128,                  // exact NEAR the lessee must attach to accept_lease
  offered_by: AccountId           // owner who made the offer; offer is stale if the owner changes
}>
inspection_valid_until: u64       // timestamp, seconds
inspector: AccountId              // immutable after init
min_reserve: u128
reserve_bps: u16                  // 1500
```

Storage staking is reserved from the box balance and counts in the invariant. Init must attach enough NEAR for storage plus a small buffer.

---

## 5. Methods

Use clear, typed errors. 1-yocto confirmation on owner-only calls that change sale or lease terms, consistent with NEAR NFT practice, unless a given method cannot accept 1 yocto because it already requires a precise attached amount (`buy`, `pay_hire`, and the lessee's `accept_lease`, which attaches exactly the offered deposit). In those cases, the attached-amount check is the confirmation. `start_lease` and `cancel_lease_offer` are owner calls and take exactly 1 yocto; the owner never attaches a deposit.

### 5.1 Box — owner

- `list(price: U128)` — owner, not locked, `price > 0`. Replaces a previous unlocked listing.
- `cancel_list()` — owner, not locked, listing exists. Clears listing.

### 5.2 Box — buyer

- `buy()` — attached NEAR == listed price, not locked, listing exists, predecessor != owner. Starts section 3.3. Reject otherwise; do not lock; refund attached NEAR by the usual panic/attach revert.

### 5.3 Box — anyone

- `rescue_sale(sale_id: u64)` — section 3.3 step 5.
- `withdraw()` — pays `pending_withdrawals[predecessor]` to predecessor, then zeros that entry. No-op or fail if zero. Cannot be used to withdraw reserve, locked buyer payments, or a lessee deposit.

### 5.4 Box — lessee / owner (lease)

A lease starts in two steps. The owner offers; only the named lessee can accept, and the lessee posts the deposit.

- `start_lease(lessee: AccountId, rate_per_day: U128, duration_secs: u64, deposit: U128)` — owner, exactly 1 yocto attached (any other amount fails), not locked, no active lease, no pending offer (`ERR_LEASE_OFFER_PENDING`), `now <= inspection_valid_until`, `reserve >= min_reserve`, `rate_per_day > 0`, `duration_secs > 0`, `deposit >= 1` (`ERR_ZERO_DEPOSIT`), `lessee != owner`. Writes `lease_offer` (terms, required deposit, `offered_by = owner`). Does not start the clock and holds no NEAR. Event `lease_offered`.
- `accept_lease()` — payable. Not locked; an offer exists (`ERR_NO_LEASE_OFFER`); predecessor == `lease_offer.lessee` (`ERR_NOT_LESSEE`); no active lease; `lease_offer.offered_by == owner`, i.e. the box has not changed hands since the offer (`ERR_STALE_OFFER`); lessee != current owner; inspection still valid and `reserve >= min_reserve`, rechecked now; attached NEAR == `lease_offer.deposit` exactly (`ERR_WRONG_DEPOSIT`). Clears the offer and sets `lease = { lessee, rate_per_day, start = now, end = now + duration_secs, deposit = attached }`, so the clock starts at acceptance. The deposit counts in `held_lessee_deposits` (rule 14). Event `lease_started`. Any refusal panics, so the attached NEAR is refunded.
- `cancel_lease_offer()` — owner, exactly 1 yocto, not locked, an offer is pending (`ERR_NO_LEASE_OFFER`). Clears the offer. Event `lease_offer_cancelled`. After a sale, the new owner can cancel a stale offer left by the seller.
- `pay_hire()` — predecessor == current lessee, a lease is active, not locked. Attached NEAR must be `> 0`. Split: `reserve += attached * reserve_bps / 10_000`; remainder added to `accrued_hire` (it belongs to the current box `owner`). No proration. If locked: fail, no split, attached NEAR reverts.
- `off_hire()` — lessee, or owner after `now > lease.end` (default). Not locked. 1 yocto. Return `deposit` (posted by the lessee in `accept_lease`) minus a documented damage field if you implement damage in this version; if damage is not in the first cut, return the full deposit to the lessee and set `lease = None`. Do not block the six tests on a rich damage model.

### 5.5 Box — inspector

- `attest_inspection(valid_until: u64)` — predecessor == `inspector`, `valid_until > now`. Sets `inspection_valid_until`. Owner, lessee, and buyer calls must fail.

### 5.6 Box — views

- `get_owner`, `get_listing`, `get_lock`, `get_lease`, `get_lease_offer`, `get_inspection`, `get_reserve`, `get_accrued_hire`, `get_pending(account)`, `get_invariant_parts` (balance, storage, reserve, pending sum, deposits, locked payments).
- No `withdraw_reserve`. No `add_key`. No `update` / `deploy`. No `delete_account`.

### 5.7 Collection

- `mint(token_id or next_id, owner, inspector, inspection_valid_until)` — authorized minter (deployer for the prototype). Creates the box sub-account, deploys item code, **deletes all access keys**, inits state, registers NEP-171 token owned by `owner`.
- `nft_transfer`, `nft_transfer_call` — succeed only if `predecessor_account_id == item_account_for(token_id)`. Otherwise fail.
- `nft_approve`, `nft_revoke`, `nft_revoke_all` — fail for every caller in this version, or exist only as stubs that always fail. There must be no path where an approved account can move the token.
- Standard views: `nft_token`, `nft_metadata`, `nft_tokens_for_owner` as needed for a valid collection.
- A view `owner_of(token_id)` used by `rescue_sale`.

---

## 6. Tests (first deliverable)

Run in this order: sandbox (near-workspaces), then the same six on testnet. Use sandbox state patching to create a stuck lock. Use time (or block) fast-forward for the inspection lapse and for the undo delay. After **every** step of every test, assert the invariant in section 2 rule 14, and assert the box account has **zero** access keys.

### Test 1 — Mid-lease sale

1. Mint a box to Owner. Inspector attests a future `valid_until`. Owner offers a lease to Lessee (`start_lease`, 1 yocto) and Lessee accepts it, posting the deposit (`accept_lease`).
2. Lessee makes a lease payment. Reserve increases by 15%; remainder is accrued to Owner.
3. Owner lists a price. Buyer calls `buy` with that exact amount.
4. Sale finalizes (callback or, if you must, a legal finalize path).
5. Assert: Buyer is owner on both box and collection. Seller's pending equals `price + lease payments earned before the sale`. Lease terms and reserve are unchanged. Lessee is still the lessee.
6. Lessee makes another lease payment. Accrued / pending for that payment belongs to Buyer, not Seller.
7. Seller withdraws and receives only `price + pre-sale lease payments`. Seller cannot withdraw the post-sale lease payment. Buyer cannot withdraw the reserve.

### Test 2 — Failed collection transfer undoes the sale

Force the collection transfer to fail (for example, a test-only failure hook, a transfer to an invalid receiver if that fails cleanly, or a sandboxed collection that returns failure). Assert: box owner is still the seller; collection owner is still the seller; listing can remain; reserve, lease, and accrued lease payments are unchanged; lock is cleared; Buyer's attached price is in `pending_withdrawals[buyer]` (or otherwise withdrawable) and Buyer can `withdraw` it; Seller is not credited the price.

### Test 3 — Direct transfer is blocked

As Owner, with a listed or unlisted box, call on the collection: `nft_transfer`, `nft_transfer_call`, and an approval path (`nft_approve` and, if any approve could succeed, a follow-on transfer). All must fail. Owner stays owner on box and collection. No pending credits change.

### Test 4 — Stuck lock

Using sandbox state patching (do not rely on a real out-of-gas in CI as the only setup):

1. Create a lock whose `sale_id` is current, with collection still showing Seller. Call `rescue_sale` before `SALE_UNDO_DELAY_BLOCKS`. It must be refused. Lock remains.
2. Fast-forward past the delay, collection still shows Seller. `rescue_sale` undoes: Buyer can withdraw the price; Seller remains owner; late `on_nft_transfer` for that `sale_id` is a no-op.
3. Set up the opposite patch: collection already shows Buyer, lock still set (simulating a successful transfer whose callback never ran). `rescue_sale` may be called immediately (no delay required) and must finalize: Buyer is owner on the box; Seller is credited `price + accrued lease payments`; a late callback for that `sale_id` is a no-op.
4. After a new `buy` with a new `sale_id`, a callback or rescue for the **old** `sale_id` is a no-op.

### Test 5 — Lapsed inspection

1. Inspector attests a short `valid_until`. Owner offers a lease and Lessee accepts it, posting the deposit (still valid). An owner attaching NEAR to `start_lease` is refused; accepts by anyone but the named lessee, or with the wrong deposit, are refused. Fast-forward past `valid_until`.
2. The active lease still works: `pay_hire` succeeds.
3. After the lease ends (or after `off_hire`), `start_lease` fails because inspection has lapsed.
4. Owner's `attest_inspection` fails. Buyer's and lessee's attest calls fail.
5. Inspector attests a new future `valid_until`. `start_lease` succeeds.

### Test 6 — Reserve cut and owner cannot take it

1. Lessee `pay_hire(X)`. `reserve` increases by `X * 1500 / 10000`. Remainder accrues to the current owner.
2. There is no successful way for the owner (or lessee, or buyer) to withdraw the reserve. Calls to any reserved-looking method fail if present; `withdraw` does not reduce `reserve`.
3. Invariant holds.

### 6.1 First thing the reviewer sees

The six tests passing in sandbox, then the same six passing on testnet, with logs and, for testnet, explorer links. Do not present a UI, a video, or a “it's working” demo as the first proof.

---

## 7. Threat model (build against this)

| Attacker goal | Why it must fail |
| --- | --- |
| Owner calls collection `nft_transfer` and skips `buy` | Collection gates every transfer path to the box. Test 3. |
| Buyer is charged but ownership never moves | Failed transfer undoes; buyer withdraws the price. Test 2. |
| Transfer succeeds, callback never runs, box still pays the seller | `rescue_sale` finalizes when the collection shows the buyer. Test 4. |
| Rescue undoes while transfer is still in flight, then transfer lands, buyer has the box and a refund | Undo only after `SALE_UNDO_DELAY_BLOCKS`; sale_id makes late callbacks no-ops; finalize if collection already shows buyer. Test 4. |
| Late callback after undo or after a new sale | Stale `sale_id` is a no-op. Test 4. |
| A lease payment arrives mid-sale and is credited to the wrong person | `pay_hire` rejected while locked. After finalize, the next lease payment goes to the buyer; after undo, to original owner. |
| Named lessee ends the lease at once and withdraws a deposit the owner posted | The owner cannot post a deposit (`start_lease` takes exactly 1 yocto); only the lessee posts it, in `accept_lease`, and `off_hire` returns that same amount to the lessee. Unit tests; test 5. |
| A lease is recorded against an account that never agreed to it | A lease exists only after the named lessee calls `accept_lease`. Unit tests; test 5. |
| Owner attests their own inspection | Only inspector can attest. Test 5. |
| Owner withdraws the reserve | No such path. Test 6. |
| Anyone with a key drains the box or swaps code | No keys on box accounts; no upgrade or delete path. Asserted in every test. |
| Seller `withdraw` fails, so the sale is reversed | Pull balances; withdrawal is a separate call. Test 1. |
| Direct approval or `nft_transfer_call` bypass | Test 3 covers all three paths. |

---

## 8. Repository and engineering

### 8.1 Suggested layout

```
onhire/
  README.md
  LICENSE                 # proprietary, all rights reserved
  SECURITY.md
  Cargo.toml              # workspace
  contracts/
    collection/
    item/
  tests/                  # near-workspaces integration tests (the six)
  scripts/
    demo.sh               # after tests are green; not the first deliverable
    deploy-testnet.sh
  .gitignore
```

### 8.2 Tooling

- Rust, `near-sdk`, `cargo-near` or the current standard NEAR contract toolchain.
- `near-workspaces` for sandbox and testnet tests.
- `cargo test` (unit) plus a clearly documented integration command that runs the six tests.
- No unnecessary dependencies. No mainnet config in default scripts.

### 8.3 README (required content)

Write it for a machine reader and a human judge:

- What it is, the problem, and how NEAR fits (NFT-as-account, native NEAR lease payments, contract-enforced rules).
- Testnet only, unaudited, not for real funds.
- “Do X → see Y” for each of the six tests and for the mid-lease sale demo.
- How control passes on purchase (list, `buy`, callback, rescue, sale_id, undo delay).
- Threat-model table (section 7, shortened).
- Constants, including `SALE_UNDO_DELAY_BLOCKS` and the measured reason it is long enough.
- Disclosure: the collection deployer key exists on testnet; box accounts have no keys and no upgrade path.
- Libraries and toolchain.
- Q-015 disclosure, exact line: `The author also entered openLID, an independent project, in this hackathon: https://github.com/tnvolman/openlid`
- Credit: Michael McClanahan (@tnvolman). License: proprietary, all rights reserved, limited NEAR HACKATHON evaluation grant (see `LICENSE`).

A matching disclosure belongs in the openLID README: `The author also entered Onhire, an independent project, in this hackathon: https://github.com/tnvolman/onhire`.

### 8.4 GitHub

- Repository: [`tnvolman/onhire`](https://github.com/tnvolman/onhire).

---

## 9. Implementation order

Work in this order. Do not start UI, video, or stretch features until the must-ship tests are green in sandbox.

1. Threat model is in section 7. Implement interfaces and test scaffolding first, including the six tests as failing tests if going test-first.
2. Collection mint (keyless box accounts) + item state + `list` / `cancel_list` / `buy` / callback / `rescue_sale` / `withdraw` / `pay_hire` / `start_lease` / `attest_inspection`.
3. Make tests 1–6 pass in sandbox.
4. Deploy to testnet and pass the same six.
5. A demo that replays the mid-lease sale story with explorer links. A small local UI (owner / lessee / buyer panes, per-box history) only after the tests are green.
6. Quality gate: mid-lease sale (test 1) and the failed-transfer plus rescue behavior (tests 2 and 4) must work before further features are added. If they do not, ship a smaller version without mid-lease sale and document that clearly.

### 9.1 Stretch (only if must-ship tests are green)

- Depot repair paid from the reserve to an allowlisted depot, with a signed inspection that also renews `inspection_valid_until`. Owner still cannot attest or withdraw the reserve.
- Seal or spare-part NFTs owned by the box that move with it on sale.
- Hosted read-only explorer.
- Fleet income view.

---

## 10. Acceptance criteria

The product meets this spec when:

- Tests 1–6 pass in sandbox, with invariant and zero-key checks after every step.
- Tests 1–6 pass on testnet, with commands and explorer links documented.
- `SALE_UNDO_DELAY_BLOCKS` is 32 or a documented raise, never a silent lower.
- Collection transfer, transfer_call, and approval paths are gated; test 3 fails those calls.
- Box accounts have no keys and no upgrade or delete path in code.
- README includes Q-015, the deployer-key disclosure, testnet warnings, and credit to Michael McClanahan (@tnvolman).
- LICENSE is the owner-approved proprietary license (all rights reserved, limited NEAR HACKATHON evaluation grant).
- No secrets, no mainnet keys, and no real funds.

---

## 11. Sources and prior decisions

This spec freezes the plan agreed on Nov 11, 2026, including the v1.1 additions and review clarifications (undo delay must outlast an in-flight transfer; test 3 includes approvals; exact-amount `buy`; cancel of an unlocked listing; native NEAR and pull balances). It supersedes the earlier memo's “test USDC or NEAR” and “lessee function-call key” and “read owner from the collection on each lease payment” ideas. Those are out.

- Official rules: https://nearhackathon.com/HACKATHON.md (Q-015, Q-012, Q-017)
- Track 10 prompt: https://github.com/nearai/near-hackathon-prompt-examples/blob/main/tracks/10-nfts.md