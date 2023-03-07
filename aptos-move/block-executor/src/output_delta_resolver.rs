// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{executor::RAYON_EXEC_POOL, task::Transaction};
use aptos_aggregator::delta_change_set::{deserialize, serialize};
use aptos_mvhashmap::{EntryCell, MVHashMap};
use aptos_state_view::TStateView;
use aptos_types::write_set::{TransactionWrite, WriteOp};

pub(crate) struct OutputDeltaResolver<T: Transaction> {
    versioned_outputs: MVHashMap<T::Key, T::Value>,
}

impl<T: Transaction> OutputDeltaResolver<T> {
    pub fn new(versioned_outputs: MVHashMap<T::Key, T::Value>) -> Self {
        Self { versioned_outputs }
    }

    /// Takes Self, vector of all involved aggregator keys (each with at least one
    /// delta to resolve in the output), resolved values from storage for each key,
    /// and blocksize, and returns a Vec of materialized deltas per transaction index.
    pub(crate) fn resolve(
        self,
        base_view: &impl TStateView<Key = T::Key>,
        block_size: usize,
    ) -> Vec<Vec<(T::Key, WriteOp)>> {
        let mut ret: Vec<Vec<(T::Key, WriteOp)>> = vec![vec![]; block_size];

        // TODO: with more deltas, re-use executor threads and process in parallel.
        for key in self.versioned_outputs.aggregator_keys() {
            let mut latest_value: Option<u128> = base_view
                .get_state_value_bytes(&key)
                .ok() // Was anything found in storage
                .and_then(|value| value.map(|bytes| deserialize(&bytes)));

            let indexed_entries = self
                .versioned_outputs
                .entry_map_for_key(&key)
                .expect("No entries found for the provided key");
            for (idx, entry) in indexed_entries.iter() {
                match &entry.cell {
                    EntryCell::Write(_, data) => {
                        latest_value = data.extract_raw_bytes().map(|bytes| deserialize(&bytes))
                    },
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
                    },
                }
            }
        }

        RAYON_EXEC_POOL.spawn(move || drop(self));

        ret
    }
}
