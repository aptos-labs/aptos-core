// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 harness: replays the whole transaction (prologue, payload, epilogue) on
//! the MonoMove-backed Aptos transaction executor and returns its output and
//! timing.
//!
//! Gas is free (the executor's zero-gas mode), so the output carries no fee
//! effects and is byte-comparable with V1's. The global context and providers
//! are built once: an untimed trial run warms the module cache, and each timed
//! sample covers execution plus materialization into a [`TransactionOutput`].

use crate::{data::BenchmarkInput, measure, timing::TimingConfig, BenchmarkRun};
use anyhow::{anyhow, Result};
use aptos_types::{
    on_chain_config::{Features, OnChainConfig},
    state_store::TStateView,
    transaction::{AuxiliaryInfo, TransactionAuxiliaryData, TransactionOutput},
};
use mono_move_aptos_state_view_providers::{
    StateViewModuleProvider, StateViewResourceProvider, DEFAULT_RESOURCE_ARENA_BYTES,
};
use mono_move_aptos_transaction_executor::{production_natives, AptosTransactionExecutor};
use mono_move_global_context::GlobalContext;

/// The provider's value arena grows with the read-set (the flat representation
/// can be larger than BCS), with [`DEFAULT_RESOURCE_ARENA_BYTES`] as the floor.
const ARENA_BYTES_PER_RESOURCE_BYTE: usize = 8;

pub fn run(input: &BenchmarkInput, timing: &TimingConfig) -> Result<BenchmarkRun> {
    let state_view = input.state.as_ref();

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire MonoMove execution guard"))?;
    let natives = production_natives(&guard);

    let module_provider = StateViewModuleProvider::new(state_view);
    // TODO(perf): the provider re-materializes each read into its arena on
    // every run (no cross-run cache), so occupancy grows with `--samples`;
    // reset it between samples once the provider exposes that.
    let total_bytes: usize = input
        .state
        .to_btree_map()
        .values()
        .map(|v| v.bytes().len())
        .sum();
    let arena_size = total_bytes
        .saturating_mul(ARENA_BYTES_PER_RESOURCE_BYTE)
        .max(DEFAULT_RESOURCE_ARENA_BYTES);
    let data_provider = StateViewResourceProvider::new(&guard, state_view, arena_size);

    let features = Features::fetch_config(state_view)
        .ok()
        .flatten()
        .unwrap_or_default();
    let usage = state_view.get_usage()?;
    let executor = AptosTransactionExecutor::new(
        &guard,
        &natives,
        &module_provider,
        &data_provider,
        &features,
        usage,
    )
    .with_zero_gas();

    let aux_info = AuxiliaryInfo::new(input.aux_info, None);
    let execute_once = || -> Result<TransactionOutput> {
        executor
            .execute_user_transaction(&input.txn, &aux_info)
            .materialize(
                &guard,
                &data_provider,
                &features,
                TransactionAuxiliaryData::default(),
            )
            .map_err(|e| anyhow!("failed to materialize V2 output: {}", e))
    };

    measure(timing, execute_once)
}
