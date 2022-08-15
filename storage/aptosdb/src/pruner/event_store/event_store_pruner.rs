// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{pruner::db_sub_pruner::DBSubPruner, EventStore};
use schemadb::SchemaBatch;
use std::sync::Arc;

#[derive(Debug)]
pub struct EventStorePruner {
    event_store: Arc<EventStore>,
}

impl DBSubPruner for EventStorePruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        self.event_store
            .prune_events(min_readable_version, target_version, db_batch)?;
        Ok(())
    }
}

impl EventStorePruner {
    pub(in crate::pruner) fn new(event_store: Arc<EventStore>) -> Self {
        EventStorePruner { event_store }
    }
}
