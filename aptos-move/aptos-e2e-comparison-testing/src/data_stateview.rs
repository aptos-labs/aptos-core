// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_language_e2e_tests::data_store::FakeDataStore;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        Result as StateViewResult, TStateView,
    },
    transaction::Version,
};
use aptos_validator_interface::AptosValidatorInterface;
use lru::LruCache;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct DataStateView {
    query_sender: Mutex<
        UnboundedSender<(
            StateKey,
            Version,
            std::sync::mpsc::Sender<Result<Option<StateValue>>>,
        )>,
    >,
    version: Version,
    code_data: FakeDataStore,
}

async fn handler_thread<'a>(
    db: Arc<dyn AptosValidatorInterface + Send>,
    mut thread_receiver: UnboundedReceiver<(
        StateKey,
        Version,
        std::sync::mpsc::Sender<Result<Option<StateValue>>>,
    )>,
) {
    const M: usize = 1024 * 1024;
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

impl DataStateView {
    pub fn new(
        db: Arc<dyn AptosValidatorInterface + Send>,
        version: Version,
        code_data: FakeDataStore,
    ) -> Self {
        let (query_sender, thread_receiver) = unbounded_channel();
        tokio::spawn(async move { handler_thread(db, thread_receiver).await });
        Self {
            query_sender: Mutex::new(query_sender),
            version,
            code_data,
        }
    }

    fn get_state_value_internal(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        if self.code_data.contains_key(state_key) {
            return self
                .code_data
                .get_state_value(state_key)
                .map_err(Into::into);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let query_handler_locked = self.query_sender.lock().unwrap();
        query_handler_locked
            .send((state_key.clone(), version, tx))
            .unwrap();
        rx.recv()?
    }
}

impl TStateView for DataStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<Option<StateValue>> {
        self.get_state_value_internal(state_key, self.version)
            .map_err(Into::into)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        unimplemented!()
    }
}
