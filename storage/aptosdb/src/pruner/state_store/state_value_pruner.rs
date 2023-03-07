// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::db_sub_pruner::DBSubPruner,
    schema::{stale_state_value_index::StaleStateValueIndexSchema, state_value::StateValueSchema},
};
use aptos_schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::Arc;

pub struct StateValuePruner {
    state_kv_db: Arc<DB>,
}

impl DBSubPruner for StateValuePruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        let mut iter = self
            .state_kv_db
            .iter::<StaleStateValueIndexSchema>(ReadOptions::default())?;
        iter.seek(&min_readable_version)?;
        for item in iter {
            let (index, _) = item?;
            if index.stale_since_version > target_version {
                break;
            }
            db_batch.delete::<StaleStateValueIndexSchema>(&index)?;
            db_batch.delete::<StateValueSchema>(&(index.state_key, index.version))?;
        }
        Ok(())
    }
}

impl StateValuePruner {
    pub(in crate::pruner) fn new(state_kv_db: Arc<DB>) -> Self {
        StateValuePruner { state_kv_db }
    }
}
