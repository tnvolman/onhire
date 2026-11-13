//! Shared harness for the seven sandbox tests (spec section 6 plus t7).
//!
//! After every step, `Env::check` asserts the rule-14 invariant both from the
//! box's own accounting and independently from the RPC account view, and
//! asserts the box account has zero access keys.

use anyhow::{anyhow, bail, Context, Result};
use near_workspaces::network::Sandbox;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::{AccountDetailsPatch, Gas, NearToken};
use near_workspaces::{Account, AccountId, Contract, Worker};
use serde_json::{json, Value};

pub const NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const STORAGE_BYTE_COST: u128 = 10_000_000_000_000_000_000; // 1e19 yocto
pub const SALE_UNDO_DELAY_BLOCKS: u64 = onhire_item::SALE_UNDO_DELAY_BLOCKS;
pub const TGAS: u64 = 1_000_000_000_000;
pub const RESERVE_BPS: u128 = onhire_item::RESERVE_BPS as u128;
pub const FEE_BPS: u128 = onhire_item::FEE_BPS as u128;

/// 15% of a hire payment to the repair reserve (unchanged by the fee).
pub fn reserve_cut(x: u128) -> u128 {
    x * RESERVE_BPS / 10_000
}
/// 1% protocol fee of a hire payment or a sale price.
pub fn fee_cut(x: u128) -> u128 {
    x * FEE_BPS / 10_000
}
/// What the owner accrues from a hire payment: 100 -> 84.
pub fn owner_cut(x: u128) -> u128 {
    x - reserve_cut(x) - fee_cut(x)
}
/// What the seller is credited from a sale price: 100 -> 99.
pub fn seller_net(price: u128) -> u128 {
    price - fee_cut(price)
}

pub fn yocto(n: u128) -> NearToken {
    NearToken::from_yoctonear(n)
}

pub fn wasm_path(name: &str) -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    root.join("target/near").join(name).join(format!("{name}.wasm"))
}

pub struct Env {
    pub worker: Worker<Sandbox>,
    pub collection: Contract,
    pub owner: Account,
    pub lessee: Account,
    pub buyer: Account,
    pub inspector: Account,
    pub outsider: Account,
    /// Receives the 1% protocol fee. Never signs, so its balance moves only by fees.
    pub fee: Account,
    pub test_name: String,
    pub step: std::cell::Cell<u32>,
}

pub fn code_mode_global() -> bool {
    std::env::var("ONHIRE_CODE_MODE").map(|m| m != "local").unwrap_or(true)
}

impl Env {
    pub async fn new(test_name: &str) -> Result<Self> {
        let worker = near_workspaces::sandbox().await?;
        let root = worker.root_account()?;
        let mk = |name: &'static str, amt: u128| {
            let root = root.clone();
            async move {
                root.create_subaccount(name)
                    .initial_balance(yocto(amt))
                    .transact()
                    .await?
                    .into_result()
                    .map_err(|e| anyhow!("create {name}: {e:?}"))
            }
        };
        let coll_acct = mk("onhire", 200 * NEAR).await?;
        let owner = mk("owner", 100 * NEAR).await?;
        let lessee = mk("lessee", 100 * NEAR).await?;
        let buyer = mk("buyer", 100 * NEAR).await?;
        let inspector = mk("inspector", 20 * NEAR).await?;
        let outsider = mk("outsider", 100 * NEAR).await?;
        let fee = mk("fee", NEAR).await?;

        let coll_wasm = std::fs::read(wasm_path("onhire_collection"))
            .context("build contracts first: scripts/test-sandbox.sh")?;
        let item_wasm = std::fs::read(wasm_path("onhire_item"))?;
        let collection = coll_acct.deploy(&coll_wasm).await?.into_result()?;
        collection
            .call("new")
            .args_json(json!({ "minter": collection.id(), "fee_account": fee.id() }))
            .transact()
            .await?
            .into_result()?;
        use near_sdk::base64::Engine;
        let code_b64 = near_sdk::base64::engine::general_purpose::STANDARD.encode(&item_wasm);
        let global = code_mode_global();
        let r = collection
            .call("set_item_code")
            .args_json(json!({ "code": code_b64, "global": global }))
            .gas(Gas::from_tgas(300))
            .transact()
            .await?;
        if !r.is_success() {
            bail!("set_item_code(global={global}) failed: {:?}", r.into_result().err());
        }
        println!("[setup] item code mode: {}", if global { "global (NEP-591)" } else { "local deploy" });
        Ok(Self {
            worker,
            collection,
            owner,
            lessee,
            buyer,
            inspector,
            outsider,
            fee,
            test_name: test_name.to_string(),
            step: std::cell::Cell::new(0),
        })
    }

    /// Mint a box to `owner`, inspection valid `valid_secs` from now.
    pub async fn mint(&self, owner: &Account, valid_secs: u64) -> Result<AccountId> {
        let deposit = if code_mode_global() { NEAR / 2 } else { 3 * NEAR };
        let now = self.now_secs().await?;
        let r = self
            .collection
            .call("mint")
            .args_json(json!({
                "owner": owner.id(),
                "inspector": self.inspector.id(),
                "inspection_valid_until": now + valid_secs,
            }))
            .deposit(yocto(deposit))
            .gas(Gas::from_tgas(150))
            .transact()
            .await?;
        let token: Option<String> = r.clone().into_result().map_err(|e| anyhow!("mint: {e:?}"))?.json()?;
        let token = token.ok_or_else(|| anyhow!("mint returned None: {:?}", r.logs()))?;
        let box_id: AccountId = format!("box-{}.{}", token, self.collection.id()).parse()?;
        println!("[{}] minted token {token} as {box_id} to {}", self.test_name, owner.id());
        Ok(box_id)
    }

    pub async fn now_secs(&self) -> Result<u64> {
        Ok(self.worker.view_block().await?.timestamp() / 1_000_000_000)
    }
    pub async fn height(&self) -> Result<u64> {
        Ok(self.worker.view_block().await?.height())
    }

    pub async fn view(&self, contract: &AccountId, f: &str, args: Value) -> Result<Value> {
        Ok(self.worker.view(contract, f).args_json(args).await?.json()?)
    }
    pub async fn bview(&self, b: &AccountId, f: &str) -> Result<Value> {
        self.view(b, f, json!({})).await
    }
    pub async fn u128_view(&self, b: &AccountId, f: &str, args: Value) -> Result<u128> {
        let v = self.view(b, f, args).await?;
        Ok(v.as_str().ok_or_else(|| anyhow!("{f}: not a U128 {v}"))?.parse()?)
    }
    pub async fn pending(&self, b: &AccountId, who: &Account) -> Result<u128> {
        self.u128_view(b, "get_pending", json!({ "account": who.id() })).await
    }
    pub async fn reserve(&self, b: &AccountId) -> Result<u128> {
        self.u128_view(b, "get_reserve", json!({})).await
    }
    pub async fn accrued(&self, b: &AccountId) -> Result<u128> {
        self.u128_view(b, "get_accrued_hire", json!({})).await
    }
    pub async fn box_owner(&self, b: &AccountId) -> Result<String> {
        Ok(self.bview(b, "get_owner").await?.as_str().unwrap().to_string())
    }
    pub async fn collection_owner(&self, token: &str) -> Result<Option<String>> {
        let v = self.view(self.collection.id(), "owner_of", json!({ "token_id": token })).await?;
        Ok(v.as_str().map(|s| s.to_string()))
    }
    pub async fn balance(&self, a: &AccountId) -> Result<u128> {
        Ok(self.worker.view_account(a).await?.balance.as_yoctonear())
    }
    pub async fn fee_balance(&self) -> Result<u128> {
        self.balance(self.fee.id()).await
    }
    pub async fn fees_paid(&self, b: &AccountId) -> Result<u128> {
        self.u128_view(b, "get_fees_paid", json!({})).await
    }
    /// The fee account gained exactly `expect` yocto since `before` (it never
    /// signs, so nothing else moves its balance). Returns the new balance.
    pub async fn assert_fee(&self, before: u128, expect: u128, what: &str) -> Result<u128> {
        let now = self.fee_balance().await?;
        if now != before + expect {
            bail!("fee check [{what}]: fee account {} gained {} yocto, expected exactly {expect}", self.fee.id(), now as i128 - before as i128);
        }
        println!("[{}]   FEE OK [{what}]: {} received exactly {expect} yocto ({} NEAR)", self.test_name, self.fee.id(), fmt_near(expect));
        Ok(now)
    }

    /// Rule 14 + zero access keys. Call after every step.
    pub async fn check(&self, b: &AccountId, what: &str) -> Result<()> {
        let n = self.step.get() + 1;
        self.step.set(n);
        let p = self.bview(b, "get_invariant_parts").await?;
        let g = |k: &str| -> u128 { p[k].as_str().unwrap().parse().unwrap() };
        let liabilities = g("reserve") + g("pending_sum") + g("held_lessee_deposits") + g("locked_buyer_payments");
        let acct = self.worker.view_account(b).await?;
        let rpc_balance = acct.balance.as_yoctonear() + acct.locked.as_yoctonear();
        let rpc_storage = acct.storage_usage as u128 * STORAGE_BYTE_COST;
        let need = rpc_storage + liabilities;
        let keys = self.worker.view_access_keys(b).await?;
        let ok = p["holds"].as_bool() == Some(true) && rpc_balance >= need && keys.is_empty();
        println!(
            "[{} step {n:02}] {what}: balance={} storage={} reserve={} pending={} deposits={} locked={} surplus={} keys={} -> {}",
            self.test_name,
            fmt_near(rpc_balance),
            fmt_near(rpc_storage),
            fmt_near(g("reserve")),
            fmt_near(g("pending_sum")),
            fmt_near(g("held_lessee_deposits")),
            fmt_near(g("locked_buyer_payments")),
            fmt_near(rpc_balance.saturating_sub(need)),
            keys.len(),
            if ok { "INVARIANT OK, ZERO KEYS" } else { "FAIL" }
        );
        if p["holds"].as_bool() != Some(true) {
            bail!("box reports invariant broken: {p}");
        }
        if rpc_balance < need {
            bail!("rpc invariant broken: balance {rpc_balance} < need {need}");
        }
        if !keys.is_empty() {
            bail!("box has {} access keys", keys.len());
        }
        Ok(())
    }

    // ------------------------------------------------------------ actions

    pub async fn call(
        &self,
        who: &Account,
        contract: &AccountId,
        f: &str,
        args: Value,
        deposit: u128,
    ) -> Result<ExecutionFinalResult> {
        Ok(who
            .call(contract, f)
            .args_json(args)
            .deposit(yocto(deposit))
            .gas(Gas::from_tgas(150))
            .transact()
            .await?)
    }

    pub async fn ok(&self, who: &Account, c: &AccountId, f: &str, args: Value, dep: u128) -> Result<Value> {
        let r = self.call(who, c, f, args, dep).await?;
        let s = r.into_result().map_err(|e| anyhow!("{f} by {} failed: {e:?}", who.id()))?;
        Ok(s.json::<Value>().unwrap_or(Value::Null))
    }

    /// Expect failure whose error text contains `needle`.
    pub async fn fail(&self, who: &Account, c: &AccountId, f: &str, args: Value, dep: u128, needle: &str) -> Result<()> {
        let r = self.call(who, c, f, args, dep).await?;
        match r.into_result() {
            Ok(v) => bail!("{f} by {} unexpectedly succeeded: {:?}", who.id(), v.json::<Value>().ok()),
            Err(e) => {
                let s = format!("{e:?}");
                if !s.contains(needle) {
                    bail!("{f} by {} failed with unexpected error (wanted {needle}): {s}", who.id());
                }
                println!("[{}]   {f} by {} refused as expected: {needle}", self.test_name, who.id());
                Ok(())
            }
        }
    }

    pub async fn buy(&self, buyer: &Account, b: &AccountId, price: u128) -> Result<String> {
        let r = buyer
            .call(b, "buy")
            .deposit(yocto(price))
            .gas(Gas::from_tgas(150))
            .transact()
            .await?;
        let s = r.into_result().map_err(|e| anyhow!("buy failed: {e:?}"))?;
        Ok(s.json::<String>()?)
    }

    /// Withdraw and verify the payout equals the caller's pending balance
    /// (plus their accrued hire if they are the current owner outside a sale
    /// lock) and nothing more. Measured on the box side, where the only
    /// other balance movement is the gas reward the box earns (< 0.01 NEAR),
    /// and on the receiver side, where only gas is lost. Returns the amount.
    pub async fn withdraw(&self, who: &Account, b: &AccountId) -> Result<u128> {
        let mut expect = self.pending(b, who).await?;
        if self.box_owner(b).await? == who.id().as_str() && self.bview(b, "get_lock").await?.is_null() {
            expect += self.accrued(b).await?;
        }
        let reserve = self.reserve(b).await?;
        let box_before = self.balance(b).await?;
        let who_before = self.balance(who.id()).await?;
        let r = who.call(b, "withdraw").gas(Gas::from_tgas(50)).transact().await?;
        r.into_result().map_err(|e| anyhow!("withdraw by {}: {e:?}", who.id()))?;
        let box_after = self.balance(b).await?;
        let who_after = self.balance(who.id()).await?;
        let box_drop = box_before as i128 - box_after as i128;
        let (expect_i, slack) = (expect as i128, (NEAR / 100) as i128);
        if box_drop > expect_i || box_drop + slack < expect_i {
            bail!("withdraw by {}: box paid {box_drop}, expected {expect}", who.id());
        }
        if who_after + NEAR / 10 < who_before + expect {
            bail!("withdraw by {}: receiver gained too little ({who_before} -> {who_after}, expected +{expect})", who.id());
        }
        if self.pending(b, who).await? != 0
            || (self.box_owner(b).await? == who.id().as_str() && self.bview(b, "get_lock").await?.is_null() && self.accrued(b).await? != 0)
        {
            bail!("pending / swept hire not zeroed after withdraw");
        }
        if self.reserve(b).await? != reserve {
            bail!("withdraw changed the reserve");
        }
        println!("[{}]   {} withdrew {} NEAR (box paid exactly that)", self.test_name, who.id(), fmt_near(expect));
        Ok(expect)
    }

    // ------------------------------------------------------ state patching

    pub async fn box_state(&self, b: &AccountId) -> Result<onhire_item::BoxContract> {
        let state = self.worker.view_state(b).await?;
        let raw = state.get(b"STATE".as_slice()).ok_or_else(|| anyhow!("no STATE"))?;
        Ok(near_sdk::borsh::from_slice(raw)?)
    }

    /// Overwrite the box STATE (and extra raw keys), add `extra_balance`
    /// yocto to the account, and keep `storage_usage` honest.
    pub async fn patch_box(
        &self,
        b: &AccountId,
        st: &onhire_item::BoxContract,
        extra: Vec<(Vec<u8>, Vec<u8>)>,
        extra_balance: u128,
    ) -> Result<()> {
        let cur = self.worker.view_state(b).await?;
        let new_state = near_sdk::borsh::to_vec(st)?;
        let mut delta: i64 = new_state.len() as i64 - cur.get(b"STATE".as_slice()).map(|v| v.len()).unwrap_or(0) as i64;
        for (k, v) in &extra {
            match cur.get(k) {
                Some(old) => delta += v.len() as i64 - old.len() as i64,
                None => delta += (k.len() + v.len() + 40) as i64,
            }
        }
        let mut tx = self.worker.patch(b).state(b"STATE", &new_state);
        for (k, v) in &extra {
            tx = tx.state(k, v);
        }
        tx = tx.account_from_current(move |acc| {
            let bal = acc.balance.as_yoctonear() + extra_balance;
            let su = (acc.storage_usage as i64 + delta) as u64;
            AccountDetailsPatch::from(acc).balance(NearToken::from_yoctonear(bal)).storage_usage(su)
        });
        tx.transact().await?;
        Ok(())
    }
}

/// Storage key of `locked_buyer_payments[sale_id]` (StorageKey::LockedPayments = 1).
pub fn locked_key(sale_id: u64) -> Vec<u8> {
    let mut k = vec![1u8];
    k.extend_from_slice(&sale_id.to_le_bytes());
    k
}
/// Storage key of `pending_withdrawals[account]` (StorageKey::Pending = 0).
pub fn pending_key(account: &str) -> Vec<u8> {
    let mut k = vec![0u8];
    k.extend(near_sdk::borsh::to_vec(&account.to_string()).unwrap());
    k
}
pub fn borsh_u128(v: u128) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

pub fn fmt_near(y: u128) -> String {
    format!("{}.{:06}", y / NEAR, (y % NEAR) / 1_000_000_000_000_000_000)
}

pub fn sdk_id(a: &Account) -> near_sdk::AccountId {
    a.id().as_str().parse().unwrap()
}
