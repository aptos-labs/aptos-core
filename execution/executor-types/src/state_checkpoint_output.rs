// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_crypto::HashValue;
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::state_store::{
    sharded_state_updates::ShardedStateUpdates, state_delta::StateDelta,
    state_summary::StateSummary,
};
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Deref)]
pub struct StateCheckpointOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl StateCheckpointOutput {
    pub fn new(
        parent_state: Arc<StateDelta>,
        result_state: Arc<StateDelta>,
        state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> Self {
        Self::new_impl(Inner {
            parent_state,
            result_state,
            state_updates_before_last_checkpoint,
            state_checkpoint_hashes,
        })
    }

    pub fn new_empty(_state_summary: StateSummary) -> Self {
        todo!()
        /* FIXME(aldenhu)
        Self::new_impl(Inner {
            parent_state: state.clone(),
            result_state: state,
            state_updates_before_last_checkpoint: None,
            state_checkpoint_hashes: vec![],
        })
         */
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(StateSummary::new_empty())
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        /* FIXME(aldenhu)
        Self::new_empty(self.result_state.clone())
         */
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct Inner {
    pub parent_state: Arc<StateDelta>,
    pub result_state: Arc<StateDelta>,
    pub state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
}
