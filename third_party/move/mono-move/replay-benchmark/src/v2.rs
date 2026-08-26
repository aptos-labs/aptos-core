// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 harness: replays the whole transaction (prologue, payload, epilogue) on
//! the MonoMove-backed Aptos transaction executor and returns its output and
//! timing.
//!
//! Gas is free (the executor's zero-gas mode), so the output carries no fee
//! effects and is byte-comparable with V1's. The global context and providers
//! are built once: an untimed trial run warms the caches, and
//! each timed sample covers execution plus materialization into a
//! [`TransactionOutput`].

use crate::{data::BenchmarkInput, measure, timing::TimingConfig, BenchmarkRun};
use anyhow::{anyhow, Result};
use aptos_types::{
    state_store::TStateView,
    transaction::{AuxiliaryInfo, TransactionAuxiliaryData, TransactionOutput},
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_aptos_state_view_providers::{StateViewModuleProvider, StateViewResourceProvider};
use mono_move_aptos_transaction_executor::{production_natives, AptosTransactionExecutor};
use mono_move_global_context::GlobalContext;

pub fn run(input: &BenchmarkInput, timing: &TimingConfig) -> Result<BenchmarkRun> {
    let state_view = input.state.as_ref();

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire MonoMove execution guard"))?;
    let natives = production_natives(&guard);

    let module_provider = StateViewModuleProvider::new(state_view);
    let data_provider = StateViewResourceProvider::new(&guard, state_view);

    let env = AptosEnvironment::new(state_view);
    let usage = state_view.get_usage()?;
    let executor = AptosTransactionExecutor::new(
        &guard,
        &natives,
        &module_provider,
        &data_provider,
        &env,
        usage,
    )
    .without_metering();

    let aux_info = AuxiliaryInfo::new(input.aux_info, None);
    let txn = input.txn.to_transaction();
    let execute_once = || -> Result<TransactionOutput> {
        executor
            .execute_transaction(&txn, &aux_info)
            .materialize(
                &data_provider,
                env.features(),
                TransactionAuxiliaryData::default(),
            )
            .map_err(|e| anyhow!("failed to materialize V2 output: {}", e))
    };

    measure(timing, execute_once)
}
