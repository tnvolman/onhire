//! Onhire collection: NEP-171 registry (with NEP-177 metadata and NEP-181
//! enumeration) and factory for keyless box accounts.
//!
//! Every transfer path is gated to the box account for that token. Approvals
//! are disabled. There is no marketplace.
//!
//! Spec: docs/ONHIRE-SPEC-v1.1.md. All rights reserved; see LICENSE.

use near_contract_standards::non_fungible_token::events::{NftMint, NftTransfer};
use near_contract_standards::non_fungible_token::metadata::{NFTContractMetadata, TokenMetadata};
use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::store::{IterableSet, LookupMap};
use near_sdk::{
    env, near, require, AccountId, BorshStorageKey, CryptoHash, Gas, NearToken, PanicOnDefault,
    Promise, PromiseOrValue,
};

pub type TokenId = String;

/// Minimum NEAR the minter must attach so the box can stake its own storage
/// plus headroom. Code staking is extra when the item code is not global.
pub const MIN_BOX_DEPOSIT_GLOBAL: NearToken = NearToken::from_millinear(100);
const GAS_FOR_BOX_INIT: Gas = Gas::from_tgas(20);
const GAS_FOR_ON_BOX_CREATED: Gas = Gas::from_tgas(20);
const CODE_KEY: &[u8] = b"ITEM_CODE";

pub mod err {
    pub const ONLY_BOX: &str =
        "ERR_ONLY_BOX: transfers must be made by the box account for this token (use buy on the box)";
    pub const APPROVALS_DISABLED: &str = "ERR_APPROVALS_DISABLED: no account can be approved to move a box";
    pub const TRANSFER_CALL_DISABLED: &str = "ERR_TRANSFER_CALL_DISABLED: boxes move only via nft_transfer";
    pub const APPROVAL_ID: &str = "ERR_APPROVAL_ID_NOT_SUPPORTED";
    pub const NO_TOKEN: &str = "ERR_TOKEN_NOT_FOUND";
    pub const SAME_OWNER: &str = "ERR_RECEIVER_IS_OWNER";
    pub const NOT_MINTER: &str = "ERR_NOT_MINTER";
    pub const CODE_SET: &str = "ERR_ITEM_CODE_ALREADY_SET";
    pub const NO_CODE: &str = "ERR_ITEM_CODE_NOT_SET";
    pub const DEPOSIT: &str = "ERR_MINT_DEPOSIT_TOO_SMALL";
    pub const BAD_PARTIES: &str = "ERR_INSPECTOR_MUST_DIFFER_FROM_OWNER";
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CodeMode {
    /// NEP-591 global contract, referenced by immutable code hash.
    Global,
    /// Fallback: code bytes stored on the collection and deployed per box.
    Local,
}

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    OwnerById,
    TokenMetadata,
    TokensPerOwner,
    TokensPerOwnerInner { account_hash: CryptoHash },
    TokenIds,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Collection {
    /// Deployer / authorized minter for the prototype.
    pub minter: AccountId,
    pub metadata: NFTContractMetadata,
    pub next_id: u64,
    pub owner_by_id: LookupMap<TokenId, AccountId>,
    pub token_metadata: LookupMap<TokenId, TokenMetadata>,
    pub tokens_per_owner: LookupMap<AccountId, IterableSet<TokenId>>,
    pub token_ids: IterableSet<TokenId>,
    pub item_code_hash: Option<Base64VecU8>,
    pub item_code_mode: Option<CodeMode>,
    /// Fixed at init and written into every box it mints; each box sends its
    /// 1% protocol fee (hire and sale) straight here.
    pub fee_account: AccountId,
}

#[near]
impl Collection {
    #[init]
    pub fn new(minter: AccountId, fee_account: AccountId) -> Self {
        Self {
            minter,
            metadata: NFTContractMetadata {
                spec: "nft-1.0.0".to_string(),
                name: "Onhire containers".to_string(),
                symbol: "ONHIRE".to_string(),
                icon: None,
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
            next_id: 1,
            owner_by_id: LookupMap::new(StorageKey::OwnerById),
            token_metadata: LookupMap::new(StorageKey::TokenMetadata),
            tokens_per_owner: LookupMap::new(StorageKey::TokensPerOwner),
            token_ids: IterableSet::new(StorageKey::TokenIds),
            item_code_hash: None,
            item_code_mode: None,
            fee_account,
        }
    }

    /// Set the box code once. `global = true` deploys it as a NEP-591 global
    /// contract identified by its hash (immutable, so boxes have no upgrade
    /// path). `global = false` is the documented fallback: the bytes are
    /// stored here and deployed into each box.
    pub fn set_item_code(&mut self, code: Base64VecU8, global: bool) -> Option<Promise> {
        self.assert_minter();
        require!(self.item_code_mode.is_none(), err::CODE_SET);
        let hash = env::sha256_array(&code.0);
        self.item_code_hash = Some(Base64VecU8(hash.to_vec()));
        if global {
            self.item_code_mode = Some(CodeMode::Global);
            Some(Promise::new(env::current_account_id()).deploy_global_contract(code.0))
        } else {
            self.item_code_mode = Some(CodeMode::Local);
            env::storage_write(CODE_KEY, &code.0);
            None
        }
    }

    /// 5.7 — create `box-{id}.{collection}` with no access keys, deploy the
    /// box code, init it, and register the token once that succeeds.
    #[payable]
    pub fn mint(
        &mut self,
        owner: AccountId,
        inspector: AccountId,
        inspection_valid_until: u64,
    ) -> Promise {
        self.assert_minter();
        require!(owner != inspector, err::BAD_PARTIES);
        let mode = self.item_code_mode.unwrap_or_else(|| env::panic_str(err::NO_CODE));
        let deposit = env::attached_deposit();
        require!(deposit >= MIN_BOX_DEPOSIT_GLOBAL, err::DEPOSIT);

        let id = self.next_id;
        self.next_id += 1;
        let token_id = format!("{:04}", id);
        let box_id = self.box_account_for(token_id.clone());

        // A freshly created sub-account has no access keys. We never add one,
        // so the key set is empty from the first block: no key to delete,
        // no path to add one, upgrade, or delete the account.
        let mut p = Promise::new(box_id).create_account().transfer(deposit);
        p = match mode {
            CodeMode::Global => {
                let h: [u8; 32] = self.item_code_hash.clone().unwrap().0.try_into().unwrap();
                p.use_global_contract(h)
            }
            CodeMode::Local => p.deploy_contract(env::storage_read(CODE_KEY).expect("code")),
        };
        let args = near_sdk::serde_json::json!({
            "collection": env::current_account_id(),
            "token_id": token_id,
            "owner": owner,
            "inspector": inspector,
            "inspection_valid_until": inspection_valid_until,
            "fee_account": self.fee_account,
        });
        p.function_call(
            "new".to_string(),
            args.to_string().into_bytes(),
            NearToken::from_yoctonear(0),
            GAS_FOR_BOX_INIT,
        )
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_ON_BOX_CREATED)
                .on_box_created(token_id, owner, env::predecessor_account_id(), U128(deposit.as_yoctonear())),
        )
    }

    #[private]
    pub fn on_box_created(
        &mut self,
        token_id: TokenId,
        owner: AccountId,
        minter: AccountId,
        deposit: U128,
    ) -> Option<TokenId> {
        let ok = matches!(env::promise_result_checked(0, 0), Ok(_));
        if !ok {
            env::log_str(&format!("box-{} creation failed; refunding minter", token_id));
            Promise::new(minter).transfer(NearToken::from_yoctonear(deposit.0)).detach();
            return None;
        }
        self.owner_by_id.insert(token_id.clone(), owner.clone());
        self.token_metadata.insert(
            token_id.clone(),
            TokenMetadata {
                title: Some(format!("Container box-{}", token_id)),
                description: Some(
                    "Onhire shipping container. Lease payments, the repair reserve, inspection and sale are enforced by its own keyless account."
                        .to_string(),
                ),
                media: None,
                media_hash: None,
                copies: Some(1),
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: Some(format!(
                    "{{\"box_account\":\"{}\"}}",
                    self.box_account_for(token_id.clone())
                )),
                reference: None,
                reference_hash: None,
            },
        );
        self.add_to_owner(&owner, &token_id);
        self.token_ids.insert(token_id.clone());
        NftMint { owner_id: &owner, token_ids: &[&token_id], memo: None }.emit();
        Some(token_id)
    }

    // ------------------------------------------------------------- NEP-171

    /// Only the box account for `token_id` may move it (rule 8).
    #[payable]
    pub fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u32>,
        memo: Option<String>,
    ) {
        self.assert_box_caller(&token_id);
        near_sdk::assert_one_yocto();
        require!(approval_id.is_none(), err::APPROVAL_ID);
        let old = self.owner_by_id.get(&token_id).cloned().unwrap_or_else(|| env::panic_str(err::NO_TOKEN));
        require!(old != receiver_id, err::SAME_OWNER);
        self.remove_from_owner(&old, &token_id);
        self.add_to_owner(&receiver_id, &token_id);
        self.owner_by_id.insert(token_id.clone(), receiver_id.clone());
        let caller = env::predecessor_account_id();
        NftTransfer {
            old_owner_id: &old,
            new_owner_id: &receiver_id,
            token_ids: &[&token_id],
            authorized_id: Some(&caller),
            memo: memo.as_deref(),
        }
        .emit();
    }

    /// Gated to the box like `nft_transfer`; the box never uses it, so it
    /// always fails in this version.
    #[payable]
    pub fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u32>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        let _ = (receiver_id, approval_id, memo, msg);
        self.assert_box_caller(&token_id);
        env::panic_str(err::TRANSFER_CALL_DISABLED)
    }

    pub fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        let owner_id = self.owner_by_id.get(&token_id)?.clone();
        Some(Token {
            metadata: self.token_metadata.get(&token_id).cloned(),
            token_id,
            owner_id,
            approved_account_ids: Some(Default::default()),
        })
    }

    // ------------------------------------------------- approvals (disabled)

    #[payable]
    pub fn nft_approve(&mut self, token_id: TokenId, account_id: AccountId, msg: Option<String>) {
        let _ = (token_id, account_id, msg);
        env::panic_str(err::APPROVALS_DISABLED)
    }
    #[payable]
    pub fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
        let _ = (token_id, account_id);
        env::panic_str(err::APPROVALS_DISABLED)
    }
    #[payable]
    pub fn nft_revoke_all(&mut self, token_id: TokenId) {
        let _ = token_id;
        env::panic_str(err::APPROVALS_DISABLED)
    }
    pub fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool {
        let _ = (token_id, approved_account_id, approval_id);
        false
    }

    // ------------------------------------------------- NEP-177 / NEP-181

    pub fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.clone()
    }
    pub fn nft_total_supply(&self) -> U128 {
        U128(self.token_ids.len() as u128)
    }
    pub fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        let start = from_index.map(|i| i.0 as usize).unwrap_or(0);
        let limit = limit.unwrap_or(50) as usize;
        self.token_ids
            .iter()
            .skip(start)
            .take(limit)
            .filter_map(|t| self.nft_token(t.clone()))
            .collect()
    }
    pub fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        U128(self.tokens_per_owner.get(&account_id).map(|s| s.len() as u128).unwrap_or(0))
    }
    pub fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        let Some(set) = self.tokens_per_owner.get(&account_id) else { return vec![] };
        let start = from_index.map(|i| i.0 as usize).unwrap_or(0);
        let limit = limit.unwrap_or(50) as usize;
        set.iter().skip(start).take(limit).filter_map(|t| self.nft_token(t.clone())).collect()
    }

    // ---------------------------------------------------------- Onhire views

    /// Used by `rescue_sale` on the box.
    pub fn owner_of(&self, token_id: TokenId) -> Option<AccountId> {
        self.owner_by_id.get(&token_id).cloned()
    }
    pub fn box_account_for(&self, token_id: TokenId) -> AccountId {
        format!("box-{}.{}", token_id, env::current_account_id())
            .parse()
            .unwrap_or_else(|_| env::panic_str("ERR_BAD_TOKEN_ID"))
    }
    pub fn get_item_code_info(&self) -> (Option<Base64VecU8>, Option<CodeMode>) {
        (self.item_code_hash.clone(), self.item_code_mode)
    }
    pub fn get_minter(&self) -> AccountId {
        self.minter.clone()
    }
    pub fn get_fee_account(&self) -> AccountId {
        self.fee_account.clone()
    }
}

impl Collection {
    fn assert_minter(&self) {
        require!(env::predecessor_account_id() == self.minter, err::NOT_MINTER);
    }

    fn assert_box_caller(&self, token_id: &TokenId) {
        require!(self.owner_by_id.contains_key(token_id), err::NO_TOKEN);
        require!(
            env::predecessor_account_id() == self.box_account_for(token_id.clone()),
            err::ONLY_BOX
        );
    }

    fn add_to_owner(&mut self, owner: &AccountId, token_id: &TokenId) {
        if !self.tokens_per_owner.contains_key(owner) {
            let set = IterableSet::new(StorageKey::TokensPerOwnerInner {
                account_hash: env::sha256_array(owner.as_bytes()),
            });
            self.tokens_per_owner.insert(owner.clone(), set);
        }
        self.tokens_per_owner.get_mut(owner).unwrap().insert(token_id.clone());
    }

    fn remove_from_owner(&mut self, owner: &AccountId, token_id: &TokenId) {
        if let Some(set) = self.tokens_per_owner.get_mut(owner) {
            set.remove(token_id);
            if set.is_empty() {
                self.tokens_per_owner.remove(owner);
            }
        }
    }
}
