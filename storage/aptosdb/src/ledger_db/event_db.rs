// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    event_store::{EmptyReader, EventStore},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        event::EventSchema,
        event_accumulator::EventAccumulatorSchema,
    },
    utils::iterators::EventsByVersionIter,
};
use aptos_accumulator::MerkleAccumulator;
use aptos_crypto::{
    HashValue,
    hash::{CryptoHash, EventAccumulatorHasher},
};
use aptos_db_indexer_schemas::schema::{
    event_by_key::EventByKeySchema, event_by_version::EventByVersionSchema,
};
use aptos_schemadb::{
    DB,
    batch::{SchemaBatch, WriteBatch},
};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    account_config::new_block_event_key, contract_event::ContractEvent, transaction::Version,
};
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct EventDb {
    db: Arc<DB>,
    // TODO(grao): Remove this after sharding migration.
    event_store: EventStore,
}

impl EventDb {
    pub(super) fn new(db: Arc<DB>, event_store: EventStore) -> Self {
        Self { db, event_store }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::EventPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn db(&self) -> &DB {
        &self.db
    }

    pub(crate) fn db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.db)
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }

    /// Returns all of the events for a given transaction version.
    pub(crate) fn get_events_by_version(&self, version: Version) -> Result<Vec<ContractEvent>> {
        let mut events = vec![];

        let mut iter = self.db.iter::<EventSchema>()?;
        // Grab the first event and then iterate until we get all events for this version.
        iter.seek(&version)?;
        while let Some(((ver, _index), event)) = iter.next().transpose()? {
            if ver != version {
                break;
            }
            events.push(event);
        }

        Ok(events)
    }

    pub(crate) fn expect_new_block_event(&self, version: Version) -> Result<ContractEvent> {
        for event in self.get_events_by_version(version)? {
            if let Some(key) = event.event_key() {
                if *key == new_block_event_key() {
                    return Ok(event);
                }
            }
        }

        Err(AptosDbError::NotFound(format!(
            "NewBlockEvent at version {}",
            version,
        )))
    }

    /// Returns an iterator that yields at most `num_versions` versions' events starting from
    /// `start_version`.
    pub(crate) fn get_events_by_version_iter(
        &self,
        start_version: Version,
        num_versions: usize,
    ) -> Result<EventsByVersionIter> {
        let mut iter = self.db.iter::<EventSchema>()?;
        iter.seek(&start_version)?;

        Ok(EventsByVersionIter::new(
            iter,
            start_version,
            start_version.checked_add(num_versions as u64).ok_or(
                AptosDbError::TooManyRequested(num_versions as u64, Version::MAX),
            )?,
        ))
    }

    /// Returns the version of the latest event committed in the event db.
    pub(crate) fn latest_version(&self) -> Result<Option<Version>> {
        let mut iter = self.db.iter::<EventSchema>()?;
        iter.seek_to_last();
        if let Some(((version, _), _)) = iter.next().transpose()? {
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    /// Saves contract events yielded by multiple transactions starting from version
    /// `first_version`.
    pub(crate) fn put_events_multiple_versions(
        &self,
        first_version: u64,
        event_vecs: &[Vec<ContractEvent>],
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        event_vecs.iter().enumerate().try_for_each(|(idx, events)| {
            let version = first_version
                .checked_add(idx as Version)
                .ok_or_else(|| AptosDbError::Other("version overflow".to_string()))?;
            self.put_events(version, events, /*skip_index=*/ false, batch)
        })
    }

    /// Saves contract events yielded by the transaction at `version`.
    pub(crate) fn put_events(
        &self,
        version: u64,
        events: &[ContractEvent],
        skip_index: bool,
        batch: &mut impl WriteBatch,
    ) -> Result<()> {
        // Event table and indices updates
        events
            .iter()
            .enumerate()
            .try_for_each::<_, Result<_>>(|(idx, event)| {
                if let ContractEvent::V1(v1) = event {
                    if !skip_index {
                        batch.put::<EventByKeySchema>(
                            &(*v1.key(), v1.sequence_number()),
                            &(version, idx as u64),
                        )?;
                        batch.put::<EventByVersionSchema>(
                            &(*v1.key(), version, v1.sequence_number()),
                            &(idx as u64),
                        )?;
                    }
                }
                batch.put::<EventSchema>(&(version, idx as u64), event)
            })?;

        if !skip_index {
            // EventAccumulatorSchema updates
            let event_hashes: Vec<HashValue> = events.iter().map(ContractEvent::hash).collect();
            let (_root_hash, writes) =
                MerkleAccumulator::<EmptyReader, EventAccumulatorHasher>::append(
                    &EmptyReader,
                    0,
                    &event_hashes,
                )?;

            writes.into_iter().try_for_each(|(pos, hash)| {
                batch.put::<EventAccumulatorSchema>(&(version, pos), &hash)
            })?;
        }

        Ok(())
    }

    /// Deletes event indices, returns number of events per version, so `prune_events` doesn't need
    /// to iterate through evnets from DB again.
    pub(crate) fn prune_event_indices(
        &self,
        start: Version,
        end: Version,
        mut indices_batch: Option<&mut SchemaBatch>,
    ) -> Result<Vec<usize>> {
        let mut ret = Vec::new();

        let mut current_version = start;

        for events in self.get_events_by_version_iter(start, (end - start) as usize)? {
            let events = events?;
            ret.push(events.len());

            if let Some(ref mut batch) = indices_batch {
                for event in events {
                    if let ContractEvent::V1(v1) = event {
                        batch.delete::<EventByKeySchema>(&(*v1.key(), v1.sequence_number()))?;
                        batch.delete::<EventByVersionSchema>(&(
                            *v1.key(),
                            current_version,
                            v1.sequence_number(),
                        ))?;
                    }
                }
            }
            current_version += 1;
        }

        Ok(ret)
    }

    /// Deletes a set of events in the range of version in [begin, end), and all related indices.
    pub(crate) fn prune_events(
        &self,
        num_events_per_version: Vec<usize>,
        start: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        let mut current_version = start;

        for num_events in num_events_per_version {
            for idx in 0..num_events {
                db_batch.delete::<EventSchema>(&(current_version, idx as u64))?;
            }
            current_version += 1;
        }
        self.event_store
            .prune_event_accumulator(start, end, db_batch)?;
        Ok(())
    }
}
