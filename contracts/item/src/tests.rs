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
    testing_env!(ctx("owner.near", 1).build());
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400);
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
    testing_env!(ctx("owner.near", 1).build());
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400);
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
    testing_env!(ctx("owner.near", 1).build());
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400);
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
    testing_env!(ctx("owner.near", 1).build());
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400);
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
    testing_env!(ctx("owner.near", 1).build());
    c.start_lease(acc("lessee.near"), U128(NEAR), 86_400);
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
