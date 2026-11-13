//! The seven sandbox tests (spec section 6 plus t7, the protocol fee), in
//! order, against a near-workspaces sandbox. Run with `scripts/test-sandbox.sh`
//! (`--test-threads=1` keeps them in order t1..t7).
//!
//! After every step: rule-14 invariant and zero access keys (`Env::check`).
//! Every hire payment and sale also checks the 1% protocol fee: the fee
//! account receives exactly 1% and the 15% reserve is untouched by it.

use anyhow::{ensure, Result};
use onhire_tests::*;
use serde_json::json;

const DAY: u64 = 86_400;

/// Mint + inspector attests + lease (owner offers with 1 yocto, the lessee
/// accepts and posts `deposit`), shared by several tests.
async fn leased_box(e: &Env, deposit: u128) -> Result<near_workspaces::AccountId> {
    let b = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b, "minted").await?;
    let now = e.now_secs().await?;
    e.ok(&e.inspector, &b, "attest_inspection", json!({ "valid_until": now + 60 * DAY }), 0).await?;
    e.check(&b, "inspector attested").await?;
    e.ok(&e.owner, &b, "start_lease",
        json!({ "lessee": e.lessee.id(), "rate_per_day": (NEAR / 10).to_string(), "duration_secs": 30 * DAY,
                "deposit": deposit.to_string() }),
        1).await?;
    e.check(&b, "lease offered").await?;
    e.ok(&e.lessee, &b, "accept_lease", json!({}), deposit).await?;
    e.check(&b, "lessee accepted lease (posted deposit)").await?;
    Ok(b)
}

// ---------------------------------------------------------------------------
// Test 1 — Mid-lease sale
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t1_mid_lease_sale() -> Result<()> {
    let e = Env::new("test1").await?;
    // 1. mint, attest, lease
    let b = leased_box(&e, 2 * NEAR).await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    let lease_before = e.bview(&b, "get_lease").await?;

    // 2. Lessee pays hire.
    let hire1 = NEAR;
    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), hire1).await?;
    e.check(&b, "lessee paid hire #1").await?;
    let reserve1 = e.reserve(&b).await?;
    ensure!(reserve1 == reserve_cut(hire1), "reserve {reserve1} (fee must not reduce it)");
    ensure!(e.accrued(&b).await? == owner_cut(hire1), "owner accrues 84%");
    let f1 = e.assert_fee(f0, fee_cut(hire1), "hire #1: 1% of the payment").await?;

    // 3. Owner lists; buyer buys with exact amount.
    let price = 10 * NEAR;
    e.ok(&e.owner, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "owner listed").await?;
    e.fail(&e.buyer, &b, "buy", json!({}), price - 1, "ERR_WRONG_ATTACHED_AMOUNT").await?;
    ensure!(e.bview(&b, "get_lock").await?.is_null(), "wrong amount must not lock");
    e.check(&b, "wrong-amount buy refused").await?;
    let outcome = e.buy(&e.buyer, &b, price).await?;
    // 4. Finalized by the callback.
    ensure!(outcome == "finalized", "buy outcome {outcome}");
    e.check(&b, "buyer bought; callback finalized").await?;
    let f2 = e.assert_fee(f1, fee_cut(price), "sale: 1% of the price").await?;

    // 5. Assertions.
    ensure!(e.box_owner(&b).await? == e.buyer.id().as_str());
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.buyer.id().as_str()));
    let pre_sale_hire = owner_cut(hire1);
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + pre_sale_hire, "seller pending = 99% of price + pre-sale hire");
    ensure!(e.reserve(&b).await? == reserve1, "reserve unchanged by the sale or its fee");
    ensure!(e.bview(&b, "get_lease").await? == lease_before, "lease unchanged");
    ensure!(e.accrued(&b).await? == 0);
    ensure!(e.bview(&b, "get_listing").await?.is_null());
    e.check(&b, "post-sale state asserted").await?;

    // 6. Lessee pays again: belongs to buyer.
    let hire2 = 2 * NEAR;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), hire2).await?;
    e.check(&b, "lessee paid hire #2 (post-sale)").await?;
    ensure!(e.accrued(&b).await? == owner_cut(hire2), "post-sale hire accrues to buyer");
    ensure!(e.reserve(&b).await? == reserve1 + reserve_cut(hire2), "reserve +15% exactly");
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + pre_sale_hire, "seller pending unchanged");
    e.assert_fee(f2, fee_cut(hire2), "hire #2: 1% of the payment").await?;
    ensure!(e.fees_paid(&b).await? == fee_cut(hire1) + fee_cut(price) + fee_cut(hire2), "box fees_paid counter");

    // 7. Seller withdraws only price + pre-sale hire.
    let got = e.withdraw(&e.owner, &b).await?;
    ensure!(got == seller_net(price) + pre_sale_hire);
    e.check(&b, "seller withdrew 99% of price + pre-sale hire").await?;
    e.fail(&e.owner, &b, "withdraw", json!({}), 0, "ERR_NOTHING_TO_WITHDRAW").await?;
    e.check(&b, "seller second withdraw refused").await?;
    let reserve_before = e.reserve(&b).await?;
    let got = e.withdraw(&e.buyer, &b).await?;
    ensure!(got == owner_cut(hire2), "buyer withdrew only post-sale hire");
    ensure!(e.reserve(&b).await? == reserve_before, "buyer cannot withdraw reserve");
    e.check(&b, "buyer withdrew post-sale hire; reserve intact").await?;
    e.fail(&e.buyer, &b, "withdraw_reserve", json!({}), 0, "MethodNotFound").await?;
    e.check(&b, "buyer withdraw_reserve absent").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2 — Failed collection transfer undoes the sale
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t2_failed_transfer_undoes() -> Result<()> {
    let e = Env::new("test2").await?;
    let b = leased_box(&e, NEAR).await?;
    // A second box for the same owner, so the forced failure below is a
    // storage-growth failure and not an accident of set cleanup.
    let b2 = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b2, "second box minted").await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR).await?;
    e.check(&b, "hire paid").await?;
    let price = 5 * NEAR;
    e.ok(&e.owner, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "listed").await?;
    let (reserve, accrued, lease) = (e.reserve(&b).await?, e.accrued(&b).await?, e.bview(&b, "get_lease").await?);
    let f0 = e.fee_balance().await?;
    let fees0 = e.fees_paid(&b).await?;

    // Force the collection transfer to fail: sandbox-patch the collection's
    // balance down to exactly its storage stake, so the transfer (which must
    // store the buyer's new owner set) fails with LackBalanceForState.
    let coll = e.collection.id().clone();
    let acct = e.worker.view_account(&coll).await?;
    let original = acct.balance;
    let exact = acct.storage_usage as u128 * STORAGE_BYTE_COST;
    e.worker
        .patch(&coll)
        .account_from_current(move |a| {
            near_workspaces::types::AccountDetailsPatch::from(a).balance(yocto(exact))
        })
        .transact()
        .await?;
    println!("[test2]   collection balance patched to its storage stake ({} NEAR)", fmt_near(exact));

    let r = e.buyer.call(&b, "buy").deposit(yocto(price))
        .gas(near_workspaces::types::Gas::from_tgas(150)).transact().await?;
    let failures = format!("{:?}", r.receipt_failures());
    ensure!(failures.contains(e.collection.id().as_str()) && failures.contains("LackBalanceForState"),
        "expected the collection nft_transfer receipt to fail with LackBalanceForState: {failures}");
    println!("[test2]   collection nft_transfer receipt failed: LackBalanceForState");
    let outcome: String = r.into_result().map_err(|e| anyhow::anyhow!("{e:?}"))?.json()?;
    ensure!(outcome == "undone", "expected undo, got {outcome}");
    e.check(&b, "buy with failing transfer -> callback undid immediately").await?;

    // Restore the collection balance.
    e.worker
        .patch(&coll)
        .account_from_current(move |a| near_workspaces::types::AccountDetailsPatch::from(a).balance(original))
        .transact()
        .await?;

    ensure!(e.box_owner(&b).await? == e.owner.id().as_str(), "box owner still seller");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.owner.id().as_str()), "collection owner still seller");
    ensure!(!e.bview(&b, "get_listing").await?.is_null(), "listing remains");
    ensure!(e.reserve(&b).await? == reserve && e.accrued(&b).await? == accrued);
    ensure!(e.bview(&b, "get_lease").await? == lease, "lease unchanged");
    ensure!(e.bview(&b, "get_lock").await?.is_null(), "lock cleared");
    ensure!(e.pending(&b, &e.buyer).await? == price, "buyer refund pending");
    ensure!(e.pending(&b, &e.owner).await? == 0, "seller not credited");
    e.assert_fee(f0, 0, "undone sale charges no fee").await?;
    ensure!(e.fees_paid(&b).await? == fees0);
    e.check(&b, "undo state asserted").await?;
    let got = e.withdraw(&e.buyer, &b).await?;
    ensure!(got == price);
    e.check(&b, "buyer withdrew refund").await?;

    // The listing remains, so the buyer can retry and now succeeds.
    ensure!(e.buy(&e.buyer, &b, price).await? == "finalized");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.buyer.id().as_str()));
    e.assert_fee(f0, fee_cut(price), "retry sale: 1% of the price").await?;
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + accrued, "seller credited 99% + accrued");
    ensure!(e.reserve(&b).await? == reserve, "reserve untouched by the sale fee");
    e.check(&b, "retry buy finalized after collection restored").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3 — Direct transfer is blocked
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t3_direct_transfer_blocked() -> Result<()> {
    let e = Env::new("test3").await?;
    let b = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b, "minted").await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    let coll = e.collection.id().clone();
    let deployer = e.collection.as_account().clone();

    for listed in [false, true] {
        let tag = if listed { "listed" } else { "unlisted" };
        if listed {
            e.ok(&e.owner, &b, "list", json!({ "price": (3 * NEAR).to_string() }), 1).await?;
            e.check(&b, "owner listed").await?;
        }
        let pending_before = (e.pending(&b, &e.owner).await?, e.pending(&b, &e.buyer).await?);
        let t = json!({ "receiver_id": e.buyer.id(), "token_id": token });
        e.fail(&e.owner, &coll, "nft_transfer", t.clone(), 1, "ERR_ONLY_BOX").await?;
        e.check(&b, &format!("[{tag}] owner nft_transfer refused")).await?;
        let tc = json!({ "receiver_id": e.buyer.id(), "token_id": token, "msg": "" });
        e.fail(&e.owner, &coll, "nft_transfer_call", tc, 1, "ERR_ONLY_BOX").await?;
        e.check(&b, &format!("[{tag}] owner nft_transfer_call refused")).await?;
        let ap = json!({ "token_id": token, "account_id": e.buyer.id() });
        e.fail(&e.owner, &coll, "nft_approve", ap.clone(), NEAR / 100, "ERR_APPROVALS_DISABLED").await?;
        e.check(&b, &format!("[{tag}] owner nft_approve refused")).await?;
        e.fail(&e.owner, &coll, "nft_revoke", ap, 1, "ERR_APPROVALS_DISABLED").await?;
        e.fail(&e.owner, &coll, "nft_revoke_all", json!({ "token_id": token }), 1, "ERR_APPROVALS_DISABLED").await?;
        e.check(&b, &format!("[{tag}] owner nft_revoke / nft_revoke_all refused")).await?;
        // Would-be approved account / marketplace transfer with an approval id.
        let ta = json!({ "receiver_id": e.buyer.id(), "token_id": token, "approval_id": 0 });
        e.fail(&e.buyer, &coll, "nft_transfer", ta, 1, "ERR_ONLY_BOX").await?;
        e.check(&b, &format!("[{tag}] approved-account style nft_transfer refused")).await?;
        // Even the collection deployer key cannot move a box.
        e.fail(&deployer, &coll, "nft_transfer", t, 1, "ERR_ONLY_BOX").await?;
        e.check(&b, &format!("[{tag}] collection deployer nft_transfer refused")).await?;
        let approved = e.view(&coll, "nft_is_approved",
            json!({ "token_id": token, "approved_account_id": e.buyer.id() })).await?;
        ensure!(approved == json!(false));

        ensure!(e.box_owner(&b).await? == e.owner.id().as_str());
        ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.owner.id().as_str()));
        ensure!(pending_before == (e.pending(&b, &e.owner).await?, e.pending(&b, &e.buyer).await?), "pending changed");
        e.check(&b, &format!("[{tag}] owner unchanged on box and collection; no pending change")).await?;
    }
    ensure!(e.fees_paid(&b).await? == 0, "blocked transfers pay no fee");
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4 — Stuck lock (sandbox state patching)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t4_stuck_lock() -> Result<()> {
    let e = Env::new("test4").await?;
    let b = leased_box(&e, NEAR).await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR).await?;
    e.check(&b, "hire paid").await?;
    let price = 4 * NEAR;
    e.ok(&e.owner, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "listed").await?;
    let f0 = e.fee_balance().await?;
    let reserve0 = e.reserve(&b).await?;

    // ---- 4.1 Patch in a lock whose sale_id is current; collection shows seller.
    let mut st = e.box_state(&b).await?;
    let sid = st.next_sale_id;
    st.next_sale_id += 1;
    let started = e.height().await?;
    st.lock = Some(onhire_item::SaleLock {
        sale_id: sid,
        seller: sdk_id(&e.owner),
        buyer: sdk_id(&e.buyer),
        price: near_sdk::json_types::U128(price),
        started_at_block: started,
    });
    st.locked_total += price;
    // The buyer's attached NEAR is on the box, as it would be after `buy`.
    e.patch_box(&b, &st, vec![(locked_key(sid), borsh_u128(price))], price).await?;
    ensure!(e.bview(&b, "get_lock").await?["sale_id"] == json!(sid));
    ensure!(e.u128_view(&b, "get_locked_payment", json!({ "sale_id": sid })).await? == price);
    e.check(&b, "4.1 patched stuck lock (collection shows seller)").await?;

    // While locked: hire and lease/sale changes are refused.
    e.fail(&e.lessee, &b, "pay_hire", json!({}), NEAR, "ERR_SALE_LOCKED").await?;
    e.fail(&e.owner, &b, "cancel_list", json!({}), 1, "ERR_SALE_LOCKED").await?;
    e.fail(&e.lessee, &b, "accept_lease", json!({}), NEAR, "ERR_SALE_LOCKED").await?;
    e.fail(&e.owner, &b, "cancel_lease_offer", json!({}), 1, "ERR_SALE_LOCKED").await?;
    e.check(&b, "4.1 pay_hire / cancel_list / accept_lease / cancel_lease_offer refused while locked").await?;

    // Rescue before the delay: refused, lock stays.
    ensure!(e.height().await? < started + SALE_UNDO_DELAY_BLOCKS);
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid }), 0, "ERR_UNDO_DELAY_NOT_ELAPSED").await?;
    ensure!(e.bview(&b, "get_lock").await?["sale_id"] == json!(sid), "lock must remain");
    e.check(&b, "4.1 early rescue refused; lock remains").await?;

    // ---- 4.2 Fast-forward past the delay; rescue undoes.
    e.worker.fast_forward(SALE_UNDO_DELAY_BLOCKS + 2).await?;
    ensure!(e.height().await? >= started + SALE_UNDO_DELAY_BLOCKS);
    let r = e.ok(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid }), 0).await?;
    ensure!(r == json!("undone"), "rescue result {r}");
    e.check(&b, "4.2 rescue after delay undid the sale").await?;
    ensure!(e.box_owner(&b).await? == e.owner.id().as_str(), "seller remains owner");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.owner.id().as_str()));
    ensure!(e.pending(&b, &e.buyer).await? == price);
    ensure!(e.pending(&b, &e.owner).await? == 0);
    // Late callback for that sale_id: it cannot be injected (private), and a
    // repeated rescue is refused as stale. Neither changes state.
    let snap = e.box_state(&b).await?;
    e.fail(&e.buyer, &b, "on_nft_transfer", json!({ "sale_id": sid }), 0, "is private").await?;
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid }), 0, "ERR_STALE_SALE_ID").await?;
    ensure!(near_sdk::borsh::to_vec(&e.box_state(&b).await?)? == near_sdk::borsh::to_vec(&snap)?, "state changed");
    e.check(&b, "4.2 late callback / stale rescue are no-ops").await?;
    let got = e.withdraw(&e.buyer, &b).await?;
    ensure!(got == price);
    let f0 = e.assert_fee(f0, 0, "4.2 undone by rescue: no fee").await?;
    e.check(&b, "4.2 buyer withdrew the price").await?;

    // ---- 4.3 Opposite patch: collection shows buyer, lock still set.
    // A real sale lands (collection -> buyer), then the box is patched back
    // to its pre-callback state: exactly a successful transfer whose
    // callback never ran.
    let accrued_before = e.accrued(&b).await?;
    ensure!(accrued_before > 0);
    let pre = e.box_state(&b).await?;
    let sid2 = pre.next_sale_id;
    let started2 = e.height().await?;
    ensure!(e.buy(&e.buyer, &b, price).await? == "finalized");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.buyer.id().as_str()));
    let f1 = e.assert_fee(f0, fee_cut(price), "4.3 real sale: 1% of the price").await?;
    let mut st = e.box_state(&b).await?;
    st.owner = sdk_id(&e.owner);
    st.accrued_hire = accrued_before;
    st.listing = pre.listing.clone();
    st.lock = Some(onhire_item::SaleLock {
        sale_id: sid2,
        seller: sdk_id(&e.owner),
        buyer: sdk_id(&e.buyer),
        price: near_sdk::json_types::U128(price),
        started_at_block: started2,
    });
    st.pending_total -= seller_net(price) + accrued_before;
    st.locked_total += price;
    st.fees_paid -= fee_cut(price);
    let seller_pending_after_patch = 0u128;
    e.patch_box(
        &b,
        &st,
        vec![
            (locked_key(sid2), borsh_u128(price)),
            (pending_key(e.owner.id().as_str()), borsh_u128(seller_pending_after_patch)),
        ],
        // Before the callback the fee had not left the box yet.
        fee_cut(price),
    )
    .await?;
    ensure!(e.box_owner(&b).await? == e.owner.id().as_str());
    ensure!(e.pending(&b, &e.owner).await? == 0);
    e.check(&b, "4.3 patched lock with collection already showing buyer").await?;
    // Immediately (no delay), rescue finalizes.
    ensure!(e.height().await? < started2 + SALE_UNDO_DELAY_BLOCKS);
    let r = e.ok(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid2 }), 0).await?;
    ensure!(r == json!("finalized"), "rescue result {r}");
    e.check(&b, "4.3 rescue finalized immediately").await?;
    ensure!(e.box_owner(&b).await? == e.buyer.id().as_str());
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + accrued_before, "seller credited 99% of price + accrued hire");
    e.assert_fee(f1, fee_cut(price), "4.3 rescue finalize: 1% of the price").await?;
    ensure!(e.reserve(&b).await? == reserve0, "reserve untouched by sale fees");
    ensure!(e.accrued(&b).await? == 0);
    let snap = e.box_state(&b).await?;
    e.fail(&e.owner, &b, "on_nft_transfer", json!({ "sale_id": sid2 }), 0, "is private").await?;
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid2 }), 0, "ERR_STALE_SALE_ID").await?;
    ensure!(near_sdk::borsh::to_vec(&e.box_state(&b).await?)? == near_sdk::borsh::to_vec(&snap)?, "state changed");
    e.check(&b, "4.3 late callback / stale rescue are no-ops").await?;

    // ---- 4.4 New sale with a new sale_id; old id is a no-op.
    e.ok(&e.buyer, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "4.4 new owner listed").await?;
    // Patch a current lock sid3 (outsider buying) so we can poke the old id
    // while a newer sale is in flight.
    let mut st = e.box_state(&b).await?;
    let sid3 = st.next_sale_id;
    ensure!(sid3 > sid2);
    st.next_sale_id += 1;
    let started3 = e.height().await?;
    st.lock = Some(onhire_item::SaleLock {
        sale_id: sid3,
        seller: sdk_id(&e.buyer),
        buyer: sdk_id(&e.outsider),
        price: near_sdk::json_types::U128(price),
        started_at_block: started3,
    });
    st.locked_total += price;
    e.patch_box(&b, &st, vec![(locked_key(sid3), borsh_u128(price))], price).await?;
    e.check(&b, "4.4 newer lock sid3 in place").await?;
    let snap = e.box_state(&b).await?;
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid2 }), 0, "ERR_STALE_SALE_ID").await?;
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid }), 0, "ERR_STALE_SALE_ID").await?;
    e.fail(&e.outsider, &b, "on_nft_transfer", json!({ "sale_id": sid2 }), 0, "is private").await?;
    ensure!(near_sdk::borsh::to_vec(&e.box_state(&b).await?)? == near_sdk::borsh::to_vec(&snap)?, "state changed");
    e.check(&b, "4.4 old sale_id rescue/callback are no-ops while sid3 locked").await?;
    // Clean up sid3 through the legal path: after the delay, undo.
    e.worker.fast_forward(SALE_UNDO_DELAY_BLOCKS + 2).await?;
    ensure!(e.ok(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid3 }), 0).await? == json!("undone"));
    e.check(&b, "4.4 sid3 undone after delay").await?;
    // And a real new buy (sid4) finalizes; old ids still stale.
    let f2 = e.fee_balance().await?;
    let outcome = e.buy(&e.outsider, &b, price).await?;
    ensure!(outcome == "finalized");
    e.assert_fee(f2, fee_cut(price), "4.4 sid4 sale: 1% of the price (sid3 undo paid none)").await?;
    ensure!(e.reserve(&b).await? == reserve0, "reserve untouched by sale fees");
    e.check(&b, "4.4 real new buy finalized").await?;
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid3 }), 0, "ERR_STALE_SALE_ID").await?;
    e.check(&b, "4.4 rescue for old sid3 is a no-op").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5 — Lapsed inspection
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t5_lapsed_inspection() -> Result<()> {
    let e = Env::new("test5").await?;
    let b = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b, "minted").await?;
    // 1. Short valid_until; lease starts while valid; fast-forward past it.
    let valid_until = e.now_secs().await? + 120;
    e.ok(&e.inspector, &b, "attest_inspection", json!({ "valid_until": valid_until }), 0).await?;
    e.check(&b, "inspector attested short validity").await?;
    let offer = json!({ "lessee": e.lessee.id(), "rate_per_day": NEAR.to_string(), "duration_secs": 10 * DAY,
                        "deposit": NEAR.to_string() });
    // The old attack: the owner can no longer attach NEAR as the deposit.
    e.fail(&e.owner, &b, "start_lease", offer.clone(), NEAR, "Requires attached deposit of exactly 1 yoctoNEAR").await?;
    ensure!(e.bview(&b, "get_lease_offer").await?.is_null() && e.bview(&b, "get_lease").await?.is_null());
    e.check(&b, "start_lease with owner-attached NEAR refused").await?;
    e.ok(&e.owner, &b, "start_lease", offer, 1).await?;
    ensure!(e.bview(&b, "get_lease").await?.is_null(), "offer does not start the lease");
    ensure!(e.bview(&b, "get_lease_offer").await?["deposit"] == json!(NEAR.to_string()));
    e.check(&b, "lease offered (owner attached 1 yocto, no deposit held)").await?;
    // Only the named lessee, with exactly the offered deposit.
    e.fail(&e.owner, &b, "accept_lease", json!({}), NEAR, "ERR_NOT_LESSEE").await?;
    e.fail(&e.outsider, &b, "accept_lease", json!({}), NEAR, "ERR_NOT_LESSEE").await?;
    e.fail(&e.lessee, &b, "accept_lease", json!({}), NEAR - 1, "ERR_WRONG_DEPOSIT").await?;
    e.fail(&e.lessee, &b, "accept_lease", json!({}), NEAR + 1, "ERR_WRONG_DEPOSIT").await?;
    e.check(&b, "accept by owner/outsider or with a wrong deposit refused").await?;
    // The deposit comes from the lessee (it pays the deposit plus gas; unused
    // prepaid gas may not be refunded yet when the balance is read).
    let lessee0 = e.balance(e.lessee.id()).await?;
    e.ok(&e.lessee, &b, "accept_lease", json!({}), NEAR).await?;
    let lessee1 = e.balance(e.lessee.id()).await?;
    ensure!(lessee0 >= lessee1 + NEAR && lessee0 < lessee1 + NEAR + NEAR / 5,
        "lessee paid the deposit ({lessee0} -> {lessee1})");
    let parts = e.bview(&b, "get_invariant_parts").await?;
    ensure!(parts["held_lessee_deposits"] == json!(NEAR.to_string()), "box holds the lessee deposit");
    ensure!(e.bview(&b, "get_lease").await?["deposit"] == json!(NEAR.to_string()));
    ensure!(e.bview(&b, "get_lease_offer").await?.is_null());
    e.check(&b, "lessee accepted lease while inspection valid (posted deposit)").await?;
    let mut n = 0;
    while e.now_secs().await? <= valid_until {
        e.worker.fast_forward(200).await?;
        n += 1;
        ensure!(n < 100, "time did not advance");
    }
    ensure!(e.bview(&b, "get_inspection").await?["valid_now"] == json!(false));
    e.check(&b, "fast-forwarded past valid_until").await?;

    // 2. Active lease still works.
    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR).await?;
    e.assert_fee(f0, fee_cut(NEAR), "hire after lapse: 1%").await?;
    ensure!(e.reserve(&b).await? == reserve_cut(NEAR) && e.accrued(&b).await? == owner_cut(NEAR));
    e.check(&b, "pay_hire on active lease after lapse succeeds").await?;

    // 3. After off_hire, start_lease fails.
    e.ok(&e.lessee, &b, "off_hire", json!({}), 1).await?;
    e.check(&b, "lessee off-hired (deposit credited)").await?;
    ensure!(e.pending(&b, &e.owner).await? == 0, "owner is credited nothing from the deposit");
    let got = e.withdraw(&e.lessee, &b).await?;
    ensure!(got == NEAR, "deposit returned to the lessee: exactly what the lessee paid");
    e.fail(&e.lessee, &b, "withdraw", json!({}), 0, "ERR_NOTHING_TO_WITHDRAW").await?;
    e.check(&b, "lessee withdrew its own deposit, nothing more").await?;
    let sl = json!({ "lessee": e.lessee.id(), "rate_per_day": NEAR.to_string(), "duration_secs": DAY,
                     "deposit": NEAR.to_string() });
    e.fail(&e.owner, &b, "start_lease", sl.clone(), 1, "ERR_INSPECTION_LAPSED").await?;
    e.check(&b, "start_lease refused: inspection lapsed").await?;

    // 4. Owner, buyer, lessee cannot attest.
    let future = e.now_secs().await? + 30 * DAY;
    for who in [&e.owner, &e.buyer, &e.lessee] {
        e.fail(who, &b, "attest_inspection", json!({ "valid_until": future }), 0, "ERR_NOT_INSPECTOR").await?;
    }
    e.check(&b, "owner/buyer/lessee attest refused").await?;

    // 5. Inspector attests; start_lease succeeds.
    e.ok(&e.inspector, &b, "attest_inspection", json!({ "valid_until": future }), 0).await?;
    e.check(&b, "inspector re-attested").await?;
    e.ok(&e.owner, &b, "start_lease", sl, 1).await?;
    e.check(&b, "start_lease succeeds after re-attestation").await?;
    e.ok(&e.lessee, &b, "accept_lease", json!({}), NEAR).await?;
    ensure!(e.bview(&b, "get_lease").await?["lessee"] == json!(e.lessee.id()));
    e.check(&b, "lessee accepted the new lease").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 6 — Reserve cut and owner cannot take it
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t6_reserve_cannot_be_taken() -> Result<()> {
    let e = Env::new("test6").await?;
    let b = leased_box(&e, NEAR).await?;
    // 1. pay_hire(X) with an awkward X to exercise rounding.
    let x: u128 = 1_234_567_890_123_456_789_012_345;
    let r0 = e.reserve(&b).await?;
    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), x).await?;
    e.check(&b, "lessee paid X").await?;
    let reserve = e.reserve(&b).await?;
    ensure!(reserve - r0 == x * 1500 / 10_000, "reserve cut is the full 15% (fee not taken from it)");
    ensure!(e.accrued(&b).await? == x - x * 1500 / 10_000 - x * 100 / 10_000, "owner accrues 85% - 1% fee");
    e.assert_fee(f0, x * 100 / 10_000, "hire X: 1% of the payment").await?;
    println!("[test6]   X={} reserve+={} fee={} accrued={}", x, reserve - r0, fee_cut(x), owner_cut(x));

    // 2. No way to withdraw the reserve.
    for m in ["withdraw_reserve", "reserve_withdraw", "claim_reserve", "add_key", "add_full_access_key",
              "delete_account", "update", "upgrade", "deploy", "migrate"] {
        for who in [&e.owner, &e.lessee, &e.buyer] {
            e.fail(who, &b, m, json!({}), 0, "MethodNotFound").await?;
        }
    }
    e.check(&b, "reserve/key/upgrade/delete methods do not exist").await?;
    let got = e.withdraw(&e.owner, &b).await?;
    ensure!(got == owner_cut(x), "owner withdraw = accrued hire only");
    ensure!(e.reserve(&b).await? == reserve, "withdraw does not reduce reserve");
    e.check(&b, "owner withdraw took hire only; reserve intact").await?;
    for who in [&e.owner, &e.lessee, &e.buyer, &e.inspector] {
        e.fail(who, &b, "withdraw", json!({}), 0, "ERR_NOTHING_TO_WITHDRAW").await?;
    }
    ensure!(e.reserve(&b).await? == reserve);
    // 3. Invariant holds.
    e.check(&b, "nobody can withdraw the reserve; invariant holds").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 7 — 1% protocol fee: exact splits and fail-closed (sandbox only)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn t7_protocol_fee() -> Result<()> {
    let e = Env::new("test7").await?;
    // Constants and recipient, from the collection and the box.
    ensure!(e.view(e.collection.id(), "get_fee_account", json!({})).await? == json!(e.fee.id()));
    let b = leased_box(&e, NEAR).await?;
    let c = e.bview(&b, "get_constants").await?;
    ensure!(c["FEE_BPS"] == json!(100) && c["RESERVE_BPS"] == json!(1500) && c["SALE_UNDO_DELAY_BLOCKS"] == json!(32), "constants {c}");
    ensure!(c["BPS_DENOMINATOR"] == json!("10000") && c["FEE_ACCOUNT"] == json!(e.fee.id()), "constants {c}");
    ensure!(e.bview(&b, "get_fee_account").await? == json!(e.fee.id()));
    e.check(&b, "constants: FEE_BPS 100, RESERVE_BPS 1500, delay 32, fee account").await?;

    // Hire of exactly 100 yocto: reserve 15, fee 1, owner 84.
    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), 100).await?;
    ensure!(e.reserve(&b).await? == 15, "reserve 15");
    ensure!(e.accrued(&b).await? == 84, "owner 84");
    let f1 = e.assert_fee(f0, 1, "hire 100 -> 15 / 1 / 84").await?;
    e.check(&b, "hire 100 yocto split 15/1/84").await?;

    // Sale of exactly 100 yocto: fee 1, seller 99, reserve untouched.
    e.ok(&e.owner, &b, "list", json!({ "price": "100" }), 1).await?;
    ensure!(e.buy(&e.buyer, &b, 100).await? == "finalized");
    ensure!(e.reserve(&b).await? == 15, "reserve untouched by the sale");
    ensure!(e.pending(&b, &e.owner).await? == 99 + 84, "seller 99 + pre-sale hire 84");
    let f2 = e.assert_fee(f1, 1, "sale 100 -> 1 / 99").await?;
    ensure!(e.fees_paid(&b).await? == 2);
    e.check(&b, "sale 100 yocto split 1/99").await?;
    ensure!(e.withdraw(&e.owner, &b).await? == 99 + 84);
    e.check(&b, "seller withdrew 99 + 84").await?;

    // Fail closed: patch a hypothetical reserve_bps that leaves the owner
    // less than the 1% fee. pay_hire must revert entirely.
    let mut st = e.box_state(&b).await?;
    let real_reserve_bps = st.reserve_bps;
    st.reserve_bps = 9_950;
    e.patch_box(&b, &st, vec![], 0).await?;
    let snap = near_sdk::borsh::to_vec(&e.box_state(&b).await?)?;
    let box0 = e.balance(&b).await?;
    e.fail(&e.lessee, &b, "pay_hire", json!({}), NEAR, "ERR_FEE_EXCEEDS_OWNER_SHARE").await?;
    ensure!(near_sdk::borsh::to_vec(&e.box_state(&b).await?)? == snap, "state changed on revert");
    ensure!(e.balance(&b).await? < box0 + NEAR / 100, "box kept the attached hire");
    e.assert_fee(f2, 0, "reverted hire pays no fee").await?;
    e.check(&b, "fail-closed: pay_hire reverted, no partial split").await?;

    // Fail closed on sale: a hypothetical fee above the price refuses `buy`
    // before anything is locked.
    let mut st = e.box_state(&b).await?;
    st.reserve_bps = real_reserve_bps;
    st.fee_bps = 20_000;
    e.patch_box(&b, &st, vec![], 0).await?;
    e.ok(&e.buyer, &b, "list", json!({ "price": NEAR.to_string() }), 1).await?;
    let snap = near_sdk::borsh::to_vec(&e.box_state(&b).await?)?;
    e.fail(&e.outsider, &b, "buy", json!({}), NEAR, "ERR_FEE_EXCEEDS_OWNER_SHARE").await?;
    ensure!(near_sdk::borsh::to_vec(&e.box_state(&b).await?)? == snap, "state changed on revert");
    ensure!(e.bview(&b, "get_lock").await?.is_null(), "no lock");
    e.assert_fee(f2, 0, "reverted buy pays no fee").await?;
    e.check(&b, "fail-closed: buy reverted before lock").await?;
    Ok(())
}
