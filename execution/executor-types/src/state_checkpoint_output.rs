// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_crypto::HashValue;
use velor_drop_helper::DropHelper;
use velor_storage_interface::state_store::state_summary::LedgerStateSummary;
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Deref)]
pub struct StateCheckpointOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl StateCheckpointOutput {
    pub fn new(
        state_summary: LedgerStateSummary,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> Self {
        Self::new_impl(Inner {
            state_summary,
            state_checkpoint_hashes,
        })
    }

    pub fn new_empty(parent_state_summary: LedgerStateSummary) -> Self {
        Self::new_impl(Inner {
            state_summary: parent_state_summary,
            state_checkpoint_hashes: vec![],
        })
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(LedgerStateSummary::new_empty())
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        Self::new_empty(self.state_summary.clone())
    }
}

#[derive(Debug)]
pub struct Inner {
    pub state_summary: LedgerStateSummary,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
}
