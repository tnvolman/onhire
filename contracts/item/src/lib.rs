//! Onhire box (item) contract.
//!
//! One shipping container = one NFT = one keyless NEAR account. The box holds
//! the lease, the repair reserve, the inspection record, the listing, the sale
//! lock, and every pull balance, together with the native NEAR that backs them.
//!
//! Spec: docs/ONHIRE-SPEC-v1.1.md (plus the 1% protocol fee, docs/FEE-PROOF.md).
//! All rights reserved; see LICENSE.
//!
//! There is deliberately NO method to add a key, upgrade/deploy code, delete
//! the account, or withdraw the reserve.

use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, ext_contract, near, require, AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault,
    Promise, PromiseError,
};

// ---------------------------------------------------------------------------
// Constants (spec 2.1)
// ---------------------------------------------------------------------------

/// 15% of each hire payment goes to the repair reserve.
pub const RESERVE_BPS: u16 = 1500;
/// `rescue_sale` may undo a sale only at `lock.started_at_block + 32` or later.
/// Never lower this below 32. Raise only if measured, and document the raise.
pub const SALE_UNDO_DELAY_BLOCKS: u64 = 32;
/// `start_lease` fails when `reserve < MIN_RESERVE`.
pub const MIN_RESERVE: u128 = 0;
pub const BPS_DENOMINATOR: u128 = 10_000;
/// 1% protocol fee on every hire payment and every sale. It is taken from the
/// owner's (or seller's) proceeds only, never from the repair reserve, and is
/// sent straight to the box's `fee_account`, which the collection sets once
/// at init and which is immutable afterwards.
pub const FEE_BPS: u16 = 100;

/// Headroom (in bytes of storage) every user-facing state change must leave
/// on top of the rule-14 invariant, so that storage growth from the change
/// itself (written after the method body returns) is always covered.
pub const STORAGE_HEADROOM_BYTES: u128 = 2_000;

const GAS_FOR_NFT_TRANSFER: Gas = Gas::from_tgas(15);
const GAS_FOR_ON_NFT_TRANSFER: Gas = Gas::from_tgas(15);
const GAS_FOR_OWNER_OF: Gas = Gas::from_tgas(10);
const GAS_FOR_ON_RESCUE: Gas = Gas::from_tgas(15);
/// `buy` must carry enough gas for itself, the transfer and the callback.
const MIN_GAS_FOR_BUY: Gas = Gas::from_tgas(60);
const MIN_GAS_FOR_RESCUE: Gas = Gas::from_tgas(40);

const NANOS_PER_SEC: u64 = 1_000_000_000;

// ---------------------------------------------------------------------------
// Errors (stable strings, asserted by tests)
// ---------------------------------------------------------------------------

pub mod err {
    pub const NOT_OWNER: &str = "ERR_NOT_OWNER";
    pub const LOCKED: &str = "ERR_SALE_LOCKED";
    pub const ZERO_PRICE: &str = "ERR_ZERO_PRICE";
    pub const NOT_LISTED: &str = "ERR_NOT_LISTED";
    pub const WRONG_AMOUNT: &str = "ERR_WRONG_ATTACHED_AMOUNT";
    pub const BUYER_IS_OWNER: &str = "ERR_BUYER_IS_OWNER";
    pub const BUYER_IS_INSPECTOR: &str = "ERR_BUYER_IS_INSPECTOR";
    pub const NOT_ENOUGH_GAS: &str = "ERR_NOT_ENOUGH_GAS";
    pub const STALE_SALE: &str = "ERR_STALE_SALE_ID";
    pub const UNDO_TOO_EARLY: &str = "ERR_UNDO_DELAY_NOT_ELAPSED";
    pub const UNEXPECTED_OWNER: &str = "ERR_COLLECTION_OWNER_UNEXPECTED";
    pub const QUERY_FAILED: &str = "ERR_COLLECTION_QUERY_FAILED";
    pub const NOTHING_TO_WITHDRAW: &str = "ERR_NOTHING_TO_WITHDRAW";
    pub const LEASE_ACTIVE: &str = "ERR_LEASE_ACTIVE";
    pub const NO_LEASE: &str = "ERR_NO_LEASE";
    pub const INSPECTION_LAPSED: &str = "ERR_INSPECTION_LAPSED";
    pub const RESERVE_BELOW_MIN: &str = "ERR_RESERVE_BELOW_MIN";
    pub const ZERO_RATE: &str = "ERR_ZERO_RATE";
    pub const ZERO_DURATION: &str = "ERR_ZERO_DURATION";
    pub const ZERO_DEPOSIT: &str = "ERR_ZERO_DEPOSIT";
    pub const WRONG_DEPOSIT: &str = "ERR_WRONG_DEPOSIT";
    pub const OFFER_PENDING: &str = "ERR_LEASE_OFFER_PENDING";
    pub const NO_OFFER: &str = "ERR_NO_LEASE_OFFER";
    pub const STALE_OFFER: &str = "ERR_STALE_OFFER";
    pub const LESSEE_IS_OWNER: &str = "ERR_LESSEE_IS_OWNER";
    pub const NOT_LESSEE: &str = "ERR_NOT_LESSEE";
    pub const ZERO_HIRE: &str = "ERR_ZERO_HIRE";
    pub const OFF_HIRE_DENIED: &str = "ERR_OFF_HIRE_NOT_ALLOWED";
    pub const NOT_INSPECTOR: &str = "ERR_NOT_INSPECTOR";
    pub const OWNER_CANNOT_ATTEST: &str = "ERR_OWNER_CANNOT_ATTEST";
    pub const VALID_UNTIL_PAST: &str = "ERR_VALID_UNTIL_NOT_IN_FUTURE";
    pub const INSOLVENT: &str = "ERR_INVARIANT_WOULD_BREAK";
    pub const BAD_INIT: &str = "ERR_BAD_INIT";
    pub const FEE_EXCEEDS_OWNER_SHARE: &str = "ERR_FEE_EXCEEDS_OWNER_SHARE";
}

// ---------------------------------------------------------------------------
// State (spec 4)
// ---------------------------------------------------------------------------

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub price: U128,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct SaleLock {
    pub sale_id: u64,
    pub seller: AccountId,
    pub buyer: AccountId,
    pub price: U128,
    pub started_at_block: u64,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct Lease {
    pub lessee: AccountId,
    /// yoctoNEAR per day, paid explicitly via `pay_hire`.
    pub rate_per_day: U128,
    /// Seconds since the Unix epoch (block timestamp).
    pub start_ts: u64,
    pub end_ts: u64,
    pub deposit: U128,
}

/// A lease the owner has offered and the named lessee has not yet accepted.
/// It holds no funds and does not start the clock.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct LeaseOffer {
    pub lessee: AccountId,
    /// yoctoNEAR per day, paid explicitly via `pay_hire`.
    pub rate_per_day: U128,
    /// Lease length, counted from acceptance.
    pub duration_secs: u64,
    /// Exact NEAR the lessee must attach to `accept_lease`.
    pub deposit: U128,
    /// Box owner who made the offer. If ownership changes, the offer is stale.
    pub offered_by: AccountId,
}

#[near]
#[derive(BorshStorageKey)]
pub enum StorageKey {
    Pending,
    LockedPayments,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct BoxContract {
    pub collection: AccountId,
    pub token_id: String,
    /// Source of truth for ownership, except during `rescue_sale`.
    pub owner: AccountId,
    /// Unpaid hire already belonging to the current owner.
    pub accrued_hire: u128,
    pub reserve: u128,
    pub pending_withdrawals: LookupMap<AccountId, u128>,
    /// Running sum of `pending_withdrawals`.
    pub pending_total: u128,
    /// sale_id -> attached purchase NEAR.
    pub locked_buyer_payments: LookupMap<u64, u128>,
    /// Running sum of `locked_buyer_payments`.
    pub locked_total: u128,
    pub listing: Option<Listing>,
    pub lock: Option<SaleLock>,
    pub next_sale_id: u64,
    pub lease: Option<Lease>,
    /// Seconds since the Unix epoch.
    pub inspection_valid_until: u64,
    /// Immutable after init.
    pub inspector: AccountId,
    pub min_reserve: u128,
    pub reserve_bps: u16,
    /// Immutable after init. Receives the protocol fee.
    pub fee_account: AccountId,
    /// Always `FEE_BPS`; kept in state next to `reserve_bps`.
    pub fee_bps: u16,
    /// Running total of protocol fees sent to `fee_account`.
    pub fees_paid: u128,
    /// Pending (unaccepted) lease offer, if any.
    pub lease_offer: Option<LeaseOffer>,
}

/// Split of one hire payment (spec 2.1 + protocol fee).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HireSplit {
    pub to_reserve: u128,
    pub fee: u128,
    pub to_owner: u128,
}

/// Split of one sale price: the reserve is not touched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SaleSplit {
    pub fee: u128,
    pub to_seller: u128,
}

/// `amount * bps / 10_000`, rounded down.
pub fn bps_of(amount: u128, bps: u16) -> u128 {
    amount * bps as u128 / BPS_DENOMINATOR
}

/// Reserve first (unchanged 15%), then the 1% fee (of the whole payment) out
/// of the owner's share. `None` if the owner's share cannot cover the fee;
/// callers revert the whole call.
pub fn split_hire(amount: u128, reserve_bps: u16, fee_bps: u16) -> Option<HireSplit> {
    let to_reserve = bps_of(amount, reserve_bps);
    let owner_share = amount.checked_sub(to_reserve)?;
    let fee = bps_of(amount, fee_bps);
    let to_owner = owner_share.checked_sub(fee)?;
    Some(HireSplit { to_reserve, fee, to_owner })
}

/// The 1% fee comes out of the seller's proceeds. `None` if it cannot.
pub fn split_sale(price: u128, fee_bps: u16) -> Option<SaleSplit> {
    let fee = bps_of(price, fee_bps);
    let to_seller = price.checked_sub(fee)?;
    Some(SaleSplit { fee, to_seller })
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct InvariantParts {
    pub balance: U128,
    pub storage_staking: U128,
    pub reserve: U128,
    pub pending_sum: U128,
    pub held_lessee_deposits: U128,
    pub locked_buyer_payments: U128,
    pub holds: bool,
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct InspectionView {
    pub inspector: AccountId,
    pub valid_until: u64,
    pub valid_now: bool,
}

#[ext_contract(ext_collection)]
#[allow(dead_code)]
trait Collection {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u32>,
        memo: Option<String>,
    );
    fn owner_of(&self, token_id: String) -> Option<AccountId>;
}

fn now_secs() -> u64 {
    env::block_timestamp() / NANOS_PER_SEC
}

fn emit(event: &str, data: near_sdk::serde_json::Value) {
    let ev = near_sdk::serde_json::json!({
        "standard": "onhire",
        "version": "1.0.0",
        "event": event,
        "data": [data],
    });
    env::log_str(&format!("EVENT_JSON:{}", ev));
}

#[near]
impl BoxContract {
    /// Called by the collection in the same batch that creates this account.
    #[init]
    pub fn new(
        collection: AccountId,
        token_id: String,
        owner: AccountId,
        inspector: AccountId,
        inspection_valid_until: u64,
        fee_account: AccountId,
    ) -> Self {
        require!(env::predecessor_account_id() == collection, err::BAD_INIT);
        let expected = format!("box-{}.{}", token_id, collection);
        require!(env::current_account_id().as_str() == expected, err::BAD_INIT);
        require!(inspector != owner, err::BAD_INIT);
        Self {
            collection,
            token_id,
            owner,
            accrued_hire: 0,
            reserve: 0,
            pending_withdrawals: LookupMap::new(StorageKey::Pending),
            pending_total: 0,
            locked_buyer_payments: LookupMap::new(StorageKey::LockedPayments),
            locked_total: 0,
            listing: None,
            lock: None,
            next_sale_id: 1,
            lease: None,
            inspection_valid_until,
            inspector,
            min_reserve: MIN_RESERVE,
            reserve_bps: RESERVE_BPS,
            fee_account,
            fee_bps: FEE_BPS,
            fees_paid: 0,
            lease_offer: None,
        }
    }

    // ----------------------------------------------------------------- owner

    /// 5.1 — list (or re-list) for `price` yoctoNEAR. 1 yocto confirmation.
    #[payable]
    pub fn list(&mut self, price: U128) {
        near_sdk::assert_one_yocto();
        self.assert_owner();
        self.assert_unlocked();
        require!(price.0 > 0, err::ZERO_PRICE);
        self.listing = Some(Listing { price });
        emit("listed", near_sdk::serde_json::json!({ "price": price }));
        self.assert_solvent();
    }

    /// 5.1 — cancel an unlocked listing. 1 yocto confirmation.
    #[payable]
    pub fn cancel_list(&mut self) {
        near_sdk::assert_one_yocto();
        self.assert_owner();
        self.assert_unlocked();
        require!(self.listing.is_some(), err::NOT_LISTED);
        self.listing = None;
        emit("listing_cancelled", near_sdk::serde_json::json!({}));
        self.assert_solvent();
    }

    // ----------------------------------------------------------------- buyer

    /// 5.2 / 3.3 — buy with exactly the listed price attached.
    #[payable]
    pub fn buy(&mut self) -> Promise {
        self.assert_unlocked();
        let listing = self.listing.clone().unwrap_or_else(|| env::panic_str(err::NOT_LISTED));
        let attached = env::attached_deposit().as_yoctonear();
        require!(attached == listing.price.0, err::WRONG_AMOUNT);
        let buyer = env::predecessor_account_id();
        require!(buyer != self.owner, err::BUYER_IS_OWNER);
        require!(buyer != self.inspector, err::BUYER_IS_INSPECTOR);
        require!(env::prepaid_gas() >= MIN_GAS_FOR_BUY, err::NOT_ENOUGH_GAS);
        // Fail closed before anything is locked: the seller's proceeds must
        // cover the protocol fee. `finalize` applies the same split.
        self.sale_split(attached);

        let sale_id = self.next_sale_id;
        self.next_sale_id += 1;
        self.locked_buyer_payments.insert(sale_id, attached);
        self.locked_total += attached;
        let lock = SaleLock {
            sale_id,
            seller: self.owner.clone(),
            buyer: buyer.clone(),
            price: listing.price,
            started_at_block: env::block_height(),
        };
        emit("sale_locked", near_sdk::serde_json::json!(lock));
        self.lock = Some(lock);
        self.assert_solvent();

        ext_collection::ext(self.collection.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(GAS_FOR_NFT_TRANSFER)
            .nft_transfer(
                buyer,
                self.token_id.clone(),
                None,
                Some(format!("onhire sale {}", sale_id)),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_ON_NFT_TRANSFER)
                    .on_nft_transfer(sale_id),
            )
    }

    /// 3.3 step 4 — callback for the collection transfer started by `buy`.
    /// Stale `sale_id` (or no lock) is a no-op.
    #[private]
    pub fn on_nft_transfer(&mut self, sale_id: u64) -> String {
        if !self.is_current_lock(sale_id) {
            env::log_str(&format!("on_nft_transfer: stale sale_id {} ignored", sale_id));
            return "noop".to_string();
        }
        let success = env::promise_results_count() == 1
            && !matches!(env::promise_result_checked(0, 256), Err(PromiseError::Failed));
        if success {
            self.finalize("callback");
            "finalized".to_string()
        } else {
            // A failed transfer does not wait for the undo delay.
            self.undo("callback");
            "undone".to_string()
        }
    }

    // --------------------------------------------------------------- anyone

    /// 3.3 step 5 — resolve a stuck lock by asking the collection who owns
    /// the token. This is the only ordinary path that reads the collection.
    pub fn rescue_sale(&mut self, sale_id: u64) -> Promise {
        require!(self.is_current_lock(sale_id), err::STALE_SALE);
        require!(env::prepaid_gas() >= MIN_GAS_FOR_RESCUE, err::NOT_ENOUGH_GAS);
        ext_collection::ext(self.collection.clone())
            .with_static_gas(GAS_FOR_OWNER_OF)
            .owner_of(self.token_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_ON_RESCUE)
                    .on_rescue_owner(sale_id),
            )
    }

    #[private]
    pub fn on_rescue_owner(
        &mut self,
        sale_id: u64,
        #[callback_result] owner: Result<Option<AccountId>, PromiseError>,
    ) -> String {
        if !self.is_current_lock(sale_id) {
            env::log_str(&format!("on_rescue_owner: stale sale_id {} ignored", sale_id));
            return "noop".to_string();
        }
        let owner = match owner {
            Ok(Some(o)) => o,
            Ok(None) => env::panic_str(err::UNEXPECTED_OWNER),
            Err(_) => env::panic_str(err::QUERY_FAILED),
        };
        let lock = self.lock.clone().unwrap();
        if owner == lock.buyer {
            self.finalize("rescue");
            "finalized".to_string()
        } else if owner == lock.seller {
            require!(
                env::block_height() >= lock.started_at_block + SALE_UNDO_DELAY_BLOCKS,
                err::UNDO_TOO_EARLY
            );
            self.undo("rescue");
            "undone".to_string()
        } else {
            env::panic_str(err::UNEXPECTED_OWNER)
        }
    }

    /// 5.3 — pay out the caller's pull balance. If the caller is the current
    /// owner and no sale is locked, their accrued hire is swept into the
    /// payout first. Never touches the reserve, locked buyer payments, or a
    /// held lessee deposit.
    pub fn withdraw(&mut self) -> Promise {
        let who = env::predecessor_account_id();
        if who == self.owner && self.lock.is_none() && self.accrued_hire > 0 {
            let hire = self.accrued_hire;
            self.accrued_hire = 0;
            self.credit(&who, hire);
        }
        let amount = self.pending_withdrawals.remove(&who).unwrap_or(0);
        require!(amount > 0, err::NOTHING_TO_WITHDRAW);
        self.pending_total -= amount;
        emit(
            "withdrawn",
            near_sdk::serde_json::json!({ "account": who, "amount": U128(amount) }),
        );
        self.assert_solvent_after_payout(amount);
        Promise::new(who).transfer(NearToken::from_yoctonear(amount))
    }

    // ---------------------------------------------------------------- lease

    /// 5.4 — step 1 of 2: the owner offers a lease to `lessee`. 1 yocto
    /// confirmation only; the owner posts no deposit. The offer records the
    /// terms and the exact `deposit` the lessee must attach to
    /// `accept_lease`. It holds no funds and does not start the clock.
    /// Refused while a sale is locked, a lease is active or another offer is
    /// pending.
    #[payable]
    pub fn start_lease(
        &mut self,
        lessee: AccountId,
        rate_per_day: U128,
        duration_secs: u64,
        deposit: U128,
    ) {
        near_sdk::assert_one_yocto();
        self.assert_owner();
        self.assert_unlocked();
        require!(self.lease.is_none(), err::LEASE_ACTIVE);
        require!(self.lease_offer.is_none(), err::OFFER_PENDING);
        require!(now_secs() <= self.inspection_valid_until, err::INSPECTION_LAPSED);
        require!(self.reserve >= self.min_reserve, err::RESERVE_BELOW_MIN);
        require!(rate_per_day.0 > 0, err::ZERO_RATE);
        require!(duration_secs > 0, err::ZERO_DURATION);
        require!(deposit.0 >= 1, err::ZERO_DEPOSIT);
        require!(lessee != self.owner, err::LESSEE_IS_OWNER);
        let offer = LeaseOffer {
            lessee,
            rate_per_day,
            duration_secs,
            deposit,
            offered_by: self.owner.clone(),
        };
        emit("lease_offered", near_sdk::serde_json::json!(offer));
        self.lease_offer = Some(offer);
        self.assert_solvent();
    }

    /// 5.4 — step 2 of 2: the named lessee accepts the pending offer by
    /// attaching exactly the offered deposit. The lease starts now and runs
    /// `duration_secs`; the attached NEAR is held as `lease.deposit` and is
    /// returned to the lessee by `off_hire`. Refused while a sale is locked,
    /// if a lease is active, if the box owner changed since the offer
    /// (stale), if the inspection has lapsed or the reserve is below the
    /// minimum. Any refusal panics, so the attachment is refunded.
    #[payable]
    pub fn accept_lease(&mut self) {
        self.assert_unlocked();
        let offer = self.lease_offer.clone().unwrap_or_else(|| env::panic_str(err::NO_OFFER));
        let who = env::predecessor_account_id();
        require!(who == offer.lessee, err::NOT_LESSEE);
        require!(self.lease.is_none(), err::LEASE_ACTIVE);
        require!(offer.offered_by == self.owner, err::STALE_OFFER);
        require!(who != self.owner, err::LESSEE_IS_OWNER);
        let now = now_secs();
        require!(now <= self.inspection_valid_until, err::INSPECTION_LAPSED);
        require!(self.reserve >= self.min_reserve, err::RESERVE_BELOW_MIN);
        let attached = env::attached_deposit().as_yoctonear();
        require!(attached == offer.deposit.0, err::WRONG_DEPOSIT);
        self.lease_offer = None;
        let lease = Lease {
            lessee: who,
            rate_per_day: offer.rate_per_day,
            start_ts: now,
            end_ts: now.saturating_add(offer.duration_secs),
            deposit: U128(attached),
        };
        emit("lease_started", near_sdk::serde_json::json!(lease));
        self.lease = Some(lease);
        self.assert_solvent();
    }

    /// 5.4 — the owner withdraws a pending, unaccepted offer. 1 yocto
    /// confirmation. Like the other owner methods it is refused while a sale
    /// is locked (accept is refused then too, so nothing can change under
    /// the lock). A new owner may cancel a stale offer left by the seller.
    #[payable]
    pub fn cancel_lease_offer(&mut self) {
        near_sdk::assert_one_yocto();
        self.assert_owner();
        self.assert_unlocked();
        let offer = self.lease_offer.take().unwrap_or_else(|| env::panic_str(err::NO_OFFER));
        emit("lease_offer_cancelled", near_sdk::serde_json::json!(offer));
        self.assert_solvent();
    }

    /// 5.4 — lessee pays hire. Rejected while a sale is locked.
    #[payable]
    pub fn pay_hire(&mut self) {
        self.assert_unlocked();
        let lease = self.lease.as_ref().unwrap_or_else(|| env::panic_str(err::NO_LEASE));
        require!(env::predecessor_account_id() == lease.lessee, err::NOT_LESSEE);
        let attached = env::attached_deposit().as_yoctonear();
        require!(attached > 0, err::ZERO_HIRE);
        // Whole call reverts if the owner's share cannot cover the fee.
        let HireSplit { to_reserve, fee, to_owner } =
            split_hire(attached, self.reserve_bps, self.fee_bps)
                .unwrap_or_else(|| env::panic_str(err::FEE_EXCEEDS_OWNER_SHARE));
        self.reserve += to_reserve;
        self.accrued_hire += to_owner;
        emit(
            "hire_paid",
            near_sdk::serde_json::json!({
                "amount": U128(attached),
                "to_reserve": U128(to_reserve),
                "fee": U128(fee),
                "to_owner": U128(to_owner),
                "owner": self.owner,
            }),
        );
        // The fee leaves the box in this call, so check rule 14 with it out.
        self.assert_solvent_after_payout(fee);
        self.pay_fee(fee, "hire");
    }

    /// 5.4 — end the lease: the lessee at any time, or the owner after the
    /// lease end. The full deposit (which the lessee posted in
    /// `accept_lease`) is credited back to the lessee as a pull balance (no
    /// damage model in this version). 1 yocto confirmation.
    #[payable]
    pub fn off_hire(&mut self) {
        near_sdk::assert_one_yocto();
        self.assert_unlocked();
        let lease = self.lease.clone().unwrap_or_else(|| env::panic_str(err::NO_LEASE));
        let who = env::predecessor_account_id();
        let allowed = who == lease.lessee || (who == self.owner && now_secs() > lease.end_ts);
        require!(allowed, err::OFF_HIRE_DENIED);
        self.lease = None;
        self.credit(&lease.lessee, lease.deposit.0);
        emit("off_hire", near_sdk::serde_json::json!({ "lessee": lease.lessee, "deposit_returned": lease.deposit }));
        self.assert_solvent();
    }

    // ------------------------------------------------------------ inspector

    /// 5.5 — only the inspector (who can never be the owner) attests.
    pub fn attest_inspection(&mut self, valid_until: u64) {
        let who = env::predecessor_account_id();
        require!(who == self.inspector, err::NOT_INSPECTOR);
        require!(who != self.owner, err::OWNER_CANNOT_ATTEST);
        require!(valid_until > now_secs(), err::VALID_UNTIL_PAST);
        self.inspection_valid_until = valid_until;
        emit("inspection_attested", near_sdk::serde_json::json!({ "valid_until": valid_until }));
        self.assert_solvent();
    }

    // ---------------------------------------------------------------- views

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }
    pub fn get_collection(&self) -> AccountId {
        self.collection.clone()
    }
    pub fn get_token_id(&self) -> String {
        self.token_id.clone()
    }
    pub fn get_listing(&self) -> Option<Listing> {
        self.listing.clone()
    }
    pub fn get_lock(&self) -> Option<SaleLock> {
        self.lock.clone()
    }
    pub fn get_next_sale_id(&self) -> u64 {
        self.next_sale_id
    }
    pub fn get_lease(&self) -> Option<Lease> {
        self.lease.clone()
    }
    /// The pending, unaccepted lease offer, if any.
    pub fn get_lease_offer(&self) -> Option<LeaseOffer> {
        self.lease_offer.clone()
    }
    pub fn get_inspection(&self) -> InspectionView {
        InspectionView {
            inspector: self.inspector.clone(),
            valid_until: self.inspection_valid_until,
            valid_now: now_secs() <= self.inspection_valid_until,
        }
    }
    pub fn get_reserve(&self) -> U128 {
        U128(self.reserve)
    }
    pub fn get_accrued_hire(&self) -> U128 {
        U128(self.accrued_hire)
    }
    pub fn get_pending(&self, account: AccountId) -> U128 {
        U128(self.pending_withdrawals.get(&account).copied().unwrap_or(0))
    }
    pub fn get_locked_payment(&self, sale_id: u64) -> U128 {
        U128(self.locked_buyer_payments.get(&sale_id).copied().unwrap_or(0))
    }
    pub fn get_constants(&self) -> near_sdk::serde_json::Value {
        near_sdk::serde_json::json!({
            "RESERVE_BPS": self.reserve_bps,
            "SALE_UNDO_DELAY_BLOCKS": SALE_UNDO_DELAY_BLOCKS,
            "MIN_RESERVE": U128(self.min_reserve),
            "BPS_DENOMINATOR": U128(BPS_DENOMINATOR),
            "FEE_BPS": self.fee_bps,
            "FEE_ACCOUNT": self.fee_account,
        })
    }
    pub fn get_fee_account(&self) -> AccountId {
        self.fee_account.clone()
    }
    /// Total protocol fees this box has sent to `fee_account`.
    pub fn get_fees_paid(&self) -> U128 {
        U128(self.fees_paid)
    }
    pub fn get_invariant_parts(&self) -> InvariantParts {
        let balance = env::account_balance().as_yoctonear()
            + env::account_locked_balance().as_yoctonear();
        let storage = self.storage_staking();
        let deposits = self.held_deposits();
        let need = storage + self.reserve + self.pending_total + deposits + self.locked_total;
        InvariantParts {
            balance: U128(balance),
            storage_staking: U128(storage),
            reserve: U128(self.reserve),
            pending_sum: U128(self.pending_total),
            held_lessee_deposits: U128(deposits),
            locked_buyer_payments: U128(self.locked_total),
            holds: balance >= need,
        }
    }
}

// -------------------------------------------------------------------- internal

impl BoxContract {
    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner, err::NOT_OWNER);
    }

    fn assert_unlocked(&self) {
        require!(self.lock.is_none(), err::LOCKED);
    }

    fn is_current_lock(&self, sale_id: u64) -> bool {
        matches!(&self.lock, Some(l) if l.sale_id == sale_id)
    }

    fn credit(&mut self, who: &AccountId, amount: u128) {
        if amount == 0 {
            return;
        }
        let cur = self.pending_withdrawals.get(who).copied().unwrap_or(0);
        self.pending_withdrawals.insert(who.clone(), cur + amount);
        self.pending_total += amount;
    }

    fn take_locked_payment(&mut self, sale_id: u64) -> u128 {
        let amount = self.locked_buyer_payments.remove(&sale_id).unwrap_or(0);
        self.locked_total -= amount;
        amount
    }

    /// 3.4 — the buyer becomes owner; seller is credited price + accrued hire.
    fn finalize(&mut self, via: &str) {
        let lock = self.lock.take().expect("lock");
        let paid = self.take_locked_payment(lock.sale_id);
        // Validated in `buy` for this exact amount, so this cannot fail here.
        let SaleSplit { fee, to_seller } = self.sale_split(paid);
        let hire = self.accrued_hire;
        self.accrued_hire = 0;
        self.credit(&lock.seller, to_seller + hire);
        self.owner = lock.buyer.clone();
        self.listing = None;
        emit(
            "sale_finalized",
            near_sdk::serde_json::json!({
                "sale_id": lock.sale_id,
                "seller": lock.seller,
                "buyer": lock.buyer,
                "price": U128(paid),
                "fee": U128(fee),
                "seller_proceeds_credited": U128(to_seller),
                "seller_hire_credited": U128(hire),
                "via": via,
            }),
        );
        self.pay_fee(fee, "sale");
    }

    fn sale_split(&self, price: u128) -> SaleSplit {
        split_sale(price, self.fee_bps).unwrap_or_else(|| env::panic_str(err::FEE_EXCEEDS_OWNER_SHARE))
    }

    /// Send the protocol fee to `fee_account` now. It is not a liability of
    /// the box afterwards (it is not in reserve, pending, deposits or locked
    /// payments). If the transfer ever failed, the refund would land back in
    /// the box as unallocated surplus, which only strengthens rule 14.
    fn pay_fee(&mut self, fee: u128, source: &str) {
        if fee == 0 {
            return;
        }
        self.fees_paid += fee;
        emit(
            "fee_paid",
            near_sdk::serde_json::json!({
                "fee_account": self.fee_account,
                "amount": U128(fee),
                "source": source,
            }),
        );
        Promise::new(self.fee_account.clone())
            .transfer(NearToken::from_yoctonear(fee))
            .detach();
    }

    /// 3.5 — nothing moves except the buyer's payment back to a pull balance.
    fn undo(&mut self, via: &str) {
        let lock = self.lock.take().expect("lock");
        let paid = self.take_locked_payment(lock.sale_id);
        self.credit(&lock.buyer, paid);
        emit(
            "sale_undone",
            near_sdk::serde_json::json!({
                "sale_id": lock.sale_id,
                "seller": lock.seller,
                "buyer": lock.buyer,
                "refund": U128(paid),
                "via": via,
            }),
        );
    }

    fn held_deposits(&self) -> u128 {
        self.lease.as_ref().map(|l| l.deposit.0).unwrap_or(0)
    }

    fn storage_staking(&self) -> u128 {
        env::storage_usage() as u128 * env::storage_byte_cost().as_yoctonear()
    }

    fn liabilities(&self) -> u128 {
        self.reserve + self.pending_total + self.held_deposits() + self.locked_total
    }

    /// Rule 14 plus headroom for storage written after the method returns.
    fn assert_solvent(&self) {
        self.assert_solvent_after_payout(0);
    }

    fn assert_solvent_after_payout(&self, payout: u128) {
        let balance = env::account_balance().as_yoctonear()
            + env::account_locked_balance().as_yoctonear();
        let headroom = STORAGE_HEADROOM_BYTES * env::storage_byte_cost().as_yoctonear();
        let need = self.storage_staking() + self.liabilities() + headroom + payout;
        require!(balance >= need, err::INSOLVENT);
    }
}

#[cfg(test)]
mod tests;
