// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::MoveDebugger;
use aptos_cli_common::{CliError, CliTypedResult};
use aptos_crypto::HashValue;
use aptos_gas_profiling::FrameName;
use aptos_types::transaction::{AuxiliaryInfo, PersistedAuxiliaryInfo, SignedTransaction};
use aptos_validator_interface::LocalModuleOverrides;
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::{environment::AptosEnvironment, prod_configs};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, output::VMOutput, resolver::StateStorageView,
};
use move_core_types::vm_status::VMStatus;
use move_vm_runtime::source_locator::{self, SourceLocator};
use std::{path::Path, sync::Arc, time::Instant};

pub fn run_transaction_using_debugger(
    debugger: &dyn MoveDebugger,
    version: u64,
    transaction: SignedTransaction,
    _hash: HashValue,
    persisted_auxiliary_info: PersistedAuxiliaryInfo,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    // Required for MOVE_TRACE_EXEC and MOVE_VM_STEP: gates exec-state attachment
    // to errors and the step-debugger loop in the interpreter.
    prod_configs::set_debugging_enabled(true);

    let state_view = debugger.state_view_at_version(version);
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);

    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);

    let (vm_status, vm_output) = vm.execute_user_transaction(
        &resolver,
        &code_storage,
        &transaction,
        &log_context,
        &AuxiliaryInfo::new(persisted_auxiliary_info, None),
    );

    Ok((vm_status, vm_output))
}

/// Replay a transaction with local module overrides and an optional source
/// locator (for line mapping and named locals).
pub fn run_transaction_with_local_overrides(
    debugger: &dyn MoveDebugger,
    version: u64,
    transaction: SignedTransaction,
    persisted_auxiliary_info: PersistedAuxiliaryInfo,
    overrides: Arc<LocalModuleOverrides>,
    locator: Option<Arc<dyn SourceLocator>>,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    // Same as above; also enables debug::print output from local Move code.
    prod_configs::set_debugging_enabled(true);

    // Install the source locator for this thread so the VM interpreter can
    // resolve source lines, parameter names, and struct field names.
    // The guard ensures the locator is cleared even if execution panics.
    struct SourceLocatorGuard;
    impl Drop for SourceLocatorGuard {
        fn drop(&mut self) {
            source_locator::clear_source_locator();
        }
    }
    let _guard = if let Some(loc) = locator {
        source_locator::set_source_locator(loc);
        Some(SourceLocatorGuard)
    } else {
        None
    };

    let state_view = debugger.state_view_at_version_with_overrides(version, overrides);
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);
    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);

    let result = vm.execute_user_transaction(
        &resolver,
        &code_storage,
        &transaction,
        &log_context,
        &AuxiliaryInfo::new(persisted_auxiliary_info, None),
    );

    Ok(result)
}

pub fn benchmark_transaction_using_debugger(
    debugger: &dyn MoveDebugger,
    version: u64,
    transaction: SignedTransaction,
    _hash: HashValue,
    persisted_auxiliary_info: PersistedAuxiliaryInfo,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    let state_view = debugger.state_view_at_version(version);
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);

    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);
    let (vm_status, vm_output) = vm.execute_user_transaction(
        &resolver,
        &code_storage,
        &transaction,
        &log_context,
        &AuxiliaryInfo::new(persisted_auxiliary_info, None),
    );

    let time_cold = {
        let n = 15;

        let mut times = vec![];
        for _i in 0..n {
            // Create a new VM each time so to include code loading as part of the
            // total running time.
            let vm = AptosVM::new(&env);
            let code_storage = state_view.as_aptos_code_storage(&env);
            let log_context = AdapterLogSchema::new(state_view.id(), 0);

            let t1 = Instant::now();
            std::hint::black_box(vm.execute_user_transaction(
                &resolver,
                &code_storage,
                &transaction,
                &log_context,
                &AuxiliaryInfo::new(persisted_auxiliary_info, None),
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

        for _i in 0..n {
            // Reuse the existing VM with warm code cache so to measure only the
            // execution time.
            let t1 = Instant::now();
            std::hint::black_box(vm.execute_user_transaction(
                &resolver,
                &code_storage,
                &transaction,
                &log_context,
                &AuxiliaryInfo::new(persisted_auxiliary_info, None),
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

/// Runs the gas profiler's consistency checks on `log` and reports any
/// discrepancies using CLI-flavored messaging (including a pointer to
/// `--skip-gas-profiler-consistency-check`).
///
/// When `skip` is true, inconsistencies are emitted as warnings on stderr so
/// the user still gets a (potentially incomplete) gas report. Otherwise, the
/// first inconsistency causes a panic so the failure is loud.
fn handle_gas_profiler_consistency_check(log: &aptos_gas_profiling::TransactionGasLog, skip: bool) {
    let errors: Vec<_> = [
        log.exec_io.check_consistency(),
        log.storage.check_consistency(),
    ]
    .into_iter()
    .filter_map(Result::err)
    .collect();
    if errors.is_empty() {
        return;
    }
    // Collect all errors before reporting so a panic on the first doesn't hide
    // a second simultaneous failure.
    let combined = errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n\n");
    if skip {
        eprintln!(
            "warning: {combined}\n\
             (consistency check was bypassed via --skip-gas-profiler-consistency-check; \
             the generated gas report may be incomplete or inaccurate.)"
        );
    } else {
        panic!(
            "{combined}\n\nRerun with --skip-gas-profiler-consistency-check to bypass this \
             check and still produce a (possibly incomplete) gas report."
        );
    }
}

pub fn profile_transaction_using_debugger(
    debugger: &dyn MoveDebugger,
    version: u64,
    transaction: SignedTransaction,
    hash: HashValue,
    persisted_auxiliary_info: PersistedAuxiliaryInfo,
    fold_unique_stack: bool,
    skip_gas_profiler_consistency_check: bool,
) -> CliTypedResult<(VMStatus, VMOutput)> {
    let (vm_status, vm_output, mut gas_log) = debugger
        .execute_transaction_at_version_with_gas_profiler(
            version,
            transaction,
            AuxiliaryInfo::new(persisted_auxiliary_info, None),
        )
        .map_err(|err| {
            CliError::UnexpectedError(format!("failed to simulate txn with gas profiler: {}", err))
        })?;

    handle_gas_profiler_consistency_check(&gas_log, skip_gas_profiler_consistency_check);

    // Optionally fold the call graph by unique stack traces
    if fold_unique_stack {
        gas_log.exec_io.call_graph = gas_log.exec_io.call_graph.fold_unique_stack();
    }

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
