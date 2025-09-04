// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This file defines event store APIs that are related to the event accumulator and events
//! themselves.
#![allow(unused)]

use super::AptosDB;
use crate::schema::{event::EventSchema, event_accumulator::EventAccumulatorSchema};
use anyhow::anyhow;
use aptos_accumulator::HashReader;
use aptos_crypto::{HashValue, hash::CryptoHash};
use aptos_db_indexer_schemas::schema::{
    event_by_key::EventByKeySchema, event_by_version::EventByVersionSchema,
};
use aptos_schemadb::{DB, batch::SchemaBatch, schema::ValueCodec};
use aptos_storage_interface::{AptosDbError, Result, db_ensure as ensure, db_other_bail};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{NewBlockEvent, new_block_event_key},
    contract_event::ContractEvent,
    event::EventKey,
    proof::position::Position,
    transaction::Version,
};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    sync::Arc,
};

#[derive(Debug)]
pub struct EventStore {
    event_db: Arc<DB>,
}

impl EventStore {
    pub fn new(event_db: Arc<DB>) -> Self {
        Self { event_db }
    }

    pub fn get_event_by_version_and_index(
        &self,
        version: Version,
        index: u64,
    ) -> Result<ContractEvent> {
        self.event_db
            .get::<EventSchema>(&(version, index))?
            .ok_or_else(|| AptosDbError::NotFound(format!("Event {} of Txn {}", index, version)))
    }

    pub fn get_txn_ver_by_seq_num(&self, event_key: &EventKey, seq_num: u64) -> Result<u64> {
        let (ver, _) = self
            .event_db
            .get::<EventByKeySchema>(&(*event_key, seq_num))?
            .ok_or_else(|| {
                AptosDbError::NotFound(format!("Index entry should exist for seq_num {}", seq_num))
            })?;
        Ok(ver)
    }

    pub fn get_event_by_key(
        &self,
        event_key: &EventKey,
        seq_num: u64,
        ledger_version: Version,
    ) -> Result<(Version, ContractEvent)> {
        let (version, index) = self.lookup_event_by_key(event_key, seq_num, ledger_version)?;
        Ok((
            version,
            self.get_event_by_version_and_index(version, index)?,
        ))
    }

    /// Get the latest sequence number on `event_key` considering all transactions with versions
    /// no greater than `ledger_version`.
    pub fn get_latest_sequence_number(
        &self,
        ledger_version: Version,
        event_key: &EventKey,
    ) -> Result<Option<u64>> {
        let mut iter = self.event_db.iter::<EventByVersionSchema>()?;
        iter.seek_for_prev(&(*event_key, ledger_version, u64::MAX));

        Ok(iter.next().transpose()?.and_then(
            |((key, _version, seq), _idx)| if &key == event_key { Some(seq) } else { None },
        ))
    }

    /// Get the next sequence number for specified event key.
    /// Returns 0 if there's no events already in the event stream.
    pub fn get_next_sequence_number(
        &self,
        ledger_version: Version,
        event_key: &EventKey,
    ) -> Result<u64> {
        self.get_latest_sequence_number(ledger_version, event_key)?
            .map_or(Ok(0), |seq| {
                seq.checked_add(1)
                    .ok_or_else(|| AptosDbError::Other("Seq num overflowed.".to_string()))
            })
    }

    /// Given `event_key` and `start_seq_num`, returns events identified by transaction version and
    /// index among all events emitted by the same transaction. Result won't contain records with a
    /// transaction version > `ledger_version` and is in ascending order.
    pub fn lookup_events_by_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        limit: u64,
        ledger_version: u64,
    ) -> Result<
        Vec<(
            u64,     // sequence number
            Version, // transaction version it belongs to
            u64,     // index among events for the same transaction
        )>,
    > {
        let mut iter = self.event_db.iter::<EventByKeySchema>()?;
        iter.seek(&(*event_key, start_seq_num))?;

        let mut result = Vec::new();
        let mut cur_seq = start_seq_num;
        for res in iter.take(limit as usize) {
            let ((path, seq), (ver, idx)) = res?;
            if path != *event_key || ver > ledger_version {
                break;
            }
            if seq != cur_seq {
                let msg = if cur_seq == start_seq_num {
                    "First requested event is probably pruned."
                } else {
                    "DB corruption: Sequence number not continuous."
                };
                db_other_bail!("{} expected: {}, actual: {}", msg, cur_seq, seq);
            }
            result.push((seq, ver, idx));
            cur_seq += 1;
        }

        Ok(result)
    }

    fn lookup_event_by_key(
        &self,
        event_key: &EventKey,
        seq_num: u64,
        ledger_version: Version,
    ) -> Result<(Version, u64)> {
        let indices = self.lookup_events_by_key(event_key, seq_num, 1, ledger_version)?;
        if indices.is_empty() {
            return Err(AptosDbError::NotFound(format!(
                "Event {} of seq num {}.",
                event_key, seq_num
            )));
        }
        let (_seq, version, index) = indices[0];

        Ok((version, index))
    }

    pub fn lookup_event_before_or_at_version(
        &self,
        event_key: &EventKey,
        version: Version,
    ) -> Result<
        Option<(
            Version, // version
            u64,     // index
            u64,     // sequence number
        )>,
    > {
        let mut iter = self.event_db.iter::<EventByVersionSchema>()?;
        iter.seek_for_prev(&(*event_key, version, u64::MAX))?;

        match iter.next().transpose()? {
            None => Ok(None),
            Some(((key, ver, seq_num), idx)) => {
                if key == *event_key {
                    Ok(Some((ver, idx, seq_num)))
                } else {
                    Ok(None)
                }
            },
        }
    }

    pub fn lookup_event_at_or_after_version(
        &self,
        event_key: &EventKey,
        version: Version,
    ) -> Result<
        Option<(
            Version, // version
            u64,     // index
            u64,     // sequence number
        )>,
    > {
        let mut iter = self.event_db.iter::<EventByVersionSchema>()?;
        iter.seek(&(*event_key, version, 0))?;

        match iter.next().transpose()? {
            None => Ok(None),
            Some(((key, ver, seq_num), idx)) => {
                if key == *event_key {
                    Ok(Some((ver, idx, seq_num)))
                } else {
                    Ok(None)
                }
            },
        }
    }

    pub fn lookup_event_after_version(
        &self,
        event_key: &EventKey,
        version: Version,
    ) -> Result<
        Option<(
            Version, // version
            u64,     // index
            u64,     // sequence number
        )>,
    > {
        let mut iter = self.event_db.iter::<EventByVersionSchema>()?;
        iter.seek(&(*event_key, version + 1, 0))?;

        match iter.next().transpose()? {
            None => Ok(None),
            Some(((key, ver, seq_num), idx)) => {
                if key == *event_key {
                    Ok(Some((ver, idx, seq_num)))
                } else {
                    Ok(None)
                }
            },
        }
    }

    /// Finds the first event sequence number in a specified stream on which `comp` returns false.
    /// (assuming the whole stream is partitioned by `comp`)
    fn search_for_event_lower_bound<C>(
        &self,
        event_key: &EventKey,
        mut comp: C,
        ledger_version: Version,
    ) -> Result<Option<u64>>
    where
        C: FnMut(&ContractEvent) -> Result<bool>,
    {
        let mut begin = 0u64;
        let mut end = match self.get_latest_sequence_number(ledger_version, event_key)? {
            Some(s) => s.checked_add(1).ok_or_else(|| {
                AptosDbError::Other("event sequence number overflew.".to_string())
            })?,
            None => return Ok(None),
        };

        // overflow not possible
        #[allow(clippy::arithmetic_side_effects)]
        {
            let mut count = end - begin;
            while count > 0 {
                let step = count / 2;
                let mid = begin + step;
                let (_version, event) = self.get_event_by_key(event_key, mid, ledger_version)?;
                if comp(&event)? {
                    begin = mid + 1;
                    count -= step + 1;
                } else {
                    count = step;
                }
            }
        }

        if begin == end {
            Ok(None)
        } else {
            Ok(Some(begin))
        }
    }

    /// Gets the version of the last transaction committed before timestamp,
    /// a commited block at or after the required timestamp must exist (otherwise it's possible
    /// the next block committed as a timestamp smaller than the one in the request).
    pub(crate) fn get_last_version_before_timestamp(
        &self,
        timestamp: u64,
        ledger_version: Version,
    ) -> Result<Version> {
        let event_key = new_block_event_key();
        let seq_at_or_after_ts = self.search_for_event_lower_bound(
            &event_key,
            |event| {
                let new_block_event: NewBlockEvent = event.try_into()?;
                Ok(new_block_event.proposed_time() < timestamp)
            },
            ledger_version,
        )?.ok_or_else(|| AptosDbError::NotFound(
            format!("No new block found beyond timestamp {}, so can't determine the last version before it.",
            timestamp,
        )))?;

        ensure!(
            seq_at_or_after_ts > 0,
            "First block started at or after timestamp {}.",
            timestamp,
        );

        let (version, _idx) =
            self.lookup_event_by_key(&event_key, seq_at_or_after_ts, ledger_version)?;

        version.checked_sub(1).ok_or_else(|| {
            AptosDbError::Other("A block with non-zero seq num started at version 0.".to_string())
        })
    }

    /// Prunes events by accumulator store for a range of version in [begin, end)
    pub(crate) fn prune_event_accumulator(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        let mut iter = self.event_db.iter::<EventAccumulatorSchema>()?;
        iter.seek(&(begin, Position::from_inorder_index(0)))?;
        while let Some(((version, position), _)) = iter.next().transpose()? {
            if version >= end {
                return Ok(());
            }
            db_batch.delete::<EventAccumulatorSchema>(&(version, position))?;
        }
        Ok(())
    }
}

struct EventHashReader<'a> {
    store: &'a EventStore,
    version: Version,
}

impl<'a> EventHashReader<'a> {
    fn new(store: &'a EventStore, version: Version) -> Self {
        Self { store, version }
    }
}

impl HashReader for EventHashReader<'_> {
    fn get(&self, position: Position) -> Result<HashValue, anyhow::Error> {
        self.store
            .event_db
            .get::<EventAccumulatorSchema>(&(self.version, position))?
            .ok_or_else(|| anyhow!("Hash at position {:?} not found.", position))
    }
}

pub(crate) struct EmptyReader;

// Asserts `get()` is never called.
impl HashReader for EmptyReader {
    fn get(&self, _position: Position) -> Result<HashValue, anyhow::Error> {
        unreachable!()
    }
}

#[cfg(test)]
mod test;
