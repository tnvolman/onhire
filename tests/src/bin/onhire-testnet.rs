//! Onhire on NEAR **testnet**: deploy the two contracts and run the seven
//! behaviors (spec section 6, plus t7 for the protocol fee) against the real
//! chain.
//!
//! After every step it asserts rule 14 (from the box and independently from
//! the RPC account view) and that the box account has zero access keys —
//! the same check as the sandbox harness. Every transaction is recorded with
//! its hash (explorer link) to `logs/testnet-records.jsonl`.
//!
//! Testnet only. Keys come from `$ONHIRE_KEYS` (default `~/.onhire-testnet`),
//! written by `scripts/testnet/create-accounts.sh`. Nothing here can target
//! mainnet: the worker is `near_workspaces::testnet()` and every account id
//! is required to end in `.testnet`.
//!
//! Usage:
//!   cargo run -p onhire-tests --bin onhire-testnet -- fee-setup|deploy|t1|...|t7
//!
//! Protocol fee: every hire payment and sale also checks that the testnet fee
//! account (`$ONHIRE_FEE_ACCOUNT`, default `onhire-fee-n12.testnet`) received
//! exactly 1% and that the 15% reserve is untouched by it. The fee account
//! must be a throwaway `.testnet` account; this runner never targets mainnet.

use anyhow::{anyhow, bail, ensure, Context, Result};
use near_workspaces::network::Testnet;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::{Gas, NearToken, SecretKey};
use near_workspaces::{Account, AccountId, Worker};
use onhire_tests::{fee_cut, fmt_near, owner_cut, reserve_cut, seller_net, wasm_path, NEAR, SALE_UNDO_DELAY_BLOCKS, STORAGE_BYTE_COST};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::io::Write;

const DAY: u64 = 86_400;
const EXPLORER: &str = "https://testnet.nearblocks.io/txns/";

fn yocto(n: u128) -> NearToken {
    NearToken::from_yoctonear(n)
}
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}
fn keys_dir() -> std::path::PathBuf {
    std::env::var("ONHIRE_KEYS")
        .map(Into::into)
        .unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".onhire-testnet"))
}
fn suffix() -> String {
    std::env::var("ONHIRE_SUFFIX").unwrap_or_else(|_| "n12".into())
}
/// The collection for the lessee-deposit build (two-step lease: owner
/// offers, lessee accepts and posts the deposit). `onhire.testnet` (tests 1-6
/// proof at 7057b06) keeps the pre-fee code and `v2.onhire.testnet` keeps the
/// owner-funded-deposit code: boxes are keyless and item code is set once,
/// so neither can take the new lease flow, and this build runs on a fresh
/// sub-account. `ONHIRE_COLLECTION` overrides.
fn collection_id() -> String {
    std::env::var("ONHIRE_COLLECTION").unwrap_or_else(|_| "v3.onhire.testnet".into())
}
/// Throwaway `.testnet` fee account for the testnet deploy.
fn fee_account_id() -> Result<AccountId> {
    let id = std::env::var("ONHIRE_FEE_ACCOUNT").unwrap_or_else(|_| format!("onhire-fee-{}.testnet", suffix()));
    ensure!(id.ends_with(".testnet"), "refusing non-testnet fee account {id}");
    Ok(id.parse()?)
}

fn load_account(worker: &Worker<Testnet>, id: &str, file: Option<&str>) -> Result<Account> {
    ensure!(id.ends_with(".testnet"), "refusing non-testnet account {id}");
    let path = keys_dir().join(file.map(|f| f.to_string()).unwrap_or(format!("{id}.json")));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).with_context(|| format!("{path:?}"))?)?;
    ensure!(v["account_id"] == json!(id), "key file {path:?} is for {}", v["account_id"]);
    let sk: SecretKey = v["private_key"].as_str().unwrap().parse()?;
    Ok(Account::from_secret_key(id.parse()?, sk, worker))
}

#[derive(Clone, Debug, Default)]
struct BuyTiming {
    sale_id: u64,
    started_at_block: u64,
    transfer_block: u64,
    callback_block: u64,
    outcome: String,
}

struct Env {
    worker: Worker<Testnet>,
    coll: Account,
    owner: Account,
    lessee: Account,
    buyer: Account,
    buyer2: Account,
    inspector: Account,
    outsider: Account,
    racer1: Account,
    /// Fee recipient (testnet throwaway). Never signs a tx in this runner.
    fee: AccountId,
    test_name: String,
    step: Cell<u32>,
    records: RefCell<std::fs::File>,
    timings: RefCell<Vec<BuyTiming>>,
}

impl Env {
    async fn new(test_name: &str) -> Result<Self> {
        let rpc = std::env::var("NEAR_TESTNET_RPC").unwrap_or_else(|_| "https://rpc.testnet.near.org".into());
        let worker = near_workspaces::testnet().rpc_addr(&rpc).await?;
        let s = suffix();
        let role = |r: &str| load_account(&worker, &format!("onhire-{r}-{s}.testnet"), None);
        std::fs::create_dir_all(root().join("logs"))?;
        let records = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root().join("logs/testnet-records.jsonl"))?;
        Ok(Self {
            coll: load_account(&worker, &collection_id(), None)?,
            owner: role("owner")?,
            lessee: role("lessee")?,
            buyer: role("buyer")?,
            buyer2: role("buyer2")?,
            inspector: role("inspector")?,
            outsider: role("outsider")?,
            racer1: role("racer1")?,
            fee: fee_account_id()?,
            worker,
            test_name: test_name.to_string(),
            step: Cell::new(0),
            records: RefCell::new(records),
            timings: RefCell::new(vec![]),
        })
    }

    fn cid(&self) -> &AccountId {
        self.coll.id()
    }

    fn record(&self, label: &str, signer: &AccountId, receiver: &AccountId, method: &str, r: &ExecutionFinalResult) {
        let tx = r.outcome().transaction_hash.to_string();
        let ok = r.is_success();
        let err = if ok { None } else { Some(format!("{:?}", r.clone().into_result().err()).chars().take(400).collect::<String>()) };
        let line = json!({
            "test": self.test_name, "pid": std::process::id(), "step": self.step.get(), "label": label,
            "signer": signer, "receiver": receiver, "method": method,
            "tx": tx, "ok": ok, "err": err, "explorer": format!("{EXPLORER}{tx}"),
            "ts": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });
        let _ = writeln!(self.records.borrow_mut(), "{line}");
        println!("[{}]     tx {} {method} by {signer} -> {} {EXPLORER}{tx}", self.test_name, if ok { "ok  " } else { "FAIL" }, receiver);
    }

    // ------------------------------------------------------------ views

    async fn view(&self, c: &AccountId, f: &str, args: Value) -> Result<Value> {
        Ok(self.worker.view(c, f).args_json(args).await?.json()?)
    }
    async fn bview(&self, b: &AccountId, f: &str) -> Result<Value> {
        self.view(b, f, json!({})).await
    }
    async fn u128_view(&self, b: &AccountId, f: &str, args: Value) -> Result<u128> {
        let v = self.view(b, f, args).await?;
        Ok(v.as_str().ok_or_else(|| anyhow!("{f}: not a U128 {v}"))?.parse()?)
    }
    async fn pending(&self, b: &AccountId, who: &Account) -> Result<u128> {
        self.u128_view(b, "get_pending", json!({ "account": who.id() })).await
    }
    async fn reserve(&self, b: &AccountId) -> Result<u128> {
        self.u128_view(b, "get_reserve", json!({})).await
    }
    async fn accrued(&self, b: &AccountId) -> Result<u128> {
        self.u128_view(b, "get_accrued_hire", json!({})).await
    }
    async fn box_owner(&self, b: &AccountId) -> Result<String> {
        Ok(self.bview(b, "get_owner").await?.as_str().unwrap().to_string())
    }
    async fn collection_owner(&self, token: &str) -> Result<Option<String>> {
        let v = self.view(self.cid(), "owner_of", json!({ "token_id": token })).await?;
        Ok(v.as_str().map(|s| s.to_string()))
    }
    async fn balance(&self, a: &AccountId) -> Result<u128> {
        Ok(self.worker.view_account(a).await?.balance.as_yoctonear())
    }
    async fn fee_balance(&self) -> Result<u128> {
        self.balance(&self.fee).await
    }
    async fn fees_paid(&self, b: &AccountId) -> Result<u128> {
        self.u128_view(b, "get_fees_paid", json!({})).await
    }
    /// The fee account gained exactly `expect` yocto since `before`. Nobody
    /// else sends to it, so the delta is exact. Polls briefly in case the
    /// fee receipt's block is not yet visible to the RPC view.
    async fn assert_fee(&self, before: u128, expect: u128, what: &str) -> Result<u128> {
        let mut now = self.fee_balance().await?;
        for _ in 0..10 {
            if now >= before + expect {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            now = self.fee_balance().await?;
        }
        let ok = now == before + expect;
        let _ = writeln!(self.records.borrow_mut(), "{}", json!({"test": self.test_name, "fee_check": what,
            "fee_account": self.fee, "before": before.to_string(), "after": now.to_string(), "expected_delta": expect.to_string(), "ok": ok}));
        ensure!(ok, "fee check [{what}]: {} gained {} yocto, expected exactly {expect}", self.fee, now as i128 - before as i128);
        println!("[{}]   FEE OK [{what}]: {} received exactly {expect} yocto ({} NEAR)", self.test_name, self.fee, fmt_near(expect));
        Ok(now)
    }
    async fn now_secs(&self) -> Result<u64> {
        Ok(self.worker.view_block().await?.timestamp() / 1_000_000_000)
    }
    async fn raw_state(&self, b: &AccountId) -> Result<Vec<u8>> {
        let st = self.worker.view_state(b).await?;
        st.get(b"STATE".as_slice()).cloned().ok_or_else(|| anyhow!("no STATE"))
    }

    /// Rule 14 + zero access keys. Called after every step.
    async fn check(&self, b: &AccountId, what: &str) -> Result<()> {
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
        let _ = writeln!(
            self.records.borrow_mut(),
            "{}",
            json!({"test": self.test_name, "step": n, "check": what, "ok": ok, "box": b})
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

    async fn call_gas(&self, who: &Account, c: &AccountId, f: &str, args: Value, dep: u128, tgas: u64) -> Result<ExecutionFinalResult> {
        let r = self.submit(who, c, f, || who.call(c, f).args_json(args.clone()).deposit(yocto(dep)).gas(Gas::from_tgas(tgas))).await?;
        self.record(f, who.id(), c, f, &r);
        Ok(r)
    }
    /// Broadcast once; if the RPC accepted it but the tx is unknown to two
    /// RPCs after 90 s (dropped before inclusion), resend once. The resend has
    /// a higher nonce, so once it lands the dropped tx can never land; the
    /// nonce audit (scripts/testnet/audit-nonces.sh) would expose a duplicate.
    async fn submit<'a>(
        &self,
        who: &Account,
        c: &AccountId,
        f: &str,
        mk: impl Fn() -> near_workspaces::operations::CallTransaction,
    ) -> Result<ExecutionFinalResult> {
        for attempt in 0..2 {
            let st = mk().transact_async().await?;
            let hash = st.hash().to_string();
            match send_once(st).await {
                Ok(r) => return Ok(r),
                Err(err) if attempt == 0 && err.to_string().contains("never landed") && self.tx_unknown(&hash, who.id()).await => {
                    println!("[{}]   {f} tx {hash} by {} was dropped before inclusion; resending once", self.test_name, who.id());
                    let _ = writeln!(self.records.borrow_mut(), "{}", json!({"test": self.test_name, "pid": std::process::id(),
                        "step": self.step.get(), "label": "dropped before inclusion (unknown to two RPCs); resent", "signer": who.id(),
                        "receiver": c, "method": f, "tx": hash, "ok": false, "landed": false}));
                }
                Err(err) => return Err(err),
            }
        }
        unreachable!()
    }
    async fn tx_unknown(&self, hash: &str, signer: &AccountId) -> bool {
        let body = json!({"jsonrpc": "2.0", "id": 1, "method": "tx",
            "params": {"tx_hash": hash, "sender_account_id": signer, "wait_until": "NONE"}}).to_string();
        let mut unknown = 0;
        for rpc in ["https://rpc.testnet.near.org", "https://rpc.testnet.fastnear.com"] {
            let out = std::process::Command::new("curl")
                .args(["-s", "-m", "20", rpc, "-H", "Content-Type: application/json", "-d", &body])
                .output();
            if let Ok(o) = out {
                if String::from_utf8_lossy(&o.stdout).contains("UNKNOWN_TRANSACTION") {
                    unknown += 1;
                }
            }
        }
        unknown == 2
    }

    /// A brand-new throwaway buyer (sub-account of the outsider, no faucet
    /// needed) that has never owned a token. Key saved to $ONHIRE_KEYS.
    async fn fresh_buyer(&self, tag: &str) -> Result<Account> {
        let sk = SecretKey::from_random(near_workspaces::types::KeyType::ED25519);
        let name = format!("{tag}{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs());
        let id: AccountId = format!("{name}.{}", self.outsider.id()).parse()?;
        ensure!(id.as_str().ends_with(".testnet"));
        std::fs::write(
            keys_dir().join(format!("{id}.json")),
            json!({"account_id": id, "public_key": sk.public_key().to_string(), "private_key": sk.to_string()}).to_string(),
        )?;
        let ex = self.outsider.create_subaccount(&name).initial_balance(yocto(NEAR * 3 / 2)).keys(sk).transact().await?;
        self.record("create fresh throwaway buyer (sub-account)", self.outsider.id(), &id, "CreateAccount", &ex.details);
        ensure!(ex.details.is_success(), "create {id} failed");
        println!("[{}]   fresh buyer {id}", self.test_name);
        Ok(ex.result)
    }

    async fn call(&self, who: &Account, c: &AccountId, f: &str, args: Value, dep: u128) -> Result<ExecutionFinalResult> {
        self.call_gas(who, c, f, args, dep, 150).await
    }
    async fn ok(&self, who: &Account, c: &AccountId, f: &str, args: Value, dep: u128) -> Result<Value> {
        let r = self.call(who, c, f, args, dep).await?;
        let s = r.into_result().map_err(|e| anyhow!("{f} by {} failed: {e:?}", who.id()))?;
        Ok(s.json::<Value>().unwrap_or(Value::Null))
    }
    async fn fail(&self, who: &Account, c: &AccountId, f: &str, args: Value, dep: u128, needle: &str) -> Result<()> {
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

    /// Record per-receipt block heights of a `buy`: the box's lock block,
    /// the collection transfer block, and the callback block.
    async fn buy_timing(&self, r: &ExecutionFinalResult, b: &AccountId) -> Result<BuyTiming> {
        let mut t = BuyTiming::default();
        for o in r.receipt_outcomes() {
            let h = self.worker.view_block().block_hash(o.block_hash).await?.height();
            for l in &o.logs {
                if let Some(j) = l.strip_prefix("EVENT_JSON:") {
                    let v: Value = serde_json::from_str(j)?;
                    match v["event"].as_str() {
                        Some("sale_locked") => {
                            t.sale_id = v["data"][0]["sale_id"].as_u64().unwrap();
                            t.started_at_block = v["data"][0]["started_at_block"].as_u64().unwrap();
                        }
                        Some("sale_finalized") => {
                            t.callback_block = h;
                            t.outcome = format!("finalized via {}", v["data"][0]["via"].as_str().unwrap_or("?"));
                        }
                        Some("sale_undone") => {
                            t.callback_block = h;
                            t.outcome = format!("undone via {}", v["data"][0]["via"].as_str().unwrap_or("?"));
                        }
                        _ => {}
                    }
                }
            }
            if &o.executor_id == self.cid() && t.transfer_block == 0 {
                t.transfer_block = h;
            }
        }
        if t.callback_block == 0 {
            // Callback ran but emitted nothing (stale no-op): find the box's last receipt.
            for o in r.receipt_outcomes() {
                if &o.executor_id == b {
                    t.callback_block = self.worker.view_block().block_hash(o.block_hash).await?.height();
                }
            }
        }
        println!(
            "[{}]   TIMING sale {}: lock@{} transfer@{} (+{}) callback@{} (+{}) -> {}",
            self.test_name,
            t.sale_id,
            t.started_at_block,
            t.transfer_block,
            t.transfer_block.saturating_sub(t.started_at_block),
            t.callback_block,
            t.callback_block.saturating_sub(t.started_at_block),
            t.outcome
        );
        let _ = writeln!(
            self.records.borrow_mut(),
            "{}",
            json!({"test": self.test_name, "timing": {"sale_id": t.sale_id, "lock_block": t.started_at_block,
                "transfer_block": t.transfer_block, "callback_block": t.callback_block,
                "callback_minus_lock": t.callback_block.saturating_sub(t.started_at_block), "outcome": t.outcome}})
        );
        self.timings.borrow_mut().push(t.clone());
        Ok(t)
    }

    async fn buy_raw(&self, buyer: &Account, b: &AccountId, price: u128) -> Result<ExecutionFinalResult> {
        let r = self.submit(buyer, b, "buy", || buyer.call(b, "buy").deposit(yocto(price)).gas(Gas::from_tgas(150))).await?;
        self.record("buy", buyer.id(), b, "buy", &r);
        Ok(r)
    }
    async fn buy(&self, buyer: &Account, b: &AccountId, price: u128) -> Result<String> {
        let r = self.buy_raw(buyer, b, price).await?;
        self.buy_timing(&r, b).await?;
        let s = r.into_result().map_err(|e| anyhow!("buy failed: {e:?}"))?;
        Ok(s.json::<String>()?)
    }

    /// Withdraw and verify the payout equals the caller's pending balance
    /// (plus accrued hire if they are the owner outside a lock) and nothing more.
    async fn withdraw(&self, who: &Account, b: &AccountId) -> Result<u128> {
        let mut expect = self.pending(b, who).await?;
        if self.box_owner(b).await? == who.id().as_str() && self.bview(b, "get_lock").await?.is_null() {
            expect += self.accrued(b).await?;
        }
        let reserve = self.reserve(b).await?;
        let box_before = self.balance(b).await?;
        let who_before = self.balance(who.id()).await?;
        let r = self.submit(who, b, "withdraw", || who.call(b, "withdraw").gas(Gas::from_tgas(50))).await?;
        self.record("withdraw", who.id(), b, "withdraw", &r);
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
        ensure!(self.pending(b, who).await? == 0, "pending not zeroed after withdraw");
        ensure!(self.reserve(b).await? == reserve, "withdraw changed the reserve");
        println!("[{}]   {} withdrew {} NEAR (box paid exactly that)", self.test_name, who.id(), fmt_near(expect));
        Ok(expect)
    }

    /// Mint a box to `owner` (minter = collection account), inspection valid `valid_secs`.
    async fn mint(&self, owner: &Account, valid_secs: u64) -> Result<AccountId> {
        let now = self.now_secs().await?;
        let r = self
            .call_gas(
                &self.coll,
                &self.cid().clone(),
                "mint",
                json!({ "owner": owner.id(), "inspector": self.inspector.id(), "inspection_valid_until": now + valid_secs }),
                NEAR / 2,
                150,
            )
            .await?;
        let token: Option<String> = r.clone().into_result().map_err(|e| anyhow!("mint: {e:?}"))?.json()?;
        let token = token.ok_or_else(|| anyhow!("mint returned None: {:?}", r.logs()))?;
        let box_id: AccountId = format!("box-{}.{}", token, self.cid()).parse()?;
        println!("[{}] minted token {token} as {box_id} to {}", self.test_name, owner.id());
        Ok(box_id)
    }

    async fn leased_box(&self, deposit: u128) -> Result<AccountId> {
        let b = self.mint(&self.owner, 30 * DAY).await?;
        self.check(&b, "minted").await?;
        let now = self.now_secs().await?;
        self.ok(&self.inspector, &b, "attest_inspection", json!({ "valid_until": now + 60 * DAY }), 0).await?;
        self.check(&b, "inspector attested").await?;
        self.ok(&self.owner, &b, "start_lease",
            json!({ "lessee": self.lessee.id(), "rate_per_day": (NEAR / 100).to_string(), "duration_secs": 30 * DAY,
                    "deposit": deposit.to_string() }),
            1).await?;
        self.check(&b, "lease offered").await?;
        self.ok(&self.lessee, &b, "accept_lease", json!({}), deposit).await?;
        self.check(&b, "lessee accepted lease (posted deposit)").await?;
        Ok(b)
    }
}

// ---------------------------------------------------------------------------
// fund / deploy
// ---------------------------------------------------------------------------

async fn fund(e: &Env) -> Result<()> {
    // Fold throwaway faucet accounts into the collection (delete -> beneficiary).
    let s = suffix();
    let mut donors = vec![];
    if keys_dir().join("probe.keep").exists() {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(keys_dir().join("probe.keep"))?)?;
        donors.push(load_account(&e.worker, v["account_id"].as_str().unwrap(), Some("probe.keep"))?);
    }
    for r in std::env::var("ONHIRE_DONORS").unwrap_or_else(|_| "racer2".into()).split(',').filter(|x| !x.is_empty()) {
        donors.push(load_account(&e.worker, &format!("onhire-{r}-{s}.testnet"), None)?);
    }
    for d in donors {
        let id = d.id().clone();
        if e.worker.view_account(&id).await.is_err() {
            println!("[fund] {id} already gone");
            continue;
        }
        let r = d.delete_account(e.cid()).await?;
        e.record("fund: delete donor into collection", &id, e.cid(), "DeleteAccount", &r);
        ensure!(r.is_success(), "delete {id}: {:?}", r.into_result().err());
    }
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    println!("[fund] collection balance {} NEAR", fmt_near(e.balance(e.cid()).await?));
    Ok(())
}

/// Move test NEAR from the (now idle) buyer2 throwaway account to the
/// collection so it can keep paying the 0.5 NEAR per mint.
async fn topup(e: &Env) -> Result<()> {
    let amt = NEAR * 6;
    let r = send_once(e.buyer2.batch(e.cid()).transfer(yocto(amt)).transact_async().await?).await?;
    e.record("top up collection from buyer2", e.buyer2.id(), e.cid(), "Transfer", &r);
    ensure!(r.is_success(), "topup failed");
    println!("[topup] collection balance {} NEAR", fmt_near(e.balance(e.cid()).await?));
    Ok(())
}

/// One time, per collection build: create the throwaway testnet fee account
/// (top-level, via the public `testnet` registrar) if it does not exist yet,
/// and the fresh collection sub-account (default `v3.onhire.testnet`, see
/// `collection_id`; `v2.onhire.testnet` was made the same way), then move
/// idle test NEAR from the other throwaway accounts into the collection for
/// the global code deploy. Keys go to $ONHIRE_KEYS. Testnet only.
async fn fee_setup(worker: &Worker<Testnet>) -> Result<()> {
    let mut rec = std::fs::OpenOptions::new().create(true).append(true).open(root().join("logs/testnet-records.jsonl"))?;
    let mut log = |label: &str, signer: &AccountId, receiver: &AccountId, method: &str, r: &ExecutionFinalResult| {
        let tx = r.outcome().transaction_hash.to_string();
        let _ = writeln!(rec, "{}", json!({"test": "testnet-fee-setup", "pid": std::process::id(), "label": label, "signer": signer,
            "receiver": receiver, "method": method, "tx": tx, "ok": r.is_success(), "explorer": format!("{EXPLORER}{tx}")}));
        println!("[fee-setup] {label}: {} {EXPLORER}{tx}", if r.is_success() { "ok" } else { "FAIL" });
    };
    let s = suffix();
    let fee = fee_account_id()?;
    let coll_id: AccountId = collection_id().parse()?;
    ensure!(coll_id.as_str().ends_with(".onhire.testnet"), "fee-setup creates the collection as a sub-account of onhire.testnet");
    let v1 = load_account(worker, "onhire.testnet", None)?;
    let donor = load_account(worker, &format!("onhire-buyer2-{s}.testnet"), None)?;
    let save_key = |id: &AccountId, sk: &SecretKey| -> Result<()> {
        let f = keys_dir().join(format!("{id}.json"));
        ensure!(!f.exists(), "key file {f:?} already exists");
        std::fs::write(&f, json!({"account_id": id, "public_key": sk.public_key().to_string(), "private_key": sk.to_string()}).to_string())?;
        std::fs::set_permissions(&f, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
        Ok(())
    };
    // 1. Fee account: top-level .testnet via the `testnet` registrar.
    if worker.view_account(&fee).await.is_err() {
        let sk = SecretKey::from_random(near_workspaces::types::KeyType::ED25519);
        save_key(&fee, &sk)?;
        let registrar: AccountId = "testnet".parse()?;
        let r = send_once(donor.call(&registrar, "create_account")
            .args_json(json!({ "new_account_id": fee, "new_public_key": sk.public_key().to_string() }))
            .deposit(yocto(NEAR / 10)).gas(Gas::from_tgas(100)).transact_async().await?).await?;
        log("create testnet fee account via testnet registrar", donor.id(), &registrar, "create_account", &r);
        ensure!(r.is_success(), "create {fee}: {:?}", r.into_result().err());
    } else {
        println!("[fee-setup] {fee} already exists");
    }
    // 2. Collection sub-account of onhire.testnet, with its own deployer key.
    if worker.view_account(&coll_id).await.is_err() {
        let sk = SecretKey::from_random(near_workspaces::types::KeyType::ED25519);
        save_key(&coll_id, &sk)?;
        let sub = coll_id.as_str().strip_suffix(".onhire.testnet").unwrap();
        let free = { let a = worker.view_account(v1.id()).await?; a.balance.as_yoctonear() - a.storage_usage as u128 * STORAGE_BYTE_COST };
        let amt = free.saturating_sub(NEAR / 5);
        let ex = v1.create_subaccount(sub).initial_balance(yocto(amt)).keys(sk).transact().await?;
        log(&format!("create collection sub-account {coll_id}"), v1.id(), &coll_id, "CreateAccount", &ex.details);
        ensure!(ex.details.is_success(), "create {coll_id} failed");
    } else {
        println!("[fee-setup] {coll_id} already exists");
    }
    // 3. Idle test NEAR -> collection (global code deploy burns ~0.1 NEAR/KB).
    let plan: Vec<(&str, u128)> = std::env::var("ONHIRE_FEE_DONATIONS")
        .unwrap_or_else(|_| "inspector:8,racer1:8,owner:4,buyer2:3".into())
        .split(',')
        .filter(|x| !x.is_empty())
        .map(|kv| { let (k, v) = kv.split_once(':').unwrap(); (k, v.parse::<u128>().unwrap() * NEAR) })
        .map(|(k, v)| (Box::leak(k.to_string().into_boxed_str()) as &str, v))
        .collect();
    for (role, amt) in plan {
        let a = load_account(worker, &format!("onhire-{role}-{s}.testnet"), None)?;
        let r = send_once(a.batch(&coll_id).transfer(yocto(amt)).transact_async().await?).await?;
        log(&format!("move {} test NEAR {role} -> collection", fmt_near(amt)), a.id(), &coll_id, "Transfer", &r);
        ensure!(r.is_success(), "transfer from {role} failed");
    }
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    for id in [&fee, &coll_id] {
        println!("[fee-setup] {id} balance {} NEAR", fmt_near(worker.view_account(id).await?.balance.as_yoctonear()));
    }
    Ok(())
}

async fn deploy(e: &Env) -> Result<()> {
    let coll_wasm = std::fs::read(wasm_path("onhire_collection")).context("build contracts first")?;
    let item_wasm = std::fs::read(wasm_path("onhire_item"))?;
    let b0 = e.balance(e.cid()).await?;
    println!("[deploy] collection {} balance {} NEAR", e.cid(), fmt_near(b0));
    let acct = e.worker.view_account(e.cid()).await?;
    if acct.storage_usage < 10_000 {
        let r = e.coll.deploy(&coll_wasm).await?;
        let d = &r.details;
        e.record("deploy collection wasm", e.cid(), e.cid(), "DeployContract", d);
        ensure!(d.is_success(), "deploy failed");
        let r = e.call_gas(&e.coll, &e.cid().clone(), "new", json!({ "minter": e.cid(), "fee_account": e.fee }), 0, 30).await?;
        ensure!(r.is_success(), "new failed: {:?}", r.into_result().err());
    } else {
        println!("[deploy] collection code already deployed (storage {} bytes)", acct.storage_usage);
    }
    let info = e.view(e.cid(), "get_item_code_info", json!({})).await?;
    if info[1].is_null() {
        let b1 = e.balance(e.cid()).await?;
        use near_sdk::base64::Engine;
        let code_b64 = near_sdk::base64::engine::general_purpose::STANDARD.encode(&item_wasm);
        let global = std::env::var("ONHIRE_CODE_MODE").map(|m| m != "local").unwrap_or(true);
        let r = e.call_gas(&e.coll, &e.cid().clone(), "set_item_code", json!({ "code": code_b64, "global": global }), 0, 300).await?;
        ensure!(r.is_success(), "set_item_code(global={global}) failed: {:?}", r.into_result().err());
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let b2 = e.balance(e.cid()).await?;
        println!("[deploy] set_item_code(global={global}) cost {} NEAR (item wasm {} bytes)", fmt_near(b1 - b2), item_wasm.len());
    }
    let info = e.view(e.cid(), "get_item_code_info", json!({})).await?;
    println!("[deploy] item code info: {info}");
    let fa = e.view(e.cid(), "get_fee_account", json!({})).await?;
    ensure!(fa == json!(e.fee), "collection fee_account {fa} != {}", e.fee);
    println!("[deploy] collection fee_account = {fa}");
    println!("[deploy] collection balance now {} NEAR", fmt_near(e.balance(e.cid()).await?));
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 1 — Mid-lease sale
// ---------------------------------------------------------------------------
async fn t1(e: &Env) -> Result<()> {
    let b = e.leased_box(NEAR / 5).await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    let lease_before = e.bview(&b, "get_lease").await?;

    let hire1 = NEAR / 10;
    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), hire1).await?;
    e.check(&b, "lessee paid hire #1").await?;
    let reserve1 = e.reserve(&b).await?;
    ensure!(reserve1 == reserve_cut(hire1), "reserve {reserve1} (fee must not reduce it)");
    ensure!(e.accrued(&b).await? == owner_cut(hire1), "owner accrues 84%");
    let f1 = e.assert_fee(f0, fee_cut(hire1), "hire #1: 1% of the payment").await?;

    let price = NEAR;
    e.ok(&e.owner, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "owner listed").await?;
    e.fail(&e.buyer, &b, "buy", json!({}), price - 1, "ERR_WRONG_ATTACHED_AMOUNT").await?;
    ensure!(e.bview(&b, "get_lock").await?.is_null(), "wrong amount must not lock");
    e.check(&b, "wrong-amount buy refused").await?;
    let outcome = e.buy(&e.buyer, &b, price).await?;
    ensure!(outcome == "finalized", "buy outcome {outcome}");
    e.check(&b, "buyer bought; callback finalized").await?;
    let f2 = e.assert_fee(f1, fee_cut(price), "sale: 1% of the price").await?;

    ensure!(e.box_owner(&b).await? == e.buyer.id().as_str());
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.buyer.id().as_str()));
    let pre_sale_hire = owner_cut(hire1);
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + pre_sale_hire, "seller pending = 99% of price + pre-sale hire");
    ensure!(e.reserve(&b).await? == reserve1, "reserve unchanged by the sale or its fee");
    ensure!(e.bview(&b, "get_lease").await? == lease_before, "lease unchanged");
    ensure!(e.accrued(&b).await? == 0);
    ensure!(e.bview(&b, "get_listing").await?.is_null());
    e.check(&b, "post-sale state asserted").await?;

    let hire2 = NEAR / 5;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), hire2).await?;
    e.check(&b, "lessee paid hire #2 (post-sale)").await?;
    ensure!(e.accrued(&b).await? == owner_cut(hire2), "post-sale hire accrues to buyer");
    ensure!(e.reserve(&b).await? == reserve1 + reserve_cut(hire2), "reserve +15% exactly");
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + pre_sale_hire, "seller pending unchanged");
    e.assert_fee(f2, fee_cut(hire2), "hire #2: 1% of the payment").await?;
    ensure!(e.fees_paid(&b).await? == fee_cut(hire1) + fee_cut(price) + fee_cut(hire2), "box fees_paid counter");

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

/// Force the next collection `nft_transfer` to fail on the real chain: the
/// collection account (which on testnet still has its deployer key, as the
/// README discloses) sends its free balance to `sink`, keeping only its
/// storage stake plus a few bytes. A transfer that must store a new owner
/// set then fails with LackBalanceForState. Returns the amount drained.
async fn drain_collection(e: &Env, sink: &Account) -> Result<u128> {
    let cid = e.cid().clone();
    let mut drained = 0u128;
    let mut gas_est: u128 = 5 * 10u128.pow(20);
    for _ in 0..6 {
        let a = e.worker.view_account(&cid).await?;
        let stake = a.storage_usage as u128 * STORAGE_BYTE_COST;
        let free = a.balance.as_yoctonear().saturating_sub(stake);
        println!("[{}]   collection free balance above stake: {} yocto ({} bytes)", e.test_name, free, free / STORAGE_BYTE_COST);
        if free <= 3 * 10u128.pow(20) {
            break;
        }
        let amount = free.saturating_sub(gas_est + 10u128.pow(20));
        if amount == 0 {
            gas_est /= 2;
            continue;
        }
        let st = e.coll.batch(sink.id()).transfer(yocto(amount)).transact_async().await?;
        let hash = st.hash().to_string();
        match send_once(st).await {
            Ok(r) => {
                e.record("drain collection free balance", &cid, sink.id(), "Transfer", &r);
                if r.is_success() {
                    drained += amount;
                    gas_est = 10u128.pow(20);
                }
            }
            Err(err) => {
                // Rejected at chunk validation (cannot keep its storage stake after
                // gas). No balance moves, but the access-key nonce is consumed.
                let _ = writeln!(e.records.borrow_mut(), "{}", json!({"test": e.test_name, "pid": std::process::id(),
                    "label": "drain transfer rejected at validation (no balance moved)", "signer": cid, "receiver": sink.id(),
                    "method": "Transfer", "tx": hash, "ok": false, "landed": false}));
                println!("[{}]   drain of {amount} rejected ({err}); retrying with more gas headroom", e.test_name);
                gas_est *= 2;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    let a = e.worker.view_account(&cid).await?;
    let free = a.balance.as_yoctonear() - a.storage_usage as u128 * STORAGE_BYTE_COST;
    println!("[{}]   collection drained {} NEAR; free above stake now {free} yocto (~{} bytes)", e.test_name, fmt_near(drained), free / STORAGE_BYTE_COST);

    Ok(drained)
}

async fn restore_collection(e: &Env, from: &Account, drained: u128) -> Result<()> {
    if drained > 0 {
        let r = send_once(from.batch(e.cid()).transfer(yocto(drained)).transact_async().await?).await?;
        e.record("restore collection balance", from.id(), e.cid(), "Transfer", &r);
        ensure!(r.is_success(), "restore failed");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2 — Failed collection transfer undoes the sale
// ---------------------------------------------------------------------------
async fn t2(e: &Env) -> Result<()> {
    let b = e.leased_box(NEAR / 10).await?;
    // Second box for the same owner, so the seller's owner set survives the
    // transfer and the forced failure is a storage-growth failure.
    let b2 = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b2, "second box minted").await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR / 10).await?;
    e.check(&b, "hire paid").await?;
    let price = NEAR / 2;
    e.ok(&e.owner, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "listed").await?;
    let (reserve, accrued, lease) = (e.reserve(&b).await?, e.accrued(&b).await?, e.bview(&b, "get_lease").await?);
    let f0 = e.fee_balance().await?;
    let fees0 = e.fees_paid(&b).await?;
    // A fresh buyer has never owned a token, so the transfer must store a new owner set.
    let buyer2 = e.fresh_buyer("t2b").await?;
    ensure!(e.view(e.cid(), "nft_supply_for_owner", json!({ "account_id": buyer2.id() })).await? == json!("0"));

    // Force the transfer to fail on the real chain: the collection account
    // (which on testnet still has its deployer key, as disclosed) sends away
    // its free balance so it holds only its storage stake plus a few bytes.
    // The collection nft_transfer must then fail with LackBalanceForState.
    let cid = e.cid().clone();
    let drained = drain_collection(e, &e.outsider).await?;

    let res: Result<()> = async {
        let r = e.buy_raw(&buyer2, &b, price).await?;
        e.buy_timing(&r, &b).await?;
        let failures = format!("{:?}", r.receipt_failures());
        ensure!(failures.contains(cid.as_str()) && failures.contains("LackBalanceForState"),
            "expected the collection nft_transfer receipt to fail with LackBalanceForState: {failures}");
        println!("[test2]   collection nft_transfer receipt failed: LackBalanceForState");
        let outcome: String = r.into_result().map_err(|e| anyhow!("{e:?}"))?.json()?;
        ensure!(outcome == "undone", "expected undo, got {outcome}");
        Ok(())
    }
    .await;
    // Always restore the collection balance.
    restore_collection(e, &e.outsider, drained).await?;
    res?;
    e.check(&b, "buy with failing transfer -> callback undid immediately").await?;

    ensure!(e.box_owner(&b).await? == e.owner.id().as_str(), "box owner still seller");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.owner.id().as_str()), "collection owner still seller");
    ensure!(!e.bview(&b, "get_listing").await?.is_null(), "listing remains");
    ensure!(e.reserve(&b).await? == reserve && e.accrued(&b).await? == accrued);
    ensure!(e.bview(&b, "get_lease").await? == lease, "lease unchanged");
    ensure!(e.bview(&b, "get_lock").await?.is_null(), "lock cleared");
    ensure!(e.pending(&b, &buyer2).await? == price, "buyer refund pending");
    ensure!(e.pending(&b, &e.owner).await? == 0, "seller not credited");
    e.assert_fee(f0, 0, "undone sale charges no fee").await?;
    ensure!(e.fees_paid(&b).await? == fees0);
    e.check(&b, "undo state asserted").await?;
    let got = e.withdraw(&buyer2, &b).await?;
    ensure!(got == price);
    e.check(&b, "buyer withdrew refund").await?;

    ensure!(e.buy(&buyer2, &b, price).await? == "finalized");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(buyer2.id().as_str()));
    e.assert_fee(f0, fee_cut(price), "retry sale: 1% of the price").await?;
    ensure!(e.pending(&b, &e.owner).await? == seller_net(price) + accrued, "seller credited 99% + accrued");
    ensure!(e.reserve(&b).await? == reserve, "reserve untouched by the sale fee");
    e.check(&b, "retry buy finalized after collection restored").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3 — Direct transfer is blocked
// ---------------------------------------------------------------------------
async fn t3(e: &Env) -> Result<()> {
    let b = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b, "minted").await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    let coll = e.cid().clone();
    let deployer = e.coll.clone();
    for listed in [false, true] {
        let tag = if listed { "listed" } else { "unlisted" };
        if listed {
            e.ok(&e.owner, &b, "list", json!({ "price": NEAR.to_string() }), 1).await?;
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
        let ta = json!({ "receiver_id": e.buyer.id(), "token_id": token, "approval_id": 0 });
        e.fail(&e.buyer, &coll, "nft_transfer", ta, 1, "ERR_ONLY_BOX").await?;
        e.check(&b, &format!("[{tag}] approved-account style nft_transfer refused")).await?;
        e.fail(&deployer, &coll, "nft_transfer", t, 1, "ERR_ONLY_BOX").await?;
        e.check(&b, &format!("[{tag}] collection deployer-key nft_transfer refused")).await?;
        let approved = e.view(&coll, "nft_is_approved", json!({ "token_id": token, "approved_account_id": e.buyer.id() })).await?;
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
// Test 4 — Stuck lock / rescue, as far as the real chain allows
// ---------------------------------------------------------------------------
/// Fire `rescue_sale(sid)` from several signers at staggered offsets while a
/// real `buy` is in flight. Returns (buy outcome, rescue results).
async fn race(e: &Env, b: &AccountId, buyer: &Account, price: u128, sid: u64, stagger_ms: u64, racers: &[Account]) -> Result<(ExecutionFinalResult, Vec<String>)> {
    let mut handles = vec![];
    for (i, r) in racers.iter().enumerate() {
        let (r, b) = (r.clone(), b.clone());
        let delay = 150 + stagger_ms * i as u64;
        handles.push(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            let res: Result<ExecutionFinalResult> = async { send_once(r.call(&b, "rescue_sale").args_json(json!({ "sale_id": sid })).gas(Gas::from_tgas(100)).transact_async().await?).await }.await;
            (r.id().clone(), res)
        }));
    }
    let buy = e.buy_raw(buyer, b, price).await?;
    let t = e.buy_timing(&buy, b).await?;
    let mut results = vec![];
    for h in handles {
        let (id, res) = h.await?;
        // A racer whose tx was dropped before inclusion (unknown to two RPCs)
        // simply did not take part in the race; it is recorded, not resent
        // (the in-flight moment it was racing is over). Any other error fails.
        let r = match res {
            Ok(r) => r,
            Err(err) if err.to_string().contains("never landed") => {
                let hash = err.to_string().split_whitespace().nth(1).unwrap_or_default().to_string();
                ensure!(e.tx_unknown(&hash, &id).await, "racer {id} tx {hash} not final but known to an RPC: {err}");
                println!("[{}]   race rescue by {id}: tx {hash} dropped before inclusion (unknown to two RPCs) -> dropped", e.test_name);
                let _ = writeln!(e.records.borrow_mut(), "{}", json!({"test": e.test_name, "pid": std::process::id(),
                    "label": "race rescue_sale dropped before inclusion (unknown to two RPCs); not resent", "signer": id,
                    "receiver": b, "method": "rescue_sale", "tx": hash, "ok": false, "landed": false}));
                results.push("dropped".to_string());
                continue;
            }
            Err(err) => return Err(err),
        };
        e.record("race rescue_sale during in-flight buy", &id, b, "rescue_sale", &r);
        let mut blocks = vec![];
        for o in r.receipt_outcomes() {
            if &o.executor_id == b || &o.executor_id == e.cid() {
                blocks.push(e.worker.view_block().block_hash(o.block_hash).await?.height());
            }
        }
        let s = match r.into_result() {
            Ok(v) => format!("ok:{}", v.json::<String>().unwrap_or_default()),
            Err(x) => {
                let s = format!("{x:?}");
                ["ERR_STALE_SALE_ID", "ERR_UNDO_DELAY_NOT_ELAPSED", "ERR_COLLECTION_OWNER_UNEXPECTED", "ERR_SALE_LOCKED"]
                    .iter()
                    .find(|n| s.contains(*n))
                    .map(|n| format!("refused:{n}"))
                    .unwrap_or(format!("other:{}", s.chars().take(200).collect::<String>()))
            }
        };
        println!("[{}]   race rescue by {id} receipts@{blocks:?} (lock@{}, callback@{}) -> {s}", e.test_name, t.started_at_block, t.callback_block);
        results.push(s);
    }
    ensure!(results.iter().any(|r| r != "dropped"), "every racer tx was dropped; no rescue raced the sale");
    Ok((buy, results))
}

async fn t4(e: &Env) -> Result<()> {
    // Spec test 4 creates a *stuck* lock by sandbox state patching (4.1-4.3);
    // that cannot be done on a live chain (no sandbox_patch_state, box has no
    // keys). On testnet we drive the same rescue code against real in-flight
    // sales and prove: rescue never resolves a sale twice, never undoes a
    // sale that is landing, late rescues after finalize/undo are no-ops, and
    // old sale ids are stale.
    let b = e.leased_box(NEAR / 10).await?;
    let token = e.bview(&b, "get_token_id").await?.as_str().unwrap().to_string();
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR / 10).await?;
    e.check(&b, "hire paid").await?;
    let price = NEAR / 2;
    let mut old_ids = vec![];
    let mut tally: std::collections::BTreeMap<String, usize> = Default::default();

    // ---- Round F: failing transfer (collection drained) + concurrent rescues.
    // A fresh buyer has never owned a token, so the transfer needs new storage.
    let fb = e.fresh_buyer("t4b").await?;
    ensure!(e.view(e.cid(), "nft_supply_for_owner", json!({ "account_id": fb.id() })).await? == json!("0"));
    e.ok(&e.owner, &b, "list", json!({ "price": price.to_string() }), 1).await?;
    e.check(&b, "4.F owner listed").await?;
    let sid = e.bview(&b, "get_next_sale_id").await?.as_u64().unwrap();
    let (accrued, reserve) = (e.accrued(&b).await?, e.reserve(&b).await?);
    let f0 = e.fee_balance().await?;
    let drained = drain_collection(e, &e.racer1).await?;
    let racers = [e.racer1.clone(), e.inspector.clone(), e.lessee.clone(), e.buyer.clone()];
    let res = race(e, &b, &fb, price, sid, 350, &racers).await;
    restore_collection(e, &e.racer1, drained).await?;
    let (buy, results) = res?;
    let failures = format!("{:?}", buy.receipt_failures());
    ensure!(failures.contains("LackBalanceForState"), "expected collection transfer to fail: {failures}");
    let outcome: String = buy.into_result().map_err(|x| anyhow!("buy: {x:?}"))?.json()?;
    let rescue_undos = results.iter().filter(|r| *r == "ok:undone").count();
    ensure!((outcome == "undone") as usize + rescue_undos == 1, "exactly one undo: callback {outcome}, rescues {results:?}");
    for r in &results {
        ensure!(["refused:ERR_STALE_SALE_ID", "refused:ERR_UNDO_DELAY_NOT_ELAPSED", "ok:noop", "dropped"].contains(&r.as_str()),
            "unexpected rescue result around a failed transfer: {r}");
        *tally.entry(format!("failed-transfer {r}")).or_default() += 1;
    }
    ensure!(e.box_owner(&b).await? == e.owner.id().as_str(), "seller still owner");
    ensure!(e.collection_owner(&token).await?.as_deref() == Some(e.owner.id().as_str()));
    ensure!(e.pending(&b, &fb).await? == price, "buyer refunded exactly once");
    ensure!(e.pending(&b, &e.owner).await? == 0, "seller not credited");
    ensure!(e.accrued(&b).await? == accrued && e.reserve(&b).await? == reserve);
    ensure!(e.bview(&b, "get_lock").await?.is_null());
    let mut fbal = e.assert_fee(f0, 0, "4.F undone sale: no fee").await?;
    e.check(&b, &format!("4.F sale {sid} transfer failed -> undone once; concurrent rescues {results:?}")).await?;
    old_ids.push(sid);
    let snap = e.raw_state(&b).await?;
    e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid }), 0, "ERR_STALE_SALE_ID").await?;
    e.fail(&e.outsider, &b, "on_nft_transfer", json!({ "sale_id": sid }), 0, "is private").await?;
    ensure!(e.raw_state(&b).await? == snap, "state changed");
    e.check(&b, "4.F after undo: rescue stale, injected callback private, state byte-identical").await?;
    let got = e.withdraw(&fb, &b).await?;
    ensure!(got == price);
    e.check(&b, "4.F buyer withdrew the price").await?;

    // ---- Rounds S: successful sales with concurrent rescues, alternating parties.
    let racers = [e.outsider.clone(), e.racer1.clone(), e.inspector.clone(), e.lessee.clone()];
    let parties = [(&e.owner, &e.buyer), (&e.buyer, &e.owner)];
    for round in 0..4 {
        let (seller, buyer) = parties[round % 2];
        e.ok(seller, &b, "list", json!({ "price": price.to_string() }), 1).await?;
        e.check(&b, &format!("4.S{round}: {} listed", seller.id())).await?;
        let sid = e.bview(&b, "get_next_sale_id").await?.as_u64().unwrap();
        let accrued = e.accrued(&b).await?;
        let seller_pending = e.pending(&b, seller).await?;
        let buyer_pending = e.pending(&b, buyer).await?;
        let (buy, results) = race(e, &b, buyer, price, sid, 300 + 50 * round as u64, &racers).await?;
        let outcome: String = buy.into_result().map_err(|x| anyhow!("buy: {x:?}"))?.json()?;
        let rescue_finals = results.iter().filter(|r| *r == "ok:finalized").count();
        ensure!(
            (outcome == "finalized" && rescue_finals == 0) || (outcome == "noop" && rescue_finals == 1),
            "exactly one finalize: callback {outcome}, rescues {results:?}"
        );
        for r in &results {
            ensure!(["refused:ERR_STALE_SALE_ID", "refused:ERR_UNDO_DELAY_NOT_ELAPSED", "ok:noop", "ok:finalized", "dropped"].contains(&r.as_str()),
                "unexpected rescue result during in-flight sale: {r}");
            *tally.entry(format!("successful-transfer {r}")).or_default() += 1;
        }
        ensure!(e.box_owner(&b).await? == buyer.id().as_str());
        ensure!(e.collection_owner(&token).await?.as_deref() == Some(buyer.id().as_str()));
        ensure!(e.pending(&b, seller).await? == seller_pending + seller_net(price) + accrued, "seller credited 99% of price + accrued");
        ensure!(e.pending(&b, buyer).await? == buyer_pending, "buyer got no refund");
        fbal = e.assert_fee(fbal, fee_cut(price), &format!("4.S{round} sale finalized once: fee charged exactly once")).await?;
        ensure!(e.reserve(&b).await? == reserve, "reserve untouched by sale fees");
        ensure!(e.bview(&b, "get_lock").await?.is_null());
        e.check(&b, &format!("4.S{round}: sale {sid} finalized once; concurrent rescues {results:?}")).await?;
        old_ids.push(sid);
    }
    println!("[test4]   rescue outcomes around real in-flight sales: {tally:?}");
    let _ = writeln!(e.records.borrow_mut(), "{}", json!({"test": e.test_name, "rescue_tally": tally}));

    // ---- 4.4 Old sale ids are stale; injected callbacks are private; no state change.
    let snap = e.raw_state(&b).await?;
    for sid in &old_ids {
        e.fail(&e.outsider, &b, "rescue_sale", json!({ "sale_id": sid }), 0, "ERR_STALE_SALE_ID").await?;
    }
    e.fail(&e.outsider, &b, "on_nft_transfer", json!({ "sale_id": old_ids[1] }), 0, "is private").await?;
    e.fail(&e.outsider, &b, "on_rescue_owner", json!({ "sale_id": old_ids[1] }), 0, "is private").await?;
    ensure!(e.raw_state(&b).await? == snap, "state changed");
    e.check(&b, "4.4 every old sale_id rescue refused as stale; injected callbacks private; state byte-identical").await?;

    // Hire after the last sale accrues to the current owner; everyone withdraws.
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR / 10).await?;
    ensure!(e.accrued(&b).await? == owner_cut(NEAR / 10));
    e.assert_fee(fbal, fee_cut(NEAR / 10), "hire after the sales: 1%").await?;
    e.check(&b, "hire after the sales accrues to the current owner").await?;
    for who in [&e.owner, &e.buyer] {
        e.withdraw(who, &b).await?;
        e.check(&b, &format!("{} withdrew", who.id())).await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5 — Lapsed inspection (real time)
// ---------------------------------------------------------------------------
async fn t5(e: &Env) -> Result<()> {
    let b = e.mint(&e.owner, 30 * DAY).await?;
    e.check(&b, "minted").await?;
    // 300 s: the attest, the offer and the accept must all land while the
    // inspection is valid, even if one of them is dropped and resent (the
    // runner waits 90 s before resending a dropped tx).
    let valid_until = e.now_secs().await? + 300;
    e.ok(&e.inspector, &b, "attest_inspection", json!({ "valid_until": valid_until }), 0).await?;
    e.check(&b, "inspector attested short validity (300 s)").await?;
    e.ok(&e.owner, &b, "start_lease",
        json!({ "lessee": e.lessee.id(), "rate_per_day": (NEAR / 100).to_string(), "duration_secs": 10 * DAY,
                "deposit": (NEAR / 10).to_string() }),
        1).await?;
    ensure!(e.bview(&b, "get_lease").await?.is_null(), "offer does not start the lease");
    e.check(&b, "lease offered (owner attached 1 yocto, no deposit held)").await?;
    // Only the named lessee, with exactly the offered deposit.
    e.fail(&e.owner, &b, "accept_lease", json!({}), NEAR / 10, "ERR_NOT_LESSEE").await?;
    e.fail(&e.outsider, &b, "accept_lease", json!({}), NEAR / 10, "ERR_NOT_LESSEE").await?;
    e.fail(&e.lessee, &b, "accept_lease", json!({}), NEAR / 10 - 1, "ERR_WRONG_DEPOSIT").await?;
    e.fail(&e.lessee, &b, "accept_lease", json!({}), NEAR / 10 + 1, "ERR_WRONG_DEPOSIT").await?;
    ensure!(e.bview(&b, "get_lease").await?.is_null(), "no lease after refused accepts");
    e.check(&b, "accept by owner/outsider or with a wrong deposit refused").await?;
    // The deposit comes from the lessee.
    let lessee0 = e.balance(e.lessee.id()).await?;
    e.ok(&e.lessee, &b, "accept_lease", json!({}), NEAR / 10).await?;
    // Unused prepaid gas (150 Tgas) is refunded in a later receipt, so let
    // it land, then allow up to 0.25 NEAR of gas on top of the deposit. The
    // exact check is the box-side `held_lessee_deposits` below.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let lessee1 = e.balance(e.lessee.id()).await?;
    println!("[{}]   lessee balance drop on accept: {} NEAR (deposit 0.1 + gas)", e.test_name, fmt_near(lessee0.saturating_sub(lessee1)));
    ensure!(lessee0 >= lessee1 + NEAR / 10 && lessee0 < lessee1 + NEAR / 10 + NEAR / 4,
        "lessee paid the deposit ({lessee0} -> {lessee1})");
    ensure!(e.bview(&b, "get_invariant_parts").await?["held_lessee_deposits"] == json!((NEAR / 10).to_string()),
        "box holds the lessee deposit");
    e.check(&b, "lessee accepted lease while inspection valid (posted deposit)").await?;
    // Wait in real time past valid_until, then read get_inspection *pinned to
    // a block height whose timestamp is past valid_until*. An unpinned
    // (optimistic, load-balanced) view can be served by a lagging RPC node
    // whose head is still inside the window; that is what the first run hit.
    // The contract is unchanged: valid_now = block_timestamp_secs <= valid_until.
    let started = std::time::Instant::now();
    loop {
        ensure!(started.elapsed().as_secs() < 600, "no block past valid_until {valid_until} showed valid_now=false within 600 s");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let blk = match e.worker.view_block().await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let (h, ts) = (blk.height(), blk.timestamp() / 1_000_000_000);
        if ts <= valid_until {
            continue;
        }
        let v: Value = match e.worker.view(&b, "get_inspection").args_json(json!({})).block_height(h).await {
            Ok(r) => r.json()?,
            Err(_) => continue, // node serving the view has not got block h yet
        };
        println!("[{}]   get_inspection pinned at block {h} (ts {ts} > valid_until {valid_until}): {v}", e.test_name);
        ensure!(v["valid_now"] == json!(false), "valid_now true at block {h} ts {ts} > valid_until {valid_until}: {v}");
        break;
    }
    e.check(&b, "waited (real time) past valid_until; valid_now=false at a block past it").await?;

    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), NEAR / 10).await?;
    e.assert_fee(f0, fee_cut(NEAR / 10), "hire after lapse: 1%").await?;
    ensure!(e.reserve(&b).await? == reserve_cut(NEAR / 10) && e.accrued(&b).await? == owner_cut(NEAR / 10));
    e.check(&b, "pay_hire on active lease after lapse succeeds").await?;

    e.ok(&e.lessee, &b, "off_hire", json!({}), 1).await?;
    e.check(&b, "lessee off-hired (deposit credited)").await?;
    ensure!(e.pending(&b, &e.owner).await? == 0, "owner is credited nothing from the deposit");
    let got = e.withdraw(&e.lessee, &b).await?;
    ensure!(got == NEAR / 10, "deposit returned to the lessee: exactly what the lessee paid");
    e.check(&b, "lessee withdrew its own deposit").await?;
    let sl = json!({ "lessee": e.lessee.id(), "rate_per_day": (NEAR / 100).to_string(), "duration_secs": DAY,
                     "deposit": (NEAR / 10).to_string() });
    // The old attack: the owner can no longer attach NEAR as the deposit.
    e.fail(&e.owner, &b, "start_lease", sl.clone(), NEAR / 10, "Requires attached deposit of exactly 1 yoctoNEAR").await?;
    e.check(&b, "start_lease with owner-attached NEAR refused").await?;
    e.fail(&e.owner, &b, "start_lease", sl.clone(), 1, "ERR_INSPECTION_LAPSED").await?;
    e.check(&b, "start_lease refused: inspection lapsed").await?;

    let future = e.now_secs().await? + 30 * DAY;
    for who in [&e.owner, &e.buyer, &e.lessee] {
        e.fail(who, &b, "attest_inspection", json!({ "valid_until": future }), 0, "ERR_NOT_INSPECTOR").await?;
    }
    e.check(&b, "owner/buyer/lessee attest refused").await?;

    e.ok(&e.inspector, &b, "attest_inspection", json!({ "valid_until": future }), 0).await?;
    e.check(&b, "inspector re-attested").await?;
    e.ok(&e.owner, &b, "start_lease", sl, 1).await?;
    e.check(&b, "start_lease succeeds after re-attestation").await?;
    e.ok(&e.lessee, &b, "accept_lease", json!({}), NEAR / 10).await?;
    e.check(&b, "lessee accepted the new lease").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 6 — Reserve cut and owner cannot take it
// ---------------------------------------------------------------------------
async fn t6(e: &Env) -> Result<()> {
    let b = e.leased_box(NEAR / 10).await?;
    let x: u128 = 123_456_789_012_345_678_901_234;
    let r0 = e.reserve(&b).await?;
    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), x).await?;
    e.check(&b, "lessee paid X").await?;
    let reserve = e.reserve(&b).await?;
    ensure!(reserve - r0 == x * 1500 / 10_000, "reserve cut is the full 15% (fee not taken from it)");
    ensure!(e.accrued(&b).await? == x - x * 1500 / 10_000 - x * 100 / 10_000, "owner accrues 85% - 1% fee");
    e.assert_fee(f0, x * 100 / 10_000, "hire X: 1% of the payment").await?;
    println!("[test6]   X={} reserve+={} fee={} accrued={}", x, reserve - r0, fee_cut(x), owner_cut(x));

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
    e.check(&b, "nobody can withdraw the reserve; invariant holds").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 7 — 1% protocol fee: exact 100-yocto splits on the real chain
// ---------------------------------------------------------------------------
async fn t7(e: &Env) -> Result<()> {
    ensure!(e.view(e.cid(), "get_fee_account", json!({})).await? == json!(e.fee), "collection fee account");
    let b = e.leased_box(NEAR / 10).await?;
    let c = e.bview(&b, "get_constants").await?;
    ensure!(c["FEE_BPS"] == json!(100) && c["RESERVE_BPS"] == json!(1500) && c["SALE_UNDO_DELAY_BLOCKS"] == json!(32), "constants {c}");
    ensure!(c["BPS_DENOMINATOR"] == json!("10000") && c["FEE_ACCOUNT"] == json!(e.fee), "constants {c}");
    println!("[test7]   box constants: {c}");
    e.check(&b, "constants: FEE_BPS 100, RESERVE_BPS 1500, delay 32, fee account").await?;

    let f0 = e.fee_balance().await?;
    e.ok(&e.lessee, &b, "pay_hire", json!({}), 100).await?;
    ensure!(e.reserve(&b).await? == 15, "reserve 15");
    ensure!(e.accrued(&b).await? == 84, "owner 84");
    let f1 = e.assert_fee(f0, 1, "hire 100 yocto -> reserve 15 / fee 1 / owner 84").await?;
    e.check(&b, "hire 100 yocto split 15/1/84").await?;

    e.ok(&e.owner, &b, "list", json!({ "price": "100" }), 1).await?;
    e.check(&b, "listed at 100 yocto").await?;
    ensure!(e.buy(&e.buyer, &b, 100).await? == "finalized");
    ensure!(e.reserve(&b).await? == 15, "reserve untouched by the sale");
    ensure!(e.pending(&b, &e.owner).await? == 99 + 84, "seller 99 + pre-sale hire 84");
    e.assert_fee(f1, 1, "sale 100 yocto -> fee 1 / seller 99").await?;
    ensure!(e.fees_paid(&b).await? == 2);
    e.check(&b, "sale 100 yocto split 1/99").await?;
    ensure!(e.withdraw(&e.owner, &b).await? == 99 + 84);
    e.check(&b, "seller withdrew 99 + 84 yocto").await?;
    Ok(())
}

/// Broadcast once (async) and poll for the final outcome. Unlike
/// `transact()`, this never re-signs and resends a transaction whose
/// `broadcast_tx_commit` merely timed out at the RPC, which on testnet can
/// otherwise put a duplicate transaction on chain.
async fn send_once(st: near_workspaces::operations::TransactionStatus) -> Result<ExecutionFinalResult> {
    let started = std::time::Instant::now();
    let mut errors = 0;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        match st.status().await {
            Ok(std::task::Poll::Ready(r)) => return Ok(r),
            Ok(std::task::Poll::Pending) => {}
            Err(e) => {
                errors += 1;
                if errors > 30 {
                    bail!("tx {} status polling failed: {e}", st.hash());
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
        if started.elapsed().as_secs() > 90 {
            bail!("tx {} not final after 90 s (never landed)", st.hash());
        }
    }
}

/// Refuse to run two copies at once (they would share signer keys).
struct RunLock(std::path::PathBuf);
impl RunLock {
    fn take() -> Result<Self> {
        let p = keys_dir().join("run.lock");
        if let Ok(old) = std::fs::read_to_string(&p) {
            let pid = old.trim();
            if std::path::Path::new(&format!("/proc/{pid}")).exists() {
                bail!("another onhire-testnet run (pid {pid}) holds {p:?}");
            }
        }
        std::fs::write(&p, std::process::id().to_string())?;
        Ok(Self(p))
    }
}
impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let _lock = RunLock::take()?;
    println!("[onhire-testnet] pid {} cmd {cmd}", std::process::id());
    if cmd == "fee-setup" {
        let rpc = std::env::var("NEAR_TESTNET_RPC").unwrap_or_else(|_| "https://rpc.testnet.near.org".into());
        return fee_setup(&near_workspaces::testnet().rpc_addr(&rpc).await?).await;
    }
    let e = Env::new(&format!("testnet-{cmd}")).await?;
    let start = std::time::Instant::now();
    let r = match cmd.as_str() {
        "fund" => fund(&e).await,
        "deploy" => deploy(&e).await,
        "topup" => topup(&e).await,
        "t1" => t1(&e).await,
        "t2" => t2(&e).await,
        "t3" => t3(&e).await,
        "t4" => t4(&e).await,
        "t5" => t5(&e).await,
        "t6" => t6(&e).await,
        "t7" => t7(&e).await,
        _ => bail!("usage: onhire-testnet fee-setup|fund|deploy|topup|t1|t2|t3|t4|t5|t6|t7"),
    };
    let t = e.timings.borrow().clone();
    if !t.is_empty() {
        let max = t.iter().map(|x| x.callback_block.saturating_sub(x.started_at_block)).max().unwrap();
        println!("[{}] buy timings: max callback - lock = {max} blocks (SALE_UNDO_DELAY_BLOCKS = {SALE_UNDO_DELAY_BLOCKS})", e.test_name);
    }
    match &r {
        Ok(()) => println!("[{}] RESULT: PASS ({} checked steps, {:.0} s)", e.test_name, e.step.get(), start.elapsed().as_secs_f64()),
        Err(err) => println!("[{}] RESULT: FAIL after {} checked steps: {err:#}", e.test_name, e.step.get()),
    }
    r
}
