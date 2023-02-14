// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod rest_interface;
mod storage_interface;

pub use crate::{rest_interface::RestDebuggerInterface, storage_interface::DBDebuggerInterface};
use anyhow::{anyhow, Result};
use aptos_state_view::TStateView;
use aptos_types::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    account_state::AccountState,
    account_view::AccountView,
    on_chain_config::ValidatorSet,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::{Transaction, TransactionInfo, Version},
};
use lru::LruCache;
use move_binary_format::file_format::CompiledModule;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

// TODO(skedia) Clean up this interfact to remove account specific logic and move to state store
// key-value interface with fine grained storage project
#[async_trait::async_trait]
pub trait AptosValidatorInterface: Sync {
    async fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>>;

    async fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>>;

    async fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)>;

    async fn get_latest_version(&self) -> Result<Version>;

    async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>>;

    async fn get_framework_modules_by_version(
        &self,
        version: Version,
    ) -> Result<Vec<CompiledModule>> {
        let mut acc = vec![];
        for module_bytes in self
            .get_account_state_by_version(CORE_CODE_ADDRESS, version)
            .await?
            .ok_or_else(|| anyhow!("Failure reading aptos root address state"))?
            .get_modules()
        {
            acc.push(
                CompiledModule::deserialize(module_bytes)
                    .map_err(|e| anyhow!("Failure deserializing module: {:?}", e))?,
            )
        }
        Ok(acc)
    }

    /// Get the account states of the most critical accounts, including:
    /// 1. Aptos Framework code address
    /// 2. All validator addresses
    async fn get_admin_accounts(
        &self,
        version: Version,
    ) -> Result<Vec<(AccountAddress, AccountState)>> {
        let mut result = vec![];
        let aptos_framework = self
            .get_account_state_by_version(CORE_CODE_ADDRESS, version)
            .await?
            .ok_or_else(|| anyhow!("Aptos framework account doesn't exist"))?;

        // Get all validator accounts
        let validators = aptos_framework
            .get_config::<ValidatorSet>()?
            .ok_or_else(|| anyhow!("validator_config doesn't exist"))?;

        // Get code account
        result.push((
            CORE_CODE_ADDRESS,
            self.get_account_state_by_version(CORE_CODE_ADDRESS, version)
                .await?
                .ok_or_else(|| anyhow!("core_code_address doesn't exist"))?,
        ));

        // Get all validator accounts
        for validator_info in validators.payload() {
            let addr = *validator_info.account_address();
            result.push((
                addr,
                self.get_account_state_by_version(addr, version)
                    .await?
                    .ok_or_else(|| anyhow!("validator {:?} doesn't exist", addr))?,
            ));
        }
        Ok(result)
    }
}

pub struct DebuggerStateView {
    query_sender:
        Mutex<UnboundedSender<(StateKey, Version, std::sync::mpsc::Sender<Option<Vec<u8>>>)>>,
    version: Version,
}

async fn handler_thread<'a>(
    db: Arc<dyn AptosValidatorInterface + Send>,
    mut thread_receiver: UnboundedReceiver<(
        StateKey,
        Version,
        std::sync::mpsc::Sender<Option<Vec<u8>>>,
    )>,
) {
    const M: usize = 1024 * 1024;
    let cache = Arc::new(Mutex::new(
        LruCache::<(StateKey, Version), Option<Vec<u8>>>::new(M),
    ));

    loop {
        let (key, version, sender) =
            if let Some((key, version, sender)) = thread_receiver.recv().await {
                (key, version, sender)
            } else {
                break;
            };

        if let Some(val) = cache.lock().unwrap().get(&(key.clone(), version)) {
            sender.send(val.clone()).unwrap();
        } else {
            assert!(version > 0, "Expecting a non-genesis version");
            let db = db.clone();
            let cache = cache.clone();
            tokio::spawn(async move {
                let val = db
                    .get_state_value_by_version(&key, version - 1)
                    .await
                    .ok()
                    .and_then(|v| v.map(|s| s.into_bytes()));
                cache.lock().unwrap().put((key, version), val.clone());
                sender.send(val)
            });
        }
    }
}

impl DebuggerStateView {
    pub fn new(db: Arc<dyn AptosValidatorInterface + Send>, version: Version) -> Self {
        let (query_sender, thread_receiver) = unbounded_channel();

        tokio::spawn(async move { handler_thread(db, thread_receiver).await });
        Self {
            query_sender: Mutex::new(query_sender),
            version,
        }
    }

    fn get_state_value_internal(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<Vec<u8>>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let query_handler_locked = self.query_sender.lock().unwrap();
        query_handler_locked
            .send((state_key.clone(), version, tx))
            .unwrap();
        Ok(rx.recv()?)
    }
}

impl TStateView for DebuggerStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        self.get_state_value_internal(state_key, self.version)
    }

    fn is_genesis(&self) -> bool {
        false
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        unimplemented!()
    }
}
