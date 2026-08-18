// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V1 harness: replays the whole transaction (prologue, payload, epilogue) on
//! the legacy AptosVM and returns its output and timing.
//!
//! This is the stock production execution path, including the production gas
//! meter — gas is free only because the loaded state's gas schedule is zero-cost
//! (see `gas.rs`), so the output carries no fee effects and is byte-comparable
//! with V2's. The VM, environment, and code storage are built once: an untimed
//! trial run warms the module cache, and each timed sample covers execution
//! plus materialization into a [`TransactionOutput`].

use crate::{data::BenchmarkInput, measure, timing::TimingConfig, BenchmarkRun};
use anyhow::{anyhow, Result};
use aptos_types::{
    state_store::TStateView,
    transaction::{AuxiliaryInfo, TransactionOutput},
};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;

pub fn run(input: &BenchmarkInput, timing: &TimingConfig) -> Result<BenchmarkRun> {
    let state_view = input.state.as_ref();
    let env = AptosEnvironment::new(state_view);
    let vm = AptosVM::new(&env);
    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);
    let aux_info = AuxiliaryInfo::new(input.aux_info, None);

    let execute_once = || -> Result<TransactionOutput> {
        let (_, vm_output) = vm.execute_user_transaction(
            &resolver,
            &code_storage,
            &input.txn,
            &log_context,
            &aux_info,
        );
        vm_output
            .try_materialize_into_transaction_output()
            .map_err(|status| anyhow!("failed to materialize V1 output: {:?}", status))
    };

    measure(timing, execute_once)
}
