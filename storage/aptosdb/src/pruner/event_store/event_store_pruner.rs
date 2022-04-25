// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{pruner::db_sub_pruner::DBSubPruner, EventStore};
use aptos_types::{contract_event::ContractEvent, event::EventKey, transaction::Version};
use itertools::Itertools;
use schemadb::SchemaBatch;
use std::{collections::HashSet, sync::Arc};

pub struct EventStorePruner {
    event_store: Arc<EventStore>,
}

impl DBSubPruner for EventStorePruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        least_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        let candidate_events =
            self.get_pruning_candidate_events(least_readable_version, target_version)?;

        let event_keys: HashSet<EventKey> =
            candidate_events.iter().map(|event| *event.key()).collect();

        self.event_store.prune_events_by_version(
            event_keys,
            least_readable_version,
            target_version,
            db_batch,
        )?;

        self.event_store
            .prune_events_by_key(&candidate_events, db_batch)?;

        self.event_store.prune_event_accumulator(
            least_readable_version,
            target_version,
            db_batch,
        )?;

        self.event_store
            .prune_event_schema(least_readable_version, target_version, db_batch)?;

        Ok(())
    }
}

impl EventStorePruner {
    pub(in crate::pruner) fn new(event_store: Arc<EventStore>) -> Self {
        EventStorePruner { event_store }
    }

    fn get_pruning_candidate_events(
        &self,
        start: Version,
        end: Version,
    ) -> anyhow::Result<Vec<ContractEvent>> {
        self.event_store
            .get_events_by_version_iter(start, (end - start) as usize)?
            .flatten_ok()
            .collect()
    }
}
