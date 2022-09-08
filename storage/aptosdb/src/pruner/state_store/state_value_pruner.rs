// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{pruner::db_sub_pruner::DBSubPruner, StateStore};
use schemadb::SchemaBatch;
use std::sync::Arc;

#[derive(Debug)]
pub struct StateValuePruner {
    state_store: Arc<StateStore>,
}

impl DBSubPruner for StateValuePruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        self.state_store
            .prune_state_values(min_readable_version, target_version, db_batch)?;
        Ok(())
    }
}

impl StateValuePruner {
    pub(in crate::pruner) fn new(state_store: Arc<StateStore>) -> Self {
        StateValuePruner { state_store }
    }
}
