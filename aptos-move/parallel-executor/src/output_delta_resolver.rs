// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::{deserialize, serialize};
use aptos_types::write_set::{TransactionWrite, WriteOp};
use mvhashmap::{EntryCell, MVHashMap};
use std::{hash::Hash, thread::spawn};

/// Resolved and serialized data for WriteOps, None means deletion.
pub type ResolvedData = Option<Vec<u8>>;

pub struct OutputDeltaResolver<K, V> {
    versioned_outputs: MVHashMap<K, V>,
}

impl<K: Hash + Clone + Eq + Send + 'static, V: TransactionWrite + Send + Sync + 'static>
    OutputDeltaResolver<K, V>
{
    pub fn new(versioned_outputs: MVHashMap<K, V>) -> Self {
        Self { versioned_outputs }
    }

    /// Takes Self, vector of all involved aggregator keys (each with at least one
    /// delta to resolve in the output), resolved values from storage for each key,
    /// and blocksize, and returns a Vec of materialized deltas per transaction index.
    pub fn resolve(
        self,
        aggregator_keys: Vec<(K, anyhow::Result<ResolvedData>)>,
        block_size: usize,
    ) -> Vec<Vec<(K, WriteOp)>> {
        let mut ret: Vec<Vec<(K, WriteOp)>> = (0..block_size).map(|_| Vec::new()).collect();

        // TODO: with more deltas, re-use executor threads and process in parallel.
        for (key, storage_val) in aggregator_keys.into_iter() {
            let mut latest_value: Option<u128> = match storage_val
                .ok() // Was anything found in storage
                .map(|value| value.map(|bytes| deserialize(&bytes)))
            {
                None => None,
                Some(v) => v,
            };

            let indexed_entries = self
                .versioned_outputs
                .entry_map_for_key(&key)
                .expect("No entries found for the provided key");
            for (idx, entry) in indexed_entries.iter() {
                match &entry.cell {
                    EntryCell::Write(_, data) => {
                        latest_value = data.extract_raw_bytes().map(|bytes| deserialize(&bytes))
                    }
                    EntryCell::Delta(delta) => {
                        // Apply to the latest value and store in outputs.
                        let aggregator_value = delta
                            .apply_to(
                                latest_value
                                    .expect("Failed to apply delta to (non-existent) aggregator"),
                            )
                            .expect("Failed to apply aggregator delta output");

                        ret[*idx].push((
                            key.clone(),
                            WriteOp::Modification(serialize(&aggregator_value)),
                        ));
                        latest_value = Some(aggregator_value);
                    }
                }
            }
        }

        spawn(move || drop(self));

        ret
    }
}
