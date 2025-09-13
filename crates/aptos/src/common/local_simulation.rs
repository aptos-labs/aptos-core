// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliError, CliTypedResult};
use aptos_crypto::HashValue;
use aptos_gas_profiling::FrameName;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::transaction::{AuxiliaryInfo, PersistedAuxiliaryInfo, SignedTransaction};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, output::VMOutput, resolver::StateStorageView,
};
use move_core_types::vm_status::VMStatus;
use std::{path::Path, time::Instant};

pub fn run_transaction_using_debugger(
    debugger: &AptosDebugger,
    version: u64,
    transaction: SignedTransaction,
    _hash: HashValue,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    let state_view = debugger.state_view_at_version(version);
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env, &state_view);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);

    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);

    let (vm_status, vm_output) = vm.execute_user_transaction(
        &resolver,
        &code_storage,
        &transaction,
        &log_context,
        &AuxiliaryInfo::default(),
    );

    Ok((vm_status, vm_output))
}

pub fn benchmark_transaction_using_debugger(
    debugger: &AptosDebugger,
    version: u64,
    transaction: SignedTransaction,
    _hash: HashValue,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    let state_view = debugger.state_view_at_version(version);
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env, &state_view);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);

    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);
    let (vm_status, vm_output) = vm.execute_user_transaction(
        &resolver,
        &code_storage,
        &transaction,
        &log_context,
        &AuxiliaryInfo::default(),
    );

    let time_cold = {
        let n = 15;

        let mut times = vec![];
        for _i in 0..n {
            // Create a new VM each time so to include code loading as part of the
            // total running time.
            let vm = AptosVM::new(&env, &state_view);
            let code_storage = state_view.as_aptos_code_storage(&env);
            let log_context = AdapterLogSchema::new(state_view.id(), 0);

            let t1 = Instant::now();
            std::hint::black_box(vm.execute_user_transaction(
                &resolver,
                &code_storage,
                &transaction,
                &log_context,
                &AuxiliaryInfo::default(),
            ));
            let t2 = Instant::now();

            times.push(t2 - t1);
        }
        times.sort();

        times[n / 2]
    };

    let time_warm = {
        let mut times = vec![];
        let n = 15;

        for i in 0..n {
            // Reuse the existing VM with warm code cache so to measure only the
            // execution time.
            let t1 = Instant::now();
            std::hint::black_box(vm.execute_user_transaction(
                &resolver,
                &code_storage,
                &transaction,
                &log_context,
                &AuxiliaryInfo::new(
                    PersistedAuxiliaryInfo::V1 {
                        transaction_index: i,
                    },
                    None,
                ),
            ));
            let t2 = Instant::now();

            times.push(t2 - t1);
        }
        times.sort();

        times[(n / 2) as usize]
    };

    println!("Running time (cold code cache): {:?}", time_cold);
    println!("Running time (warm code cache): {:?}", time_warm);

    Ok((vm_status, vm_output))
}

pub fn profile_transaction_using_debugger(
    debugger: &AptosDebugger,
    version: u64,
    transaction: SignedTransaction,
    hash: HashValue,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    let (vm_status, vm_output, gas_log) = debugger
        .execute_transaction_at_version_with_gas_profiler(
            version,
            transaction,
            AuxiliaryInfo::new(
                PersistedAuxiliaryInfo::V1 {
                    transaction_index: 2,
                },
                None,
            ),
        )
        .map_err(|err| {
            CliError::UnexpectedError(format!("failed to simulate txn with gas profiler: {}", err))
        })?;

    // Generate a human-readable name for the report
    let entry_point = gas_log.entry_point();

    let human_readable_name = match entry_point {
        FrameName::Script => "script".to_string(),
        FrameName::TransactionBatch => "transaction batch".to_string(),
        FrameName::Function {
            module_id, name, ..
        } => {
            let addr_short = module_id.address().short_str_lossless();
            let addr_truncated = if addr_short.len() > 4 {
                &addr_short[..4]
            } else {
                addr_short.as_str()
            };
            format!("0x{}-{}-{}", addr_truncated, module_id.name(), name)
        },
    };
    let raw_file_name = format!("txn-{}-{}", hash, human_readable_name);

    // Generate the report
    let path = Path::new("gas-profiling").join(raw_file_name);
    gas_log.generate_html_report(&path, format!("Gas Report - {}", human_readable_name))?;

    println!("Gas report saved to {}.", path.display());

    Ok((vm_status, vm_output))
}
