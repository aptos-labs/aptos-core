// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod rest_interface;
mod storage_interface;

pub use crate::{rest_interface::RestDebuggerInterface, storage_interface::DBDebuggerInterface};
use anyhow::Result;
use aptos_framework::natives::code::PackageMetadata;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewId, StateViewResult, TStateView,
    },
    transaction::{Transaction, TransactionInfo, Version},
};
use lru::LruCache;
use move_core_types::language_storage::ModuleId;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Clone, Copy)]
pub struct FilterCondition {
    pub skip_failed_txns: bool,
    pub skip_publish_txns: bool,
    pub check_source_code: bool,
    pub target_account: Option<AccountAddress>,
}

// TODO(skedia) Clean up this interfact to remove account specific logic and move to state store
// key-value interface with fine grained storage project
#[async_trait::async_trait]
pub trait AptosValidatorInterface: Sync {
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

    async fn get_and_filter_committed_transactions(
        &self,
        start: Version,
        limit: u64,
        filter_condition: FilterCondition,
        package_cache: &mut HashMap<
            ModuleId,
            (
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            ),
        >,
    ) -> Result<
        Vec<(
            u64,
            Transaction,
            Option<(
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            )>,
        )>,
    >;

    async fn get_latest_ledger_info_version(&self) -> Result<Version>;

    async fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>>;
}

pub struct DebuggerStateView {
    query_sender: Mutex<
        UnboundedSender<(
            StateKey,
            Version,
            std::sync::mpsc::Sender<Result<Option<StateValue>>>,
        )>,
    >,
    version: Version,
}

async fn handler_thread(
    db: Arc<dyn AptosValidatorInterface + Send>,
    mut thread_receiver: UnboundedReceiver<(
        StateKey,
        Version,
        std::sync::mpsc::Sender<Result<Option<StateValue>>>,
    )>,
) {
    const M: NonZeroUsize = NonZeroUsize::new(1024 * 1024).unwrap();
    let cache = Arc::new(Mutex::new(LruCache::<
        (StateKey, Version),
        Option<StateValue>,
    >::new(M)));
    loop {
        let (key, version, sender) =
            if let Some((key, version, sender)) = thread_receiver.recv().await {
                (key, version, sender)
            } else {
                break;
            };
        if let Some(val) = cache.lock().unwrap().get(&(key.clone(), version)) {
            sender.send(Ok(val.clone())).unwrap();
        } else {
            assert!(version > 0, "Expecting a non-genesis version");
            let db = db.clone();
            let cache = cache.clone();
            tokio::spawn(async move {
                let res = db.get_state_value_by_version(&key, version - 1).await;
                match res {
                    Ok(val) => {
                        cache.lock().unwrap().put((key, version), val.clone());
                        sender.send(Ok(val))
                    },
                    Err(err) => sender.send(Err(err)),
                }
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
    ) -> Result<Option<StateValue>> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.query_sender
            .lock()
            .unwrap()
            .send((state_key.clone(), version, tx))
            .unwrap();
        rx.recv()?
    }
}

impl TStateView for DebuggerStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        StateViewId::Replay
    }

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<Option<StateValue>> {
        self.get_state_value_internal(state_key, self.version)
            .map_err(Into::into)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        unimplemented!()
    }

    fn next_version(&self) -> Version {
        self.version + 1
    }
}
