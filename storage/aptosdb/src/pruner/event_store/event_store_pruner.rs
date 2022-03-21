// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{
    event::EventSchema, metrics::DIEM_PRUNER_LEAST_READABLE_VERSION, pruner::db_pruner::DBPruner,
    EventStore,
};
use aptos_types::{
    contract_event::ContractEvent,
    event::EventKey,
    transaction::{AtomicVersion, Version},
};
use itertools::Itertools;
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::{
    collections::HashSet,
    sync::{atomic::Ordering, Arc},
};

pub const EVENT_STORE_PRUNER_NAME: &str = "event store pruner";

pub struct EventStorePruner {
    db: Arc<DB>,
    event_store: Arc<EventStore>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    least_readable_version: AtomicVersion,
}

impl DBPruner for EventStorePruner {
    fn name(&self) -> &'static str {
        EVENT_STORE_PRUNER_NAME
    }

    fn prune(&self, db_batch: &mut SchemaBatch, max_versions: u64) -> anyhow::Result<Version> {
        // Current target version  might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        let current_target_version = self.get_currrent_batch_target(max_versions);
        let candidate_events = self
            .get_pruning_candidate_events(self.least_readable_version(), current_target_version)?;

        let event_keys: HashSet<EventKey> =
            candidate_events.iter().map(|event| *event.key()).collect();

        self.event_store.prune_events_by_version(
            event_keys,
            self.least_readable_version(),
            current_target_version,
            db_batch,
        )?;

        self.event_store
            .prune_events_by_key(&candidate_events, db_batch)?;

        self.event_store.prune_event_accumulator(
            self.least_readable_version(),
            current_target_version,
            db_batch,
        )?;

        self.event_store.prune_event_schema(
            self.least_readable_version(),
            current_target_version,
            db_batch,
        )?;

        self.record_progress(current_target_version);
        Ok(current_target_version)
    }

    fn initialize_least_readable_version(&self) -> anyhow::Result<Version> {
        let mut iter = self.db.iter::<EventSchema>(ReadOptions::default())?;
        iter.seek_to_first();
        let version = iter.next().transpose()?.map_or(0, |(key, _)| key.0);
        Ok(version)
    }

    fn least_readable_version(&self) -> Version {
        self.least_readable_version.load(Ordering::Relaxed)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed)
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    fn record_progress(&self, least_readable_version: Version) {
        self.least_readable_version
            .store(least_readable_version, Ordering::Relaxed);
        DIEM_PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&["event_store"])
            .set(least_readable_version as i64);
    }
}

impl EventStorePruner {
    pub(in crate::pruner) fn new(db: Arc<DB>, event_store: Arc<EventStore>) -> Self {
        EventStorePruner {
            db,
            event_store,
            target_version: AtomicVersion::new(0),
            least_readable_version: AtomicVersion::new(0),
        }
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
