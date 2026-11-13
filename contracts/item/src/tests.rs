//! Unit tests for the box contract. Covers the paths that cannot be driven
//! from outside the box in a sandbox (private callbacks invoked by the box
//! itself with a stale or current `sale_id`).

use super::*;
use near_sdk::test_utils::{get_logs, VMContextBuilder};
use near_sdk::{testing_env, PromiseResult};

fn acc(s: &str) -> AccountId {
    s.parse().unwrap()
}
fn collection() -> AccountId {
    acc("onhire.test.near")
}
fn fee() -> AccountId {
    acc("fee.near")
}
fn box_id() -> AccountId {
    acc("box-0001.onhire.test.near")
}
const NEAR: u128 = 10u128.pow(24);

fn ctx(pred: &str, deposit: u128) -> VMContextBuilder {
    let mut b = VMContextBuilder::new();
    b.current_account_id(box_id())
        .predecessor_account_id(acc(pred))
        .signer_account_id(acc(pred))
        .account_balance(NearToken::from_near(100))
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .prepaid_gas(Gas::from_tgas(300))
        .block_timestamp(1_000 * NANOS_PER_SEC)
        .block_height(100);
    b
}

fn setup() -> BoxContract {
    testing_env!(ctx("onhire.test.near", 0).build());
    BoxContract::new(collection(), "0001".into(), acc("owner.near"), acc("inspector.near"), 10_000, fee())
}

fn locked_box() -> BoxContract {
    let mut c = setup();
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(5 * NEAR));
    testing_env!(ctx("buyer.near", 5 * NEAR).build());
    let _ = c.buy();
    assert_eq!(c.lock.as_ref().unwrap().sale_id, 1);
    c
}

/// Owner offers a lease (1 yocto) and the lessee accepts it, attaching
/// `deposit`.
fn offer(c: &mut BoxContract, deposit: u128) {
    testing_env!(ctx("owner.near", 1).build());
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400, U128(deposit));
}

fn lease(c: &mut BoxContract, deposit: u128) {
    offer(c, deposit);
    testing_env!(ctx("lessee.near", deposit).build());
    c.accept_lease();
}

fn fails<F: FnOnce()>(f: F, needle: &str) {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let e = r.expect_err(&format!("expected panic {needle}"));
    let msg = e
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert!(msg.contains(needle), "wanted {needle}, got {msg}");
}

fn callback_ctx(result: PromiseResult) {
    let b = ctx("box-0001.onhire.test.near", 0);
    testing_env!(
        b.build(),
        near_sdk::test_vm_config(),
        near_sdk::RuntimeFeesConfig::test(),
        Default::default(),
        vec![result]
    );
}

#[test]
fn reserve_split_is_15_percent() {
    let mut c = setup();
    lease(&mut c, NEAR);
    testing_env!(ctx("lessee.near", 10_000).build());
    c.pay_hire();
    assert_eq!(c.reserve, 1_500);
    // 1% fee comes out of the owner's 85%, never the reserve.
    assert_eq!(c.accrued_hire, 8_400);
    assert_eq!(c.fees_paid, 100);
}

#[test]
fn callback_success_finalizes_and_failure_undoes() {
    let mut c = locked_box();
    callback_ctx(PromiseResult::Successful(vec![]));
    assert_eq!(c.on_nft_transfer(1), "finalized");
    assert_eq!(c.owner, acc("buyer.near"));
    // Seller gets 99%; 1% went to the fee account.
    assert_eq!(c.get_pending(acc("owner.near")).0, 5 * NEAR - 5 * NEAR / 100);
    assert_eq!(c.fees_paid, 5 * NEAR / 100);

    let mut c = locked_box();
    callback_ctx(PromiseResult::Failed);
    assert_eq!(c.on_nft_transfer(1), "undone");
    assert_eq!(c.owner, acc("owner.near"));
    assert_eq!(c.get_pending(acc("buyer.near")).0, 5 * NEAR);
    assert_eq!(c.get_pending(acc("owner.near")).0, 0);
    assert!(c.listing.is_some());
    // An undone sale charges no fee; the buyer gets the full price back.
    assert_eq!(c.fees_paid, 0);
}

#[test]
fn stale_callback_is_noop() {
    let mut c = locked_box();
    callback_ctx(PromiseResult::Failed);
    assert_eq!(c.on_nft_transfer(1), "undone");
    // Late success callback for the same (now stale) sale id.
    callback_ctx(PromiseResult::Successful(vec![]));
    assert_eq!(c.on_nft_transfer(1), "noop");
    assert_eq!(c.owner, acc("owner.near"));
    assert_eq!(c.get_pending(acc("owner.near")).0, 0);
    assert!(get_logs().iter().any(|l| l.contains("stale sale_id 1")));

    // New sale (id 2); callback and rescue callback for id 1 are no-ops.
    testing_env!(ctx("buyer.near", 5 * NEAR).build());
    let _ = c.buy();
    let before = c.lock.clone();
    callback_ctx(PromiseResult::Failed);
    assert_eq!(c.on_nft_transfer(1), "noop");
    assert_eq!(c.on_rescue_owner(1, Ok(Some(acc("buyer.near")))), "noop");
    assert_eq!(c.lock, before);
}

#[test]
fn rescue_rules() {
    let mut c = locked_box();
    callback_ctx(PromiseResult::Successful(vec![]));
    // Seller still owner on collection, delay not elapsed -> refuse.
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut c2 = locked_box();
        callback_ctx(PromiseResult::Successful(vec![]));
        c2.on_rescue_owner(1, Ok(Some(acc("owner.near"))));
    }));
    assert!(r.is_err());
    // After delay -> undo.
    let mut b = ctx("box-0001.onhire.test.near", 0);
    b.block_height(100 + SALE_UNDO_DELAY_BLOCKS);
    testing_env!(b.build());
    assert_eq!(c.on_rescue_owner(1, Ok(Some(acc("owner.near")))), "undone");
    assert_eq!(c.get_pending(acc("buyer.near")).0, 5 * NEAR);
}

#[test]
#[should_panic(expected = "ERR_COLLECTION_OWNER_UNEXPECTED")]
fn rescue_refuses_unknown_owner() {
    let mut c = locked_box();
    callback_ctx(PromiseResult::Successful(vec![]));
    c.on_rescue_owner(1, Ok(Some(acc("mallory.near"))));
}

#[test]
#[should_panic(expected = "ERR_SALE_LOCKED")]
fn pay_hire_rejected_while_locked() {
    let mut c = setup();
    lease(&mut c, NEAR);
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(NEAR));
    testing_env!(ctx("buyer.near", NEAR).build());
    let _ = c.buy();
    testing_env!(ctx("lessee.near", 100).build());
    c.pay_hire();
}

#[test]
#[should_panic(expected = "ERR_WRONG_ATTACHED_AMOUNT")]
fn buy_wrong_amount() {
    let mut c = setup();
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(NEAR));
    testing_env!(ctx("buyer.near", NEAR - 1).build());
    let _ = c.buy();
}

#[test]
#[should_panic(expected = "ERR_BAD_INIT")]
fn inspector_cannot_be_owner() {
    testing_env!(ctx("onhire.test.near", 0).build());
    BoxContract::new(collection(), "0001".into(), acc("owner.near"), acc("owner.near"), 10, fee());
}

// ------------------------------------------------------------- protocol fee

#[test]
fn fee_constants() {
    assert_eq!(FEE_BPS, 100);
    assert_eq!(BPS_DENOMINATOR, 10_000);
    assert_eq!(RESERVE_BPS, 1500);
    assert_eq!(SALE_UNDO_DELAY_BLOCKS, 32);
    let c = setup();
    assert_eq!(c.fee_bps, FEE_BPS);
    assert_eq!(c.get_fee_account(), fee());
    assert_eq!(c.get_constants()["FEE_BPS"], 100);
    assert_eq!(c.get_constants()["FEE_ACCOUNT"], "fee.near");
}

#[test]
fn split_hire_100_is_15_1_84() {
    assert_eq!(
        split_hire(100, RESERVE_BPS, FEE_BPS),
        Some(HireSplit { to_reserve: 15, fee: 1, to_owner: 84 })
    );
    // Reserve is the same 15% it was before the fee existed.
    for x in [1u128, 99, 100, 101, 12_345, NEAR / 10, 123_456_789_012_345_678_901_234] {
        let s = split_hire(x, RESERVE_BPS, FEE_BPS).unwrap();
        assert_eq!(s.to_reserve, x * 1500 / 10_000);
        assert_eq!(s.fee, x * 100 / 10_000);
        assert_eq!(s.to_reserve + s.fee + s.to_owner, x);
    }
}

#[test]
fn split_sale_100_is_1_99() {
    assert_eq!(split_sale(100, FEE_BPS), Some(SaleSplit { fee: 1, to_seller: 99 }));
    let s = split_sale(5 * NEAR, FEE_BPS).unwrap();
    assert_eq!(s.fee + s.to_seller, 5 * NEAR);
    assert_eq!(s.fee, 5 * NEAR / 100);
}

#[test]
fn split_fails_closed_when_owner_share_cannot_cover_fee() {
    // Owner share 0.5% < 1% fee.
    assert_eq!(split_hire(10_000, 9_950, FEE_BPS), None);
    assert_eq!(split_sale(10_000, 20_000), None);
}

#[test]
fn pay_hire_100_yocto_sends_1_to_fee_account() {
    let mut c = setup();
    lease(&mut c, NEAR);
    let reserve0 = c.reserve;
    testing_env!(ctx("lessee.near", 100).build());
    c.pay_hire();
    assert_eq!(c.reserve - reserve0, 15);
    assert_eq!(c.accrued_hire, 84);
    assert_eq!(c.fees_paid, 1);
    let logs = get_logs();
    assert!(logs.iter().any(|l| l.contains("\"event\":\"fee_paid\"") && l.contains("fee.near") && l.contains("\"amount\":\"1\"")));
    assert!(logs.iter().any(|l| l.contains("hire_paid") && l.contains("\"to_reserve\":\"15\"") && l.contains("\"fee\":\"1\"") && l.contains("\"to_owner\":\"84\"")));
}

#[test]
fn sale_100_yocto_credits_99_and_fee_1_reserve_untouched() {
    let mut c = setup();
    lease(&mut c, NEAR);
    testing_env!(ctx("lessee.near", 100).build());
    c.pay_hire();
    let reserve = c.reserve;
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(100));
    testing_env!(ctx("buyer.near", 100).build());
    let _ = c.buy();
    callback_ctx(PromiseResult::Successful(vec![]));
    assert_eq!(c.on_nft_transfer(1), "finalized");
    // 99 sale proceeds + 84 pre-sale hire; 1 + 1 fees; reserve unchanged.
    assert_eq!(c.get_pending(acc("owner.near")).0, 99 + 84);
    assert_eq!(c.fees_paid, 2);
    assert_eq!(c.reserve, reserve);
    assert_eq!(c.locked_total, 0);
    assert!(get_logs().iter().any(|l| l.contains("fee_paid") && l.contains("\"source\":\"sale\"")));
}

#[test]
#[should_panic(expected = "ERR_FEE_EXCEEDS_OWNER_SHARE")]
fn pay_hire_reverts_when_owner_share_cannot_cover_fee() {
    let mut c = setup();
    lease(&mut c, NEAR);
    c.reserve_bps = 9_950; // hypothetical: leaves the owner 0.5%
    testing_env!(ctx("lessee.near", 10_000).build());
    c.pay_hire();
}

#[test]
#[should_panic(expected = "ERR_FEE_EXCEEDS_OWNER_SHARE")]
fn buy_reverts_before_locking_when_fee_exceeds_price() {
    let mut c = setup();
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(100));
    c.fee_bps = 20_000; // hypothetical: fee is 200% of the price
    testing_env!(ctx("buyer.near", 100).build());
    let _ = c.buy();
}

// ------------------------------------------------- two-step lease (offer/accept)

fn ctx_at(pred: &str, deposit: u128, secs: u64) -> VMContextBuilder {
    let mut b = ctx(pred, deposit);
    b.block_timestamp(secs * NANOS_PER_SEC);
    b
}

#[test]
fn offer_holds_nothing_and_accept_starts_clock_with_lessee_deposit() {
    let mut c = setup();
    offer(&mut c, 3 * NEAR);
    assert!(c.lease.is_none());
    assert_eq!(c.held_deposits(), 0);
    let o = c.get_lease_offer().unwrap();
    assert_eq!(o.lessee, acc("lessee.near"));
    assert_eq!(o.deposit, U128(3 * NEAR));
    assert_eq!(o.duration_secs, 86_400);
    assert_eq!(o.offered_by, acc("owner.near"));
    assert!(get_logs().iter().any(|l| l.contains("\"event\":\"lease_offered\"")));

    // Accepted later: the clock starts at accept time, not offer time.
    testing_env!(ctx_at("lessee.near", 3 * NEAR, 5_000).build());
    c.accept_lease();
    assert!(get_logs().iter().any(|l| l.contains("\"event\":\"lease_started\"")));
    assert!(c.get_lease_offer().is_none());
    let l = c.get_lease().unwrap();
    assert_eq!(l.lessee, acc("lessee.near"));
    assert_eq!((l.start_ts, l.end_ts), (5_000, 5_000 + 86_400));
    assert_eq!(l.deposit, U128(3 * NEAR));
    assert_eq!(c.held_deposits(), 3 * NEAR);
    assert_eq!(c.get_invariant_parts().held_lessee_deposits, U128(3 * NEAR));
}

#[test]
fn old_attack_is_impossible_lessee_gets_back_only_what_it_paid() {
    let mut c = setup();
    // The owner can no longer attach NEAR as a deposit: exactly 1 yocto only.
    for attached in [0, 2, NEAR, 5 * NEAR] {
        fails(
            || {
                testing_env!(ctx("owner.near", attached).build());
                c.start_lease(acc("lessee.near"), U128(NEAR), 86_400, U128(NEAR));
            },
            "Requires attached deposit of exactly 1 yoctoNEAR",
        );
    }
    assert!(c.lease_offer.is_none() && c.lease.is_none());

    lease(&mut c, 2 * NEAR);
    testing_env!(ctx("lessee.near", 1).build());
    c.off_hire();
    assert!(c.lease.is_none());
    assert_eq!(c.get_pending(acc("lessee.near")).0, 2 * NEAR);
    assert_eq!(c.get_pending(acc("owner.near")).0, 0);
    assert_eq!(c.pending_total, 2 * NEAR);
    testing_env!(ctx("lessee.near", 0).build());
    let _ = c.withdraw();
    assert_eq!(c.pending_total, 0);
    assert_eq!(c.get_pending(acc("lessee.near")).0, 0);
    assert!(get_logs().iter().any(|l| l.contains("\"event\":\"withdrawn\"") && l.contains(&format!("\"amount\":\"{}\"", 2 * NEAR))));
    // Nothing more for the lessee to take.
    fails(
        || {
            testing_env!(ctx("lessee.near", 0).build());
            let _ = c.withdraw();
        },
        err::NOTHING_TO_WITHDRAW,
    );
}

#[test]
fn only_named_lessee_can_accept() {
    let mut c = setup();
    offer(&mut c, NEAR);
    for who in ["owner.near", "outsider.near", "inspector.near"] {
        fails(
            || {
                testing_env!(ctx(who, NEAR).build());
                c.accept_lease();
            },
            err::NOT_LESSEE,
        );
    }
    assert!(c.lease.is_none() && c.lease_offer.is_some());
    testing_env!(ctx("lessee.near", NEAR).build());
    c.accept_lease();
    assert!(c.lease.is_some());
}

#[test]
fn wrong_deposit_is_rejected() {
    let mut c = setup();
    offer(&mut c, NEAR);
    for attached in [NEAR - 1, NEAR + 1, 0] {
        fails(
            || {
                testing_env!(ctx("lessee.near", attached).build());
                c.accept_lease();
            },
            err::WRONG_DEPOSIT,
        );
    }
    assert!(c.lease.is_none() && c.lease_offer.is_some());
    testing_env!(ctx("lessee.near", NEAR).build());
    c.accept_lease();
    assert_eq!(c.get_lease().unwrap().deposit, U128(NEAR));
}

#[test]
fn start_lease_checks() {
    let mut c = setup();
    let start = |c: &mut BoxContract, lessee: &str, rate: u128, dur: u64, dep: u128| {
        testing_env!(ctx("owner.near", 1).build());
        c.start_lease(acc(lessee), U128(rate), dur, U128(dep));
    };
    fails(|| start(&mut c, "lessee.near", NEAR, 86_400, 0), err::ZERO_DEPOSIT);
    fails(|| start(&mut c, "lessee.near", 0, 86_400, 1), err::ZERO_RATE);
    fails(|| start(&mut c, "lessee.near", NEAR, 0, 1), err::ZERO_DURATION);
    fails(|| start(&mut c, "owner.near", NEAR, 86_400, 1), err::LESSEE_IS_OWNER);
    fails(
        || {
            testing_env!(ctx("outsider.near", 1).build());
            c.start_lease(acc("lessee.near"), U128(NEAR), 86_400, U128(1));
        },
        err::NOT_OWNER,
    );
    fails(
        || {
            testing_env!(ctx_at("owner.near", 1, 20_000).build());
            c.start_lease(acc("lessee.near"), U128(NEAR), 86_400, U128(1));
        },
        err::INSPECTION_LAPSED,
    );
    assert!(c.lease_offer.is_none());
    // Minimum deposit of 1 yocto is accepted.
    start(&mut c, "lessee.near", NEAR, 86_400, 1);
    assert_eq!(c.get_lease_offer().unwrap().deposit, U128(1));
}

#[test]
fn accept_refused_after_inspection_lapses() {
    let mut c = setup();
    offer(&mut c, NEAR);
    fails(
        || {
            testing_env!(ctx_at("lessee.near", NEAR, 20_000).build());
            c.accept_lease();
        },
        err::INSPECTION_LAPSED,
    );
    assert!(c.lease.is_none() && c.lease_offer.is_some());
}

#[test]
fn cancel_lease_offer_rules() {
    let mut c = setup();
    // No offer -> refused.
    fails(
        || {
            testing_env!(ctx("owner.near", 1).build());
            c.cancel_lease_offer();
        },
        err::NO_OFFER,
    );
    offer(&mut c, NEAR);
    // Non-owner (including the lessee) -> refused.
    for who in ["lessee.near", "outsider.near"] {
        fails(
            || {
                testing_env!(ctx(who, 1).build());
                c.cancel_lease_offer();
            },
            err::NOT_OWNER,
        );
    }
    // Owner without the 1 yocto -> refused.
    fails(
        || {
            testing_env!(ctx("owner.near", 0).build());
            c.cancel_lease_offer();
        },
        "Requires attached deposit of exactly 1 yoctoNEAR",
    );
    assert!(c.lease_offer.is_some());
    testing_env!(ctx("owner.near", 1).build());
    c.cancel_lease_offer();
    assert!(c.get_lease_offer().is_none());
    assert!(get_logs().iter().any(|l| l.contains("\"event\":\"lease_offer_cancelled\"")));
    // Accept after cancel -> refused.
    fails(
        || {
            testing_env!(ctx("lessee.near", NEAR).build());
            c.accept_lease();
        },
        err::NO_OFFER,
    );
    // Cancel after accept -> refused (nothing pending).
    lease(&mut c, NEAR);
    fails(
        || {
            testing_env!(ctx("owner.near", 1).build());
            c.cancel_lease_offer();
        },
        err::NO_OFFER,
    );
    assert!(c.lease.is_some());
}

#[test]
fn accept_and_cancel_refused_under_sale_lock() {
    let mut c = setup();
    offer(&mut c, NEAR);
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(5 * NEAR));
    testing_env!(ctx("buyer.near", 5 * NEAR).build());
    let _ = c.buy();
    assert!(c.lock.is_some());
    fails(
        || {
            testing_env!(ctx("lessee.near", NEAR).build());
            c.accept_lease();
        },
        err::LOCKED,
    );
    fails(
        || {
            testing_env!(ctx("owner.near", 1).build());
            c.cancel_lease_offer();
        },
        err::LOCKED,
    );
    assert!(c.lease.is_none() && c.lease_offer.is_some());
}

#[test]
fn second_offer_and_accept_refused_when_pending_or_active() {
    let mut c = setup();
    offer(&mut c, NEAR);
    // Another offer while one is pending.
    fails(|| offer(&mut c, 2 * NEAR), err::OFFER_PENDING);
    assert_eq!(c.get_lease_offer().unwrap().deposit, U128(NEAR));
    testing_env!(ctx("lessee.near", NEAR).build());
    c.accept_lease();
    // A second accept finds no offer.
    fails(
        || {
            testing_env!(ctx("lessee.near", NEAR).build());
            c.accept_lease();
        },
        err::NO_OFFER,
    );
    // A new offer while a lease is active.
    fails(|| offer(&mut c, NEAR), err::LEASE_ACTIVE);
    // Defensive: an offer present next to an active lease cannot be accepted.
    let active = c.lease.clone();
    c.lease_offer = Some(LeaseOffer {
        lessee: acc("lessee.near"),
        rate_per_day: U128(NEAR),
        duration_secs: 86_400,
        deposit: U128(NEAR),
        offered_by: acc("owner.near"),
    });
    fails(
        || {
            testing_env!(ctx("lessee.near", NEAR).build());
            c.accept_lease();
        },
        err::LEASE_ACTIVE,
    );
    assert_eq!(c.lease, active);
}

#[test]
fn stale_offer_after_ownership_change_is_refused() {
    let mut c = setup();
    offer(&mut c, NEAR);
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(5 * NEAR));
    testing_env!(ctx("buyer.near", 5 * NEAR).build());
    let _ = c.buy();
    callback_ctx(PromiseResult::Successful(vec![]));
    assert_eq!(c.on_nft_transfer(1), "finalized");
    assert_eq!(c.owner, acc("buyer.near"));
    fails(
        || {
            testing_env!(ctx("lessee.near", NEAR).build());
            c.accept_lease();
        },
        err::STALE_OFFER,
    );
    assert!(c.lease.is_none());
    // The old owner can no longer cancel; the new owner can, then offer anew.
    fails(
        || {
            testing_env!(ctx("owner.near", 1).build());
            c.cancel_lease_offer();
        },
        err::NOT_OWNER,
    );
    testing_env!(ctx("buyer.near", 1).build());
    c.cancel_lease_offer();
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400, U128(NEAR));
    assert_eq!(c.get_lease_offer().unwrap().offered_by, acc("buyer.near"));
    testing_env!(ctx("lessee.near", NEAR).build());
    c.accept_lease();
    assert_eq!(c.get_lease().unwrap().deposit, U128(NEAR));
}

#[test]
fn accept_refused_when_lessee_became_owner() {
    // The lessee buys the box while its own offer is pending: the offer is
    // stale, and the owner cannot lease to itself.
    let mut c = setup();
    offer(&mut c, NEAR);
    testing_env!(ctx("owner.near", 1).build());
    c.list(U128(5 * NEAR));
    testing_env!(ctx("lessee.near", 5 * NEAR).build());
    let _ = c.buy();
    callback_ctx(PromiseResult::Successful(vec![]));
    assert_eq!(c.on_nft_transfer(1), "finalized");
    fails(
        || {
            testing_env!(ctx("lessee.near", NEAR).build());
            c.accept_lease();
        },
        err::STALE_OFFER,
    );
}
