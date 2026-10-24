// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V1 harness: replays the whole transaction (prologue, payload, epilogue) on
//! the legacy AptosVM.
//!
//! This is the stock production execution path, with the production gas meter
//! charging nothing (see `gas.rs`), as V2 runs unmetered: the output carries no
//! fee effects and is byte-comparable with V2's. Each run builds the VM, environment and code storage on the state it runs on.

use crate::{gas::ZeroChargeAlgebra, overrides::ReplayView};
use anyhow::{anyhow, Result};
use aptos_gas_meter::{StandardGasAlgebra, StandardGasMeter};
use aptos_memory_usage_tracker::MemoryTrackedGasMeter;
use aptos_types::{
    state_store::StateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        ExecutionStatus, PersistedAuxiliaryInfo, Transaction, TransactionOutput, TransactionStatus,
    },
};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{module_and_script_storage::AsAptosCodeStorage, output::VMOutput};
use move_core_types::vm_status::{StatusCode, VMStatus};

/// What V1 did.
pub(crate) struct V1Run {
    pub output: TransactionOutput,
    /// Whether V1 stopped at a limit its meter enforces, read from its VM status as well as from the
    /// output's: before gas feature version `RELEASE_V1_38`, a stop at the memory limit is kept as
    /// a plain `ExecutionFailure`, which does not say so.
    pub hit_metering_limit: bool,
    /// V1's VM status, which says more than the output's status about why it failed (which native
    /// it lacks, for one).
    pub vm_status: VMStatus,
}

/// Why a capture or a state completion stops when V1 stopped at a metering limit its status on chain
/// does not show (see [`V1Run`]): replayed gas-free, as for a stop on chain.
#[derive(Debug)]
pub(crate) struct MeteringStop;

impl std::fmt::Display for MeteringStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "V1 stopped at a metering limit, which unmetered MonoMove does not enforce"
        )
    }
}

impl std::error::Error for MeteringStop {}

impl MeteringStop {
    /// Why capture or import skips a record that failed with `err`: a [`MeteringStop`] has a reason of
    /// its own, anything else `other`.
    pub(crate) fn skip_reason(err: &anyhow::Error, other: &'static str) -> &'static str {
        if err.is::<MeteringStop>() {
            "hit a metering limit in V1"
        } else {
            other
        }
    }
}

/// Whether `code` is a stop at a limit the gas meter enforces.
pub(crate) fn is_metering_limit(code: StatusCode) -> bool {
    matches!(
        code,
        StatusCode::OUT_OF_GAS
            | StatusCode::EXECUTION_LIMIT_REACHED
            | StatusCode::IO_LIMIT_REACHED
            | StatusCode::STORAGE_LIMIT_REACHED
            | StatusCode::MEMORY_LIMIT_EXCEEDED
            | StatusCode::DEPENDENCY_LIMIT_REACHED
    )
}

/// Whether a transaction committed with `status` on chain stopped at a limit the gas meter
/// enforces, which unmetered MonoMove does not.
pub(crate) fn hit_metering_limit_on_chain(status: &ExecutionStatus) -> bool {
    match status {
        ExecutionStatus::OutOfGas => true,
        ExecutionStatus::MiscellaneousError(Some(code)) => is_metering_limit(*code),
        ExecutionStatus::Success
        | ExecutionStatus::MoveAbort { .. }
        | ExecutionStatus::ExecutionFailure { .. }
        | ExecutionStatus::MiscellaneousError(None) => false,
    }
}

/// Whether the status is a stop at a limit the gas meter enforces, which unmetered MonoMove does
/// not.
pub(crate) fn hit_metering_limit(status: &TransactionStatus) -> bool {
    match status {
        TransactionStatus::Keep(status) => hit_metering_limit_on_chain(status),
        TransactionStatus::Discard(code) => is_metering_limit(*code),
        TransactionStatus::Retry => false,
    }
}

/// Executes `txn` on V1 against `state` and materializes its output. `state` is a replay's
/// [`ReplayView`] of the patched state (see [`run_caught`]), or, for capture, a view that records
/// the reads of the unpatched chain state.
pub(crate) fn execute<S: StateView + Sync>(
    state: &S,
    txn: &SignatureVerifiedTransaction,
    aux_info: &AuxiliaryInfo,
) -> Result<V1Run> {
    let env = AptosEnvironment::new(state);
    let vm = AptosVM::new(&env);
    let resolver = state.as_move_resolver();
    let code_storage = state.as_aptos_code_storage(&env);
    let log_context = AdapterLogSchema::new(state.id(), 0);

    let (vm_status, vm_output) = match txn {
        SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)) => {
            match vm.execute_user_transaction_with_custom_gas_meter(
                &resolver,
                &code_storage,
                txn,
                &log_context,
                |feature_version, vm_params, storage_params, limits, balance, kill_switch| {
                    MemoryTrackedGasMeter::new(StandardGasMeter::new(
                        ZeroChargeAlgebra(StandardGasAlgebra::new(
                            feature_version,
                            vm_params,
                            storage_params,
                            limits,
                            balance,
                            kill_switch,
                        )),
                        false,
                    ))
                },
                aux_info,
            ) {
                Ok((vm_status, vm_output, _)) => (vm_status, vm_output),
                // As `AptosVM::execute_user_transaction` reports a rejected transaction.
                Err(status) => {
                    let vm_output = VMOutput::empty_with_status(TransactionStatus::Discard(
                        status.status_code(),
                    ));
                    (status, vm_output)
                },
            }
        },
        // System transactions go through the block-level entry point, pre-marked as
        // signature-verified like the block executor does.
        SignatureVerifiedTransaction::Valid(
            Transaction::GenesisTransaction(_)
            | Transaction::BlockMetadata(_)
            | Transaction::BlockMetadataExt(_)
            | Transaction::BlockEpilogue(_)
            | Transaction::StateCheckpoint(_)
            | Transaction::ValidatorTransaction(_),
        )
        | SignatureVerifiedTransaction::Invalid(_) => vm
            .execute_single_transaction(txn, &resolver, &code_storage, &log_context, aux_info)
            .map_err(|status| anyhow!("V1 rejected the transaction: {:?}", status))?,
    };
    let output = vm_output
        .try_materialize_into_transaction_output()
        .map_err(|status| anyhow!("failed to materialize V1 output: {:?}", status))?;
    let hit_metering_limit =
        is_metering_limit(vm_status.status_code()) || hit_metering_limit(output.status());
    Ok(V1Run {
        output,
        hit_metering_limit,
        vm_status,
    })
}

/// Runs `txn` on V1 against `view` once, catching a panic: what a replay of a record does.
pub(crate) fn run_caught(
    view: &ReplayView,
    txn: &Transaction,
    aux_info: PersistedAuxiliaryInfo,
) -> std::thread::Result<Result<V1Run>> {
    let txn = SignatureVerifiedTransaction::Valid(txn.clone());
    let aux_info = AuxiliaryInfo::new(aux_info, None);
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute(view, &txn, &aux_info)
    }))
}
