// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_crypto::HashValue;
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::state_store::state_summary::StateSummary;
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Deref)]
pub struct StateCheckpointOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl StateCheckpointOutput {
    pub fn new(
        last_state_checkpoint_summary: Option<StateSummary>,
        result_state_summary: StateSummary,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> Self {
        Self::new_impl(Inner {
            last_state_checkpoint_summary,
            result_state_summary,
            state_checkpoint_hashes,
        })
    }

    pub fn new_empty(parent_state_summary: StateSummary) -> Self {
        Self::new_impl(Inner {
            last_state_checkpoint_summary: None,
            result_state_summary: parent_state_summary,
            state_checkpoint_hashes: vec![],
        })
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
        Self::new_empty(self.result_state_summary.clone())
    }
}

#[derive(Debug)]
pub struct Inner {
    pub last_state_checkpoint_summary: Option<StateSummary>,
    pub result_state_summary: StateSummary,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
}
