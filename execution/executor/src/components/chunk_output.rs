// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::components::apply_chunk_output::ApplyChunkOutput;
use anyhow::Result;
use diem_crypto::hash::TransactionAccumulatorHasher;
use diem_state_view::StateView;
use diem_types::{
    proof::accumulator::InMemoryAccumulator,
    protocol_spec::DpnProto,
    transaction::{Transaction, TransactionOutput},
};
use diem_vm::VMExecutor;
use executor_types::ExecutedChunk;
use fail::fail_point;
use std::{collections::HashSet, sync::Arc};
use storage_interface::state_view::{StateCache, VerifiedStateView};

pub struct ChunkOutput {
    /// Input transactions.
    pub transactions: Vec<Transaction>,
    /// Raw VM output.
    pub transaction_outputs: Vec<TransactionOutput>,
    /// Carries the frozen base state view, so all in-mem nodes involved won't drop before the
    /// execution result is processed; as well as al the accounts touched during execution, together
    /// with their proofs.
    pub state_cache: StateCache,
}

impl ChunkOutput {
    pub fn by_transaction_execution<V: VMExecutor>(
        transactions: Vec<Transaction>,
        state_view: VerifiedStateView<DpnProto>,
    ) -> Result<Self> {
        let transaction_outputs = V::execute_block(transactions.clone(), &state_view)?;

        Ok(Self {
            transactions,
            transaction_outputs,
            state_cache: state_view.into(),
        })
    }

    pub fn by_transaction_output(
        transactions_and_outputs: Vec<(Transaction, TransactionOutput)>,
        state_view: VerifiedStateView<DpnProto>,
    ) -> Result<Self> {
        let (transactions, transaction_outputs): (Vec<_>, Vec<_>) =
            transactions_and_outputs.into_iter().unzip();

        // collect all accounts touched and dedup
        let access_paths = transaction_outputs
            .iter()
            .map(|o| o.write_set())
            .flatten()
            .collect::<HashSet<_>>();

        // prime the state cache by fetching all touched accounts
        // TODO: add concurrency
        access_paths
            .iter()
            .map(|(p, _)| state_view.get(p))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            transactions,
            transaction_outputs,
            state_cache: state_view.into(),
        })
    }

    pub fn apply_to_ledger(
        self,
        base_accumulator: &Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
    ) -> Result<ExecutedChunk> {
        fail_point!("executor::vm_execute_chunk", |_| {
            Err(anyhow::anyhow!("Injected error in apply_to_ledger."))
        });
        ApplyChunkOutput::apply(self, base_accumulator)
    }
}
