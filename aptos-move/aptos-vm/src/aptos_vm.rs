// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{
        discard_error_output, discard_error_vm_status, PreprocessedTransaction, VMAdapter,
    },
    aptos_vm_impl::{get_transaction_output, AptosVMImpl, AptosVMInternals},
    block_executor::{AptosTransactionOutput, BlockAptosVM},
    counters::*,
    data_cache::StorageAdapter,
    errors::expect_only_successful_execution,
    move_vm_ext::{MoveResolverExt, RespawnedSession, SessionExt, SessionId},
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    system_module_names::*,
    transaction_metadata::TransactionMetadata,
    verifier, VMExecutor, VMValidator,
};
use anyhow::{anyhow, Result};
use aptos_block_executor::txn_commit_hook::NoOpTransactionCommitHook;
use aptos_crypto::HashValue;
use aptos_framework::natives::code::PublishRequest;
use aptos_gas_algebra::Gas;
use aptos_gas_meter::{AptosGasMeter, StandardGasAlgebra, StandardGasMeter};
use aptos_gas_schedule::VMGasParameters;
use aptos_logger::{enabled, prelude::*, Level};
use aptos_memory_usage_tracker::MemoryTrackedGasMeter;
use aptos_state_view::StateView;
use aptos_types::{
    account_config,
    account_config::new_block_event_key,
    block_executor::partitioner::PartitionedTransactions,
    block_metadata::BlockMetadata,
    fee_statement::FeeStatement,
    on_chain_config::{new_epoch_event_key, FeatureFlag, TimedFeatureOverride},
    state_store::state_key::StateKey,
    transaction::{
        EntryFunction, ExecutionError, ExecutionStatus, ModuleBundle, Multisig,
        MultisigTransactionPayload, SignatureCheckedTransaction, SignedTransaction, Transaction,
        TransactionOutput, TransactionPayload, TransactionStatus, VMValidatorResult,
        WriteSetPayload,
    },
    vm_status::{AbortLocation, StatusCode, VMStatus},
    write_set::WriteOp,
};
use aptos_utils::{aptos_try, return_on_failure};
use aptos_vm_logging::{log_schema::AdapterLogSchema, speculative_error, speculative_log};
use aptos_vm_types::{
    change_set::VMChangeSet,
    output::VMOutput,
    storage::{ChangeSetConfigs, StorageGasParameters},
};
use fail::fail_point;
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{verification_error, Location, PartialVMError, VMError, VMResult},
    CompiledModule, IndexKind,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    transaction_argument::convert_txn_args,
    value::{serialize_values, MoveValue},
    vm_status::StatusType,
};
use move_vm_runtime::session::SerializedReturnValues;
use move_vm_types::gas::UnmeteredGasMeter;
use num_cpus;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cmp::{max, min},
    collections::{BTreeMap, BTreeSet},
    convert::{AsMut, AsRef},
    marker::Sync,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

static EXECUTION_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static NUM_EXECUTION_SHARD: OnceCell<usize> = OnceCell::new();
static NUM_PROOF_READING_THREADS: OnceCell<usize> = OnceCell::new();
static PARANOID_TYPE_CHECKS: OnceCell<bool> = OnceCell::new();
static PROCESSED_TRANSACTIONS_DETAILED_COUNTERS: OnceCell<bool> = OnceCell::new();
static TIMED_FEATURE_OVERRIDE: OnceCell<TimedFeatureOverride> = OnceCell::new();

pub static RAYON_EXEC_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .thread_name(|index| format!("par_exec_{}", index))
            .build()
            .unwrap(),
    )
});

/// Remove this once the bundle is removed from the code.
static MODULE_BUNDLE_DISALLOWED: AtomicBool = AtomicBool::new(true);
pub fn allow_module_bundle_for_test() {
    MODULE_BUNDLE_DISALLOWED.store(false, Ordering::Relaxed);
}

pub struct AptosVM(pub(crate) AptosVMImpl);

struct AptosSimulationVM(AptosVM);

macro_rules! unwrap_or_discard {
    ($res:expr) => {
        match $res {
            Ok(s) => s,
            Err(e) => return discard_error_vm_status(e),
        }
    };
}

impl AptosVM {
    pub fn new(state: &impl StateView) -> Self {
        Self(AptosVMImpl::new(state))
    }

    pub fn new_for_validation(state: &impl StateView) -> Self {
        info!(
            AdapterLogSchema::new(state.id(), 0),
            "Adapter created for Validation"
        );
        Self::new(state)
    }

    /// Sets execution concurrency level when invoked the first time.
    pub fn set_concurrency_level_once(mut concurrency_level: usize) {
        concurrency_level = min(concurrency_level, num_cpus::get());
        // Only the first call succeeds, due to OnceCell semantics.
        EXECUTION_CONCURRENCY_LEVEL.set(concurrency_level).ok();
    }

    /// Get the concurrency level if already set, otherwise return default 1
    /// (sequential execution).
    ///
    /// The concurrency level is fixed to 1 if gas profiling is enabled.
    pub fn get_concurrency_level() -> usize {
        match EXECUTION_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 1,
        }
    }

    pub fn set_num_shards_once(mut num_shards: usize) {
        num_shards = max(num_shards, 1);
        // Only the first call succeeds, due to OnceCell semantics.
        NUM_EXECUTION_SHARD.set(num_shards).ok();
    }

    pub fn get_num_shards() -> usize {
        match NUM_EXECUTION_SHARD.get() {
            Some(num_shards) => *num_shards,
            None => 1,
        }
    }

    /// Sets runtime config when invoked the first time.
    pub fn set_paranoid_type_checks(enable: bool) {
        // Only the first call succeeds, due to OnceCell semantics.
        PARANOID_TYPE_CHECKS.set(enable).ok();
    }

    /// Get the paranoid type check flag if already set, otherwise return default true
    pub fn get_paranoid_checks() -> bool {
        match PARANOID_TYPE_CHECKS.get() {
            Some(enable) => *enable,
            None => true,
        }
    }

    // Set the override profile for timed features.
    pub fn set_timed_feature_override(profile: TimedFeatureOverride) {
        TIMED_FEATURE_OVERRIDE.set(profile).ok();
    }

    pub fn get_timed_feature_override() -> Option<TimedFeatureOverride> {
        TIMED_FEATURE_OVERRIDE.get().cloned()
    }

    /// Sets the # of async proof reading threads.
    pub fn set_num_proof_reading_threads_once(mut num_threads: usize) {
        // TODO(grao): Do more analysis to tune this magic number.
        num_threads = min(num_threads, 256);
        // Only the first call succeeds, due to OnceCell semantics.
        NUM_PROOF_READING_THREADS.set(num_threads).ok();
    }

    /// Returns the # of async proof reading threads if already set, otherwise return default value
    /// (32).
    pub fn get_num_proof_reading_threads() -> usize {
        match NUM_PROOF_READING_THREADS.get() {
            Some(num_threads) => *num_threads,
            None => 32,
        }
    }

    /// Sets addigional details in counters when invoked the first time.
    pub fn set_processed_transactions_detailed_counters() {
        // Only the first call succeeds, due to OnceCell semantics.
        PROCESSED_TRANSACTIONS_DETAILED_COUNTERS.set(true).ok();
    }

    /// Get whether we should capture additional details in counters
    pub fn get_processed_transactions_detailed_counters() -> bool {
        match PROCESSED_TRANSACTIONS_DETAILED_COUNTERS.get() {
            Some(value) => *value,
            None => false,
        }
    }

    pub fn internals(&self) -> AptosVMInternals {
        AptosVMInternals::new(&self.0)
    }

    /// Load a module into its internal MoveVM's code cache.
    pub fn load_module(
        &self,
        module_id: &ModuleId,
        resolver: &impl MoveResolverExt,
    ) -> VMResult<Arc<CompiledModule>> {
        self.0.load_module(module_id, resolver)
    }

    /// Generates a transaction output for a transaction that encountered errors during the
    /// execution process. This is public for now only for tests.
    pub fn failed_transaction_cleanup(
        &self,
        error_code: VMStatus,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl MoveResolverExt,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
    ) -> VMOutput {
        self.failed_transaction_cleanup_and_keep_vm_status(
            error_code,
            gas_meter,
            txn_data,
            resolver,
            log_context,
            change_set_configs,
        )
        .1
    }

    pub fn as_move_resolver<'a, S: StateView>(&self, state_view: &'a S) -> StorageAdapter<'a, S> {
        StorageAdapter::new_with_cached_config(
            state_view,
            self.0.get_gas_feature_version(),
            self.0.get_features(),
        )
    }

    fn fee_statement_from_gas_meter(
        txn_data: &TransactionMetadata,
        gas_meter: &impl AptosGasMeter,
    ) -> FeeStatement {
        let gas_used = txn_data
            .max_gas_amount()
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount");
        FeeStatement::new(
            gas_used.into(),
            u64::from(gas_meter.execution_gas_used()),
            u64::from(gas_meter.io_gas_used()),
            u64::from(gas_meter.storage_fee_used_in_gas_units()),
            u64::from(gas_meter.storage_fee_used()),
        )
    }

    fn failed_transaction_cleanup_and_keep_vm_status(
        &self,
        error_code: VMStatus,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl MoveResolverExt,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
    ) -> (VMStatus, VMOutput) {
        let mut session = self
            .0
            .new_session(resolver, SessionId::epilogue_meta(txn_data));

        match TransactionStatus::from_vm_status(
            error_code.clone(),
            self.0
                .get_features()
                .is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION),
        ) {
            TransactionStatus::Keep(status) => {
                // Inject abort info if available.
                let status = match status {
                    ExecutionStatus::MoveAbort {
                        location: AbortLocation::Module(module),
                        code,
                        ..
                    } => {
                        let info = self.0.extract_abort_info(&module, code);
                        ExecutionStatus::MoveAbort {
                            location: AbortLocation::Module(module),
                            code,
                            info,
                        }
                    },
                    _ => status,
                };
                // The transaction should be charged for gas, so run the epilogue to do that.
                // This is running in a new session that drops any side effects from the
                // attempted transaction (e.g., spending funds that were needed to pay for gas),
                // so even if the previous failure occurred while running the epilogue, it
                // should not fail now. If it somehow fails here, there is no choice but to
                // discard the transaction.
                if let Err(e) = self.0.run_failure_epilogue(
                    &mut session,
                    gas_meter.balance(),
                    txn_data,
                    log_context,
                ) {
                    return discard_error_vm_status(e);
                }
                let fee_statement = AptosVM::fee_statement_from_gas_meter(txn_data, gas_meter);
                let txn_output = get_transaction_output(
                    &mut (),
                    session,
                    fee_statement,
                    status,
                    change_set_configs,
                )
                .unwrap_or_else(|e| discard_error_vm_status(e).1);
                (error_code, txn_output)
            },
            TransactionStatus::Discard(status) => {
                (VMStatus::error(status, None), discard_error_output(status))
            },
            TransactionStatus::Retry => unreachable!(),
        }
    }

    fn success_transaction_cleanup(
        &self,
        mut respawned_session: RespawnedSession,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        respawned_session.execute(|session| {
            self.0
                .run_success_epilogue(session, gas_meter.balance(), txn_data, log_context)
        })?;
        let change_set = respawned_session.finish(change_set_configs)?;
        let fee_statement = AptosVM::fee_statement_from_gas_meter(txn_data, gas_meter);
        let output = VMOutput::new(
            change_set,
            fee_statement,
            TransactionStatus::Keep(ExecutionStatus::Success),
        );

        Ok((VMStatus::Executed, output))
    }

    fn validate_and_execute_entry_function(
        &self,
        session: &mut SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        senders: Vec<AccountAddress>,
        script_fn: &EntryFunction,
    ) -> Result<SerializedReturnValues, VMStatus> {
        let function = session.load_function(
            script_fn.module(),
            script_fn.function(),
            script_fn.ty_args(),
        )?;
        let struct_constructors = self
            .0
            .get_features()
            .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS);
        let args = verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
            session,
            senders,
            script_fn.args().to_vec(),
            &function,
            struct_constructors,
        )?;
        Ok(session.execute_entry_function(
            script_fn.module(),
            script_fn.function(),
            script_fn.ty_args().to_vec(),
            args,
            gas_meter,
        )?)
    }

    fn execute_script_or_entry_function(
        &self,
        resolver: &impl MoveResolverExt,
        mut session: SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        payload: &TransactionPayload,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        fail_point!("move_adapter::execute_script_or_entry_function", |_| {
            Err(VMStatus::Error {
                status_code: StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                sub_status: Some(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
                message: None,
            })
        });

        // Run the execution logic
        {
            gas_meter.charge_intrinsic_gas_for_transaction(txn_data.transaction_size())?;

            match payload {
                TransactionPayload::Script(script) => {
                    let loaded_func =
                        session.load_script(script.code(), script.ty_args().to_vec())?;
                    let args =
                        verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
                            &mut session,
                            txn_data.senders(),
                            convert_txn_args(script.args()),
                            &loaded_func,
                            self.0
                                .get_features()
                                .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
                        )?;
                    session.execute_script(
                        script.code(),
                        script.ty_args().to_vec(),
                        args,
                        gas_meter,
                    )?;
                },
                TransactionPayload::EntryFunction(script_fn) => {
                    self.validate_and_execute_entry_function(
                        &mut session,
                        gas_meter,
                        txn_data.senders(),
                        script_fn,
                    )?;
                },

                // Not reachable as this function should only be invoked for entry or script
                // transaction payload.
                _ => {
                    return Err(VMStatus::error(StatusCode::UNREACHABLE, None));
                },
            };

            self.resolve_pending_code_publish(
                &mut session,
                gas_meter,
                new_published_modules_loaded,
            )?;

            let respawned_session = self.charge_change_set_and_respawn_session(
                session,
                resolver,
                gas_meter,
                change_set_configs,
                txn_data,
            )?;

            self.success_transaction_cleanup(
                respawned_session,
                gas_meter,
                txn_data,
                log_context,
                change_set_configs,
            )
        }
    }

    fn charge_change_set_and_respawn_session<'r, 'l>(
        &'l self,
        session: SessionExt,
        resolver: &'r impl MoveResolverExt,
        gas_meter: &mut impl AptosGasMeter,
        change_set_configs: &ChangeSetConfigs,
        txn_data: &TransactionMetadata,
    ) -> Result<RespawnedSession<'r, 'l>, VMStatus> {
        let change_set = session.finish(&mut (), change_set_configs)?;

        for (key, op) in change_set.write_set_iter() {
            gas_meter.charge_io_gas_for_write(key, op)?;
        }

        gas_meter.charge_storage_fee_for_all(
            change_set.write_set_iter(),
            change_set.events(),
            txn_data.transaction_size,
            txn_data.gas_unit_price,
        )?;

        // TODO(Gas): Charge for aggregator writes
        let session_id = SessionId::epilogue_meta(txn_data);
        RespawnedSession::spawn(&self.0, session_id, resolver, change_set)
    }

    // Execute a multisig transaction:
    // 1. Obtain the payload of the transaction to execute. This could have been stored on chain
    // when the multisig transaction was created.
    // 2. Execute the target payload. If this fails, discard the session and keep the gas meter and
    // failure object. In case of success, keep the session and also do any necessary module publish
    // cleanup.
    // 3. Call post transaction cleanup function in multisig account module with the result from (2)
    fn execute_multisig_transaction(
        &self,
        resolver: &impl MoveResolverExt,
        mut session: SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        txn_payload: &Multisig,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        fail_point!("move_adapter::execute_multisig_transaction", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        gas_meter.charge_intrinsic_gas_for_transaction(txn_data.transaction_size())?;

        // Step 1: Obtain the payload. If any errors happen here, the entire transaction should fail
        let invariant_violation_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("MultiSig transaction error".to_string())
                .finish(Location::Undefined)
        };
        let provided_payload = if let Some(payload) = &txn_payload.transaction_payload {
            bcs::to_bytes(&payload).map_err(|_| invariant_violation_error())?
        } else {
            // Default to empty bytes if payload is not provided.
            bcs::to_bytes::<Vec<u8>>(&vec![]).map_err(|_| invariant_violation_error())?
        };
        // Failures here will be propagated back.
        let payload_bytes: Vec<Vec<u8>> = session
            .execute_function_bypass_visibility(
                &MULTISIG_ACCOUNT_MODULE,
                GET_NEXT_TRANSACTION_PAYLOAD,
                vec![],
                serialize_values(&vec![
                    MoveValue::Address(txn_payload.multisig_address),
                    MoveValue::vector_u8(provided_payload),
                ]),
                gas_meter,
            )?
            .return_values
            .into_iter()
            .map(|(bytes, _ty)| bytes)
            .collect::<Vec<_>>();
        let payload_bytes = payload_bytes
            .first()
            // We expect the payload to either exists on chain or be passed along with the
            // transaction.
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Multisig payload bytes return error".to_string())
                    .finish(Location::Undefined)
            })?;
        // We have to deserialize twice as the first time returns the actual return type of the
        // function, which is vec<u8>. The second time deserializes it into the correct
        // EntryFunction payload type.
        // If either deserialization fails for some reason, that means the user provided incorrect
        // payload data either during transaction creation or execution.
        let deserialization_error = PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
            .finish(Location::Undefined);
        let payload_bytes =
            bcs::from_bytes::<Vec<u8>>(payload_bytes).map_err(|_| deserialization_error.clone())?;
        let payload = bcs::from_bytes::<MultisigTransactionPayload>(&payload_bytes)
            .map_err(|_| deserialization_error)?;

        // Step 2: Execute the target payload. Transaction failure here is tolerated. In case of any
        // failures, we'll discard the session and start a new one. This ensures that any data
        // changes are not persisted.
        // The multisig transaction would still be considered executed even if execution fails.
        let execution_result = match payload {
            MultisigTransactionPayload::EntryFunction(entry_function) => self
                .execute_multisig_entry_function(
                    &mut session,
                    gas_meter,
                    txn_payload.multisig_address,
                    &entry_function,
                    new_published_modules_loaded,
                ),
        };

        // Step 3: Call post transaction cleanup function in multisig account module with the result
        // from Step 2.
        // Note that we don't charge execution or writeset gas for cleanup routines. This is
        // consistent with the high-level success/failure cleanup routines for user transactions.
        let cleanup_args = serialize_values(&vec![
            MoveValue::Address(txn_data.sender),
            MoveValue::Address(txn_payload.multisig_address),
            MoveValue::vector_u8(payload_bytes),
        ]);
        let respawned_session = if let Err(execution_error) = execution_result {
            // Invalidate the loader cache in case there was a new module loaded from a module
            // publish request that failed.
            // This is redundant with the logic in execute_user_transaction but unfortunately is
            // necessary here as executing the underlying call can fail without this function
            // returning an error to execute_user_transaction.
            if *new_published_modules_loaded {
                self.0.mark_loader_cache_as_invalid();
            };
            self.failure_multisig_payload_cleanup(
                resolver,
                execution_error,
                txn_data,
                cleanup_args,
            )?
        } else {
            self.success_multisig_payload_cleanup(
                resolver,
                session,
                gas_meter,
                txn_data,
                cleanup_args,
                change_set_configs,
            )?
        };

        // TODO(Gas): Charge for aggregator writes
        self.success_transaction_cleanup(
            respawned_session,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
        )
    }

    fn execute_multisig_entry_function(
        &self,
        session: &mut SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        multisig_address: AccountAddress,
        payload: &EntryFunction,
        new_published_modules_loaded: &mut bool,
    ) -> Result<(), VMStatus> {
        // If txn args are not valid, we'd still consider the transaction as executed but
        // failed. This is primarily because it's unrecoverable at this point.
        self.validate_and_execute_entry_function(
            session,
            gas_meter,
            vec![multisig_address],
            payload,
        )?;

        // Resolve any pending module publishes in case the multisig transaction is deploying
        // modules.
        self.resolve_pending_code_publish(session, gas_meter, new_published_modules_loaded)?;
        Ok(())
    }

    fn success_multisig_payload_cleanup<'r, 'l>(
        &'l self,
        resolver: &'r impl MoveResolverExt,
        session: SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        cleanup_args: Vec<Vec<u8>>,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<RespawnedSession<'r, 'l>, VMStatus> {
        // Charge gas for writeset before we do cleanup. This ensures we don't charge gas for
        // cleanup writeset changes, which is consistent with outer-level success cleanup
        // flow. We also wouldn't need to worry that we run out of gas when doing cleanup.
        let mut respawned_session = self.charge_change_set_and_respawn_session(
            session,
            resolver,
            gas_meter,
            change_set_configs,
            txn_data,
        )?;
        respawned_session.execute(|session| {
            session.execute_function_bypass_visibility(
                &MULTISIG_ACCOUNT_MODULE,
                SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP,
                vec![],
                cleanup_args,
                &mut UnmeteredGasMeter,
            )
        })?;
        Ok(respawned_session)
    }

    fn failure_multisig_payload_cleanup<'r, 'l>(
        &'l self,
        resolver: &'r impl MoveResolverExt,
        execution_error: VMStatus,
        txn_data: &TransactionMetadata,
        mut cleanup_args: Vec<Vec<u8>>,
    ) -> Result<RespawnedSession<'r, 'l>, VMStatus> {
        // Start a fresh session for running cleanup that does not contain any changes from
        // the inner function call earlier (since it failed).
        let mut respawned_session = RespawnedSession::spawn(
            &self.0,
            SessionId::epilogue_meta(txn_data),
            resolver,
            VMChangeSet::empty(),
        )?;

        let execution_error = ExecutionError::try_from(execution_error)
            .map_err(|_| VMStatus::error(StatusCode::UNREACHABLE, None))?;
        // Serialization is not expected to fail so we're using invariant_violation error here.
        cleanup_args.push(bcs::to_bytes(&execution_error).map_err(|_| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("MultiSig payload cleanup error.".to_string())
                .finish(Location::Undefined)
        })?);
        respawned_session.execute(|session| {
            session.execute_function_bypass_visibility(
                &MULTISIG_ACCOUNT_MODULE,
                FAILED_TRANSACTION_EXECUTION_CLEANUP,
                vec![],
                cleanup_args,
                &mut UnmeteredGasMeter,
            )
        })?;
        Ok(respawned_session)
    }

    fn verify_module_bundle(
        session: &mut SessionExt,
        module_bundle: &ModuleBundle,
    ) -> VMResult<()> {
        for module_blob in module_bundle.iter() {
            match CompiledModule::deserialize(module_blob.code()) {
                Ok(module) => {
                    // verify the module doesn't exist
                    if session.load_module(&module.self_id()).is_ok() {
                        return Err(verification_error(
                            StatusCode::DUPLICATE_MODULE_NAME,
                            IndexKind::AddressIdentifier,
                            module.self_handle_idx().0,
                        )
                        .finish(Location::Undefined));
                    }
                },
                Err(err) => return Err(err.finish(Location::Undefined)),
            }
        }
        Ok(())
    }

    /// Execute all module initializers.
    fn execute_module_initialization(
        &self,
        session: &mut SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        modules: &[CompiledModule],
        exists: BTreeSet<ModuleId>,
        senders: &[AccountAddress],
        new_published_modules_loaded: &mut bool,
    ) -> VMResult<()> {
        let init_func_name = ident_str!("init_module");
        for module in modules {
            if exists.contains(&module.self_id()) {
                // Call initializer only on first publish.
                continue;
            }
            *new_published_modules_loaded = true;
            let init_function = session.load_function(&module.self_id(), init_func_name, &[]);
            // it is ok to not have init_module function
            // init_module function should be (1) private and (2) has no return value
            // Note that for historic reasons, verification here is treated
            // as StatusCode::CONSTRAINT_NOT_SATISFIED, there this cannot be unified
            // with the general verify_module above.
            if init_function.is_ok() {
                if verifier::module_init::verify_module_init_function(module).is_ok() {
                    let args: Vec<Vec<u8>> = senders
                        .iter()
                        .map(|s| MoveValue::Signer(*s).simple_serialize().unwrap())
                        .collect();
                    session.execute_function_bypass_visibility(
                        &module.self_id(),
                        init_func_name,
                        vec![],
                        args,
                        gas_meter,
                    )?;
                } else {
                    return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
                        .finish(Location::Undefined));
                }
            }
        }
        Ok(())
    }

    /// Deserialize a module bundle.
    fn deserialize_module_bundle(&self, modules: &ModuleBundle) -> VMResult<Vec<CompiledModule>> {
        let max_version = if self
            .0
            .get_features()
            .is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6)
        {
            6
        } else {
            5
        };
        let mut result = vec![];
        for module_blob in modules.iter() {
            match CompiledModule::deserialize_with_max_version(module_blob.code(), max_version) {
                Ok(module) => {
                    result.push(module);
                },
                Err(_err) => {
                    return Err(PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                        .finish(Location::Undefined))
                },
            }
        }
        Ok(result)
    }

    /// Execute a module bundle load request.
    /// TODO: this is going to be deprecated and removed in favor of code publishing via
    /// NativeCodeContext
    fn execute_modules(
        &self,
        resolver: &impl MoveResolverExt,
        mut session: SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        modules: &ModuleBundle,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        if MODULE_BUNDLE_DISALLOWED.load(Ordering::Relaxed) {
            return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
        }
        fail_point!("move_adapter::execute_module", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        gas_meter.charge_intrinsic_gas_for_transaction(txn_data.transaction_size())?;

        Self::verify_module_bundle(&mut session, modules)?;
        session.publish_module_bundle_with_compat_config(
            modules.clone().into_inner(),
            txn_data.sender(),
            gas_meter,
            Compatibility::new(
                true,
                true,
                !self
                    .0
                    .get_features()
                    .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
            ),
        )?;

        // call init function of the each module
        self.execute_module_initialization(
            &mut session,
            gas_meter,
            &self.deserialize_module_bundle(modules)?,
            BTreeSet::new(),
            &[txn_data.sender()],
            new_published_modules_loaded,
        )?;

        let respawned_session = self.charge_change_set_and_respawn_session(
            session,
            resolver,
            gas_meter,
            change_set_configs,
            txn_data,
        )?;

        self.success_transaction_cleanup(
            respawned_session,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
        )
    }

    /// Resolve a pending code publish request registered via the NativeCodeContext.
    fn resolve_pending_code_publish(
        &self,
        session: &mut SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        new_published_modules_loaded: &mut bool,
    ) -> VMResult<()> {
        if let Some(PublishRequest {
            destination,
            bundle,
            expected_modules,
            allowed_deps,
            check_compat: _,
        }) = session.extract_publish_request()
        {
            // TODO: unfortunately we need to deserialize the entire bundle here to handle
            // `init_module` and verify some deployment conditions, while the VM need to do
            // the deserialization again. Consider adding an API to MoveVM which allows to
            // directly pass CompiledModule.
            let modules = self.deserialize_module_bundle(&bundle)?;

            // Validate the module bundle
            self.validate_publish_request(session, &modules, expected_modules, allowed_deps)?;

            // Check what modules exist before publishing.
            let mut exists = BTreeSet::new();
            for m in &modules {
                let id = m.self_id();
                if session.exists_module(&id)? {
                    exists.insert(id);
                }
            }

            // Publish the bundle and execute initializers
            // publish_module_bundle doesn't actually load the published module into
            // the loader cache. It only puts the module data in the data cache.
            return_on_failure!(session.publish_module_bundle_with_compat_config(
                bundle.into_inner(),
                destination,
                gas_meter,
                Compatibility::new(
                    true,
                    true,
                    !self
                        .0
                        .get_features()
                        .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
                ),
            ));

            self.execute_module_initialization(
                session,
                gas_meter,
                &modules,
                exists,
                &[destination],
                new_published_modules_loaded,
            )
        } else {
            Ok(())
        }
    }

    /// Validate a publish request.
    fn validate_publish_request(
        &self,
        session: &mut SessionExt,
        modules: &[CompiledModule],
        mut expected_modules: BTreeSet<String>,
        allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
    ) -> VMResult<()> {
        for m in modules {
            if !expected_modules.remove(m.self_id().name().as_str()) {
                return Err(Self::metadata_validation_error(&format!(
                    "unregistered module: '{}'",
                    m.self_id().name()
                )));
            }
            if let Some(allowed) = &allowed_deps {
                for dep in m.immediate_dependencies() {
                    if !allowed
                        .get(dep.address())
                        .map(|modules| {
                            modules.contains("") || modules.contains(dep.name().as_str())
                        })
                        .unwrap_or(false)
                    {
                        return Err(Self::metadata_validation_error(&format!(
                            "unregistered dependency: '{}'",
                            dep
                        )));
                    }
                }
            }
            aptos_framework::verify_module_metadata(m, self.0.get_features())
                .map_err(|err| Self::metadata_validation_error(&err.to_string()))?;
        }
        verifier::resource_groups::validate_resource_groups(session, modules)?;

        if !expected_modules.is_empty() {
            return Err(Self::metadata_validation_error(
                "not all registered modules published",
            ));
        }
        Ok(())
    }

    fn metadata_validation_error(msg: &str) -> VMError {
        PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
            .with_message(format!("metadata and code bundle mismatch: {}", msg))
            .finish(Location::Undefined)
    }

    fn make_standard_gas_meter(
        &self,
        balance: Gas,
        log_context: &AdapterLogSchema,
    ) -> Result<MemoryTrackedGasMeter<StandardGasMeter<StandardGasAlgebra>>, VMStatus> {
        Ok(MemoryTrackedGasMeter::new(StandardGasMeter::new(
            StandardGasAlgebra::new(
                self.0.get_gas_feature_version(),
                self.0.get_gas_parameters(log_context)?.vm.clone(),
                self.0.get_storage_gas_parameters(log_context)?.clone(),
                balance,
            ),
        )))
    }

    fn execute_user_transaction_impl(
        &self,
        resolver: &impl MoveResolverExt,
        txn: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
        gas_meter: &mut impl AptosGasMeter,
    ) -> (VMStatus, VMOutput) {
        // Revalidate the transaction.
        let mut session = self.0.new_session(resolver, SessionId::prologue(txn));
        if let Err(err) = self.validate_signature_checked_transaction(
            &mut session,
            resolver,
            txn,
            false,
            log_context,
        ) {
            return discard_error_vm_status(err);
        };

        if self.0.get_gas_feature_version() >= 1 {
            // Create a new session so that the data cache is flushed.
            // This is to ensure we correctly charge for loading certain resources, even if they
            // have been previously cached in the prologue.
            //
            // TODO(Gas): Do this in a better way in the future, perhaps without forcing the data cache to be flushed.
            // By releasing resource group cache, we start with a fresh slate for resource group
            // cost accounting.
            resolver.release_resource_group_cache();
            session = self.0.new_session(resolver, SessionId::txn(txn));
        }

        let storage_gas_params = unwrap_or_discard!(self.0.get_storage_gas_parameters(log_context));
        let txn_data = TransactionMetadata::new(txn);

        // We keep track of whether any newly published modules are loaded into the Vm's loader
        // cache as part of executing transactions. This would allow us to decide whether the cache
        // should be flushed later.
        let mut new_published_modules_loaded = false;
        let result = match txn.payload() {
            payload @ TransactionPayload::Script(_)
            | payload @ TransactionPayload::EntryFunction(_) => self
                .execute_script_or_entry_function(
                    resolver,
                    session,
                    gas_meter,
                    &txn_data,
                    payload,
                    log_context,
                    &mut new_published_modules_loaded,
                    &storage_gas_params.change_set_configs,
                ),
            TransactionPayload::Multisig(payload) => self.execute_multisig_transaction(
                resolver,
                session,
                gas_meter,
                &txn_data,
                payload,
                log_context,
                &mut new_published_modules_loaded,
                &storage_gas_params.change_set_configs,
            ),

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundle(m) => self.execute_modules(
                resolver,
                session,
                gas_meter,
                &txn_data,
                m,
                log_context,
                &mut new_published_modules_loaded,
                &storage_gas_params.change_set_configs,
            ),
        };

        let gas_usage = txn_data
            .max_gas_amount()
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount set");
        TXN_GAS_USAGE.observe(u64::from(gas_usage) as f64);

        match result {
            Ok(output) => output,
            Err(err) => {
                // Invalidate the loader cache in case there was a new module loaded from a module
                // publish request that failed.
                // This ensures the loader cache is flushed later to align storage with the cache.
                // None of the modules in the bundle will be committed to storage,
                // but some of them may have ended up in the cache.
                if new_published_modules_loaded {
                    self.0.mark_loader_cache_as_invalid();
                };

                let txn_status = TransactionStatus::from_vm_status(
                    err.clone(),
                    self.0
                        .get_features()
                        .is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION),
                );
                if txn_status.is_discarded() {
                    discard_error_vm_status(err)
                } else {
                    self.failed_transaction_cleanup_and_keep_vm_status(
                        err,
                        gas_meter,
                        &txn_data,
                        resolver,
                        log_context,
                        &storage_gas_params.change_set_configs,
                    )
                }
            },
        }
    }

    fn execute_user_transaction(
        &self,
        resolver: &impl MoveResolverExt,
        txn: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, VMOutput) {
        let balance = TransactionMetadata::new(txn).max_gas_amount();
        // TODO: would we end up having a diverging behavior by creating the gas meter at an earlier time?
        let mut gas_meter = unwrap_or_discard!(self.make_standard_gas_meter(balance, log_context));

        self.execute_user_transaction_impl(resolver, txn, log_context, &mut gas_meter)
    }

    pub fn execute_user_transaction_with_custom_gas_meter<G, F>(
        state_view: &impl StateView,
        txn: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
        make_gas_meter: F,
    ) -> Result<(VMStatus, VMOutput, G), VMStatus>
    where
        G: AptosGasMeter,
        F: FnOnce(u64, VMGasParameters, StorageGasParameters, Gas) -> Result<G, VMStatus>,
    {
        // TODO(Gas): revisit this.
        let vm = AptosVM::new(state_view);

        // TODO(Gas): avoid creating txn metadata twice.
        let balance = TransactionMetadata::new(txn).max_gas_amount();
        let mut gas_meter = make_gas_meter(
            vm.0.get_gas_feature_version(),
            vm.0.get_gas_parameters(log_context)?.vm.clone(),
            vm.0.get_storage_gas_parameters(log_context)?.clone(),
            balance,
        )?;

        let resolver = StorageAdapter::new_with_cached_config(
            state_view,
            vm.0.get_gas_feature_version(),
            vm.0.get_features(),
        );
        let (status, output) =
            vm.execute_user_transaction_impl(&resolver, txn, log_context, &mut gas_meter);

        Ok((status, output, gas_meter))
    }

    fn execute_writeset(
        &self,
        resolver: &impl MoveResolverExt,
        writeset_payload: &WriteSetPayload,
        txn_sender: Option<AccountAddress>,
        session_id: SessionId,
    ) -> Result<VMChangeSet, VMStatus> {
        let mut gas_meter = UnmeteredGasMeter;
        let change_set_configs =
            ChangeSetConfigs::unlimited_at_gas_feature_version(self.0.get_gas_feature_version());

        match writeset_payload {
            WriteSetPayload::Direct(change_set) => {
                VMChangeSet::try_from_storage_change_set(change_set.clone(), &change_set_configs)
            },
            WriteSetPayload::Script { script, execute_as } => {
                let mut tmp_session = self.0.new_session(resolver, session_id);
                let senders = match txn_sender {
                    None => vec![*execute_as],
                    Some(sender) => vec![sender, *execute_as],
                };

                let loaded_func =
                    tmp_session.load_script(script.code(), script.ty_args().to_vec())?;
                let args =
                    verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
                        &mut tmp_session,
                        senders,
                        convert_txn_args(script.args()),
                        &loaded_func,
                        self.0
                            .get_features()
                            .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
                    )?;

                return_on_failure!(tmp_session.execute_script(
                    script.code(),
                    script.ty_args().to_vec(),
                    args,
                    &mut gas_meter,
                ));
                Ok(tmp_session.finish(&mut (), &change_set_configs)?)
            },
        }
    }

    fn read_writeset<'a>(
        &self,
        state_view: &impl StateView,
        write_set: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> Result<(), VMStatus> {
        // All Move executions satisfy the read-before-write property. Thus we need to read each
        // access path that the write set is going to update.
        for (state_key, _) in write_set.into_iter() {
            state_view
                .get_state_value_bytes(state_key)
                .map_err(|_| VMStatus::error(StatusCode::STORAGE_ERROR, None))?;
        }
        Ok(())
    }

    fn validate_waypoint_change_set(
        change_set: &VMChangeSet,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let has_new_block_event = change_set
            .events()
            .iter()
            .any(|e| *e.key() == new_block_event_key());
        let has_new_epoch_event = change_set
            .events()
            .iter()
            .any(|e| *e.key() == new_epoch_event_key());
        if has_new_block_event && has_new_epoch_event {
            Ok(())
        } else {
            error!(
                *log_context,
                "[aptos_vm] waypoint txn needs to emit new epoch and block"
            );
            Err(VMStatus::error(StatusCode::INVALID_WRITE_SET, None))
        }
    }

    pub(crate) fn process_waypoint_change_set(
        &self,
        resolver: &impl MoveResolverExt,
        writeset_payload: WriteSetPayload,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        // TODO: user specified genesis id to distinguish different genesis write sets
        let genesis_id = HashValue::zero();
        let change_set = self.execute_writeset(
            resolver,
            &writeset_payload,
            Some(aptos_types::account_config::reserved_vm_address()),
            SessionId::genesis(genesis_id),
        )?;

        Self::validate_waypoint_change_set(&change_set, log_context)?;
        self.read_writeset(resolver, change_set.write_set_iter())?;
        assert!(
            change_set.aggregator_write_set().is_empty(),
            "Waypoint change set should not have any aggregator writes."
        );

        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = VMOutput::new(change_set, FeeStatement::zero(), VMStatus::Executed.into());
        Ok((VMStatus::Executed, output))
    }

    pub(crate) fn process_block_prologue(
        &self,
        resolver: &impl MoveResolverExt,
        block_metadata: BlockMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        fail_point!("move_adapter::process_block_prologue", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let txn_data = TransactionMetadata {
            sender: account_config::reserved_vm_address(),
            max_gas_amount: 0.into(),
            ..Default::default()
        };
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self
            .0
            .new_session(resolver, SessionId::block_meta(&block_metadata));

        let args = serialize_values(&block_metadata.get_prologue_move_args(txn_data.sender));
        session
            .execute_function_bypass_visibility(
                &BLOCK_MODULE,
                BLOCK_PROLOGUE,
                vec![],
                args,
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .or_else(|e| {
                expect_only_successful_execution(e, BLOCK_PROLOGUE.as_str(), log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = get_transaction_output(
            &mut (),
            session,
            FeeStatement::zero(),
            ExecutionStatus::Success,
            &self
                .0
                .get_storage_gas_parameters(log_context)?
                .change_set_configs,
        )?;
        Ok((VMStatus::Executed, output))
    }

    /// Executes a SignedTransaction without performing signature verification.
    pub fn simulate_signed_transaction(
        txn: &SignedTransaction,
        state_view: &impl StateView,
    ) -> (VMStatus, TransactionOutput) {
        let vm = AptosVM::new(state_view);
        let simulation_vm = AptosSimulationVM(vm);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let (vm_status, vm_output) = simulation_vm.simulate_signed_transaction(
            &simulation_vm.0.as_move_resolver(state_view),
            txn,
            &log_context,
        );
        (
            vm_status,
            vm_output
                .try_into_transaction_output(state_view)
                .expect("Simulation cannot fail"),
        )
    }

    pub fn execute_view_function(
        state_view: &impl StateView,
        module_id: ModuleId,
        func_name: Identifier,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
        gas_budget: u64,
    ) -> Result<Vec<Vec<u8>>> {
        let vm = AptosVM::new(state_view);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let mut gas_meter =
            MemoryTrackedGasMeter::new(StandardGasMeter::new(StandardGasAlgebra::new(
                vm.0.get_gas_feature_version(),
                vm.0.get_gas_parameters(&log_context)?.vm.clone(),
                vm.0.get_storage_gas_parameters(&log_context)?.clone(),
                gas_budget,
            )));
        let resolver = vm.as_move_resolver(state_view);
        let mut session = vm.new_session(&resolver, SessionId::Void);

        let func_inst = session.load_function(&module_id, &func_name, &type_args)?;
        let metadata = vm.0.extract_module_metadata(&module_id);
        let arguments = verifier::view_function::validate_view_function(
            &mut session,
            arguments,
            func_name.as_ident_str(),
            &func_inst,
            metadata.as_ref(),
            vm.0.get_features()
                .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
        )?;

        Ok(session
            .execute_function_bypass_visibility(
                &module_id,
                func_name.as_ident_str(),
                type_args,
                arguments,
                &mut gas_meter,
            )
            .map_err(|err| anyhow!("Failed to execute function: {:?}", err))?
            .return_values
            .into_iter()
            .map(|(bytes, _ty)| bytes)
            .collect::<Vec<_>>())
    }

    fn run_prologue_with_payload(
        &self,
        session: &mut SessionExt,
        resolver: &impl MoveResolverExt,
        payload: &TransactionPayload,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        // Whether the prologue is run as part of tx simulation.
        is_simulation: bool,
    ) -> Result<(), VMStatus> {
        match payload {
            TransactionPayload::Script(_) => {
                self.0.check_gas(resolver, txn_data, log_context)?;
                self.0.run_script_prologue(session, txn_data, log_context)
            },
            TransactionPayload::EntryFunction(_) => {
                // NOTE: Script and EntryFunction shares the same prologue
                self.0.check_gas(resolver, txn_data, log_context)?;
                self.0.run_script_prologue(session, txn_data, log_context)
            },
            TransactionPayload::Multisig(multisig_payload) => {
                self.0.check_gas(resolver, txn_data, log_context)?;
                // Still run script prologue for multisig transaction to ensure the same tx
                // validations are still run for this multisig execution tx, which is submitted by
                // one of the owners.
                self.0.run_script_prologue(session, txn_data, log_context)?;
                // Skip validation if this is part of tx simulation.
                // This allows simulating multisig txs without having to first create the multisig
                // tx.
                if !is_simulation {
                    self.0
                        .run_multisig_prologue(session, txn_data, multisig_payload, log_context)
                } else {
                    Ok(())
                }
            },

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundle(_module) => {
                if MODULE_BUNDLE_DISALLOWED.load(Ordering::Relaxed) {
                    return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
                }
                self.0.check_gas(resolver, txn_data, log_context)?;
                self.0.run_module_prologue(session, txn_data, log_context)
            },
        }
    }
}

// Executor external API
impl VMExecutor for AptosVM {
    /// Execute a block of `transactions`. The output vector will have the exact same length as the
    /// input vector. The discarded transactions will be marked as `TransactionStatus::Discard` and
    /// have an empty `WriteSet`. Also `state_view` is immutable, and does not have interior
    /// mutability. Writes to be applied to the data view are encoded in the write set part of a
    /// transaction output.
    fn execute_block(
        transactions: Vec<Transaction>,
        state_view: &(impl StateView + Sync),
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        fail_point!("move_adapter::execute_block", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        info!(
            log_context,
            "Executing block, transaction count: {}",
            transactions.len()
        );

        let count = transactions.len();
        let ret = BlockAptosVM::execute_block::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
        >(
            Arc::clone(&RAYON_EXEC_POOL),
            transactions,
            state_view,
            Self::get_concurrency_level(),
            maybe_block_gas_limit,
            None,
        );
        if ret.is_ok() {
            // Record the histogram count for transactions per block.
            BLOCK_TRANSACTION_COUNT.observe(count as f64);
        }
        ret
    }

    fn execute_block_sharded<S: StateView + Sync + Send + 'static, C: ExecutorClient<S>>(
        sharded_block_executor: &ShardedBlockExecutor<S, C>,
        transactions: PartitionedTransactions,
        state_view: Arc<S>,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        info!(
            log_context,
            "Executing block, transaction count: {}",
            transactions.num_txns()
        );

        let count = transactions.num_txns();
        let ret = sharded_block_executor.execute_block(
            state_view,
            transactions,
            AptosVM::get_concurrency_level(),
            maybe_block_gas_limit,
        );
        if ret.is_ok() {
            // Record the histogram count for transactions per block.
            BLOCK_TRANSACTION_COUNT.observe(count as f64);
        }
        ret
    }
}

// VMValidator external API
impl VMValidator for AptosVM {
    /// Determine if a transaction is valid. Will return `None` if the transaction is accepted,
    /// `Some(Err)` if the VM rejects it, with `Err` as an error code. Verification performs the
    /// following steps:
    /// 1. The signature on the `SignedTransaction` matches the public key included in the
    ///    transaction
    /// 2. The script to be executed is under given specific configuration.
    /// 3. Invokes `Account.prologue`, which checks properties such as the transaction has the
    /// right sequence number and the sender has enough balance to pay for the gas.
    /// TBD:
    /// 1. Transaction arguments matches the main function's type signature.
    ///    We don't check this item for now and would execute the check at execution time.
    fn validate_transaction(
        &self,
        transaction: SignedTransaction,
        state_view: &impl StateView,
    ) -> VMValidatorResult {
        let _timer = TXN_VALIDATION_SECONDS.start_timer();
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let txn = match Self::check_signature(transaction) {
            Ok(t) => t,
            _ => {
                return VMValidatorResult::error(StatusCode::INVALID_SIGNATURE);
            },
        };

        let resolver = self.as_move_resolver(state_view);
        let mut session = self.0.new_session(&resolver, SessionId::prologue(&txn));
        let validation_result = self.validate_signature_checked_transaction(
            &mut session,
            &resolver,
            &txn,
            true,
            &log_context,
        );

        // Increment the counter for transactions verified.
        let (counter_label, result) = match validation_result {
            Ok(_) => (
                "success",
                VMValidatorResult::new(None, txn.gas_unit_price()),
            ),
            Err(err) => (
                "failure",
                VMValidatorResult::new(Some(err.status_code()), 0),
            ),
        };

        TRANSACTIONS_VALIDATED
            .with_label_values(&[counter_label])
            .inc();

        result
    }
}

impl VMAdapter for AptosVM {
    fn new_session<'r>(
        &self,
        resolver: &'r impl MoveResolverExt,
        session_id: SessionId,
    ) -> SessionExt<'r, '_> {
        self.0.new_session(resolver, session_id)
    }

    fn check_signature(txn: SignedTransaction) -> Result<SignatureCheckedTransaction> {
        txn.check_signature()
    }

    fn check_transaction_format(&self, txn: &SignedTransaction) -> Result<(), VMStatus> {
        if txn.contains_duplicate_signers() {
            return Err(VMStatus::error(
                StatusCode::SIGNERS_CONTAIN_DUPLICATES,
                None,
            ));
        }

        Ok(())
    }

    fn run_prologue(
        &self,
        session: &mut SessionExt,
        resolver: &impl MoveResolverExt,
        transaction: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let txn_data = TransactionMetadata::new(transaction);
        self.run_prologue_with_payload(
            session,
            resolver,
            transaction.payload(),
            &txn_data,
            log_context,
            false,
        )
    }

    fn should_restart_execution(vm_output: &VMOutput) -> bool {
        let new_epoch_event_key = aptos_types::on_chain_config::new_epoch_event_key();
        vm_output
            .change_set()
            .events()
            .iter()
            .any(|event| *event.key() == new_epoch_event_key)
    }

    fn execute_single_transaction(
        &self,
        txn: &PreprocessedTransaction,
        resolver: &impl MoveResolverExt,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput, Option<String>), VMStatus> {
        Ok(match txn {
            PreprocessedTransaction::BlockMetadata(block_metadata) => {
                fail_point!("aptos_vm::execution::block_metadata");
                let (vm_status, output) =
                    self.process_block_prologue(resolver, block_metadata.clone(), log_context)?;
                (vm_status, output, Some("block_prologue".to_string()))
            },
            PreprocessedTransaction::WaypointWriteSet(write_set_payload) => {
                let (vm_status, output) = self.process_waypoint_change_set(
                    resolver,
                    write_set_payload.clone(),
                    log_context,
                )?;
                (vm_status, output, Some("waypoint_write_set".to_string()))
            },
            PreprocessedTransaction::UserTransaction(txn) => {
                fail_point!("aptos_vm::execution::user_transaction");
                let sender = txn.sender().to_string();
                let _timer = TXN_TOTAL_SECONDS.start_timer();
                let (vm_status, output) = self.execute_user_transaction(resolver, txn, log_context);

                if let StatusType::InvariantViolation = vm_status.status_type() {
                    match vm_status.status_code() {
                        // Type resolution failure can be triggered by user input when providing a bad type argument, skip this case.
                        StatusCode::TYPE_RESOLUTION_FAILURE
                            if vm_status.sub_status()
                                == Some(move_core_types::vm_status::sub_status::type_resolution_failure::EUSER_TYPE_LOADING_FAILURE) => {},
                        // The known Move function failure and type resolution failure could be a result of speculative execution. Use speculative logger.
                        StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION
                        | StatusCode::TYPE_RESOLUTION_FAILURE => {
                            speculative_error!(
                                log_context,
                                format!(
                                    "[aptos_vm] Transaction breaking invariant violation. txn: {:?}, status: {:?}",
                                    bcs::to_bytes::<SignedTransaction>(&**txn),
                                    vm_status
                                ),
                            );
                        },
                        // Paranoid mode failure. We need to be alerted about this ASAP.
                        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
                            if vm_status.sub_status()
                                == Some(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE) =>
                        {
                            error!(
                                *log_context,
                                "[aptos_vm] Transaction breaking paranoid mode. txn: {:?}, status: {:?}",
                                bcs::to_bytes::<SignedTransaction>(&**txn),
                                vm_status,
                            );
                        },
                        // Paranoid mode failure but with reference counting
                        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
                            if vm_status.sub_status()
                                == Some(move_core_types::vm_status::sub_status::unknown_invariant_violation::EREFERENCE_COUNTING_FAILURE) =>
                        {
                            error!(
                                *log_context,
                                "[aptos_vm] Transaction breaking paranoid mode. txn: {:?}, status: {:?}",
                                bcs::to_bytes::<SignedTransaction>(&**txn),
                                vm_status,
                            );
                        },
                        // Ignore Storage Error as it can be intentionally triggered by parallel execution.
                        StatusCode::STORAGE_ERROR => (),
                        // We will log the rest of invariant violation directly with regular logger as they shouldn't happen.
                        //
                        // TODO: Add different counters for the error categories here.
                        _ => {
                            error!(
                                *log_context,
                                "[aptos_vm] Transaction breaking invariant violation. txn: {:?}, status: {:?}",
                                bcs::to_bytes::<SignedTransaction>(&**txn),
                                vm_status,
                            );
                        },
                    }
                }

                // Increment the counter for user transactions executed.
                let counter_label = match output.status() {
                    TransactionStatus::Keep(_) => Some("success"),
                    TransactionStatus::Discard(_) => Some("discarded"),
                    TransactionStatus::Retry => None,
                };
                if let Some(label) = counter_label {
                    USER_TRANSACTIONS_EXECUTED.with_label_values(&[label]).inc();
                }
                (vm_status, output, Some(sender))
            },
            PreprocessedTransaction::InvalidSignature => {
                let (vm_status, output) =
                    discard_error_vm_status(VMStatus::error(StatusCode::INVALID_SIGNATURE, None));
                (vm_status, output, None)
            },
            PreprocessedTransaction::StateCheckpoint => {
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                let output = VMOutput::empty_with_status(status);
                (VMStatus::Executed, output, Some("state_checkpoint".into()))
            },
        })
    }
}

impl AsRef<AptosVMImpl> for AptosVM {
    fn as_ref(&self) -> &AptosVMImpl {
        &self.0
    }
}

impl AsMut<AptosVMImpl> for AptosVM {
    fn as_mut(&mut self) -> &mut AptosVMImpl {
        &mut self.0
    }
}

impl AptosSimulationVM {
    fn validate_simulated_transaction(
        &self,
        session: &mut SessionExt,
        resolver: &impl MoveResolverExt,
        transaction: &SignedTransaction,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        self.0.check_transaction_format(transaction)?;
        self.0.run_prologue_with_payload(
            session,
            resolver,
            transaction.payload(),
            txn_data,
            log_context,
            true,
        )
    }

    fn simulate_signed_transaction(
        &self,
        resolver: &impl MoveResolverExt,
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, VMOutput) {
        // simulation transactions should not carry valid signatures, otherwise malicious fullnodes
        // may execute them without user's explicit permission.
        if txn.signature_is_valid() {
            return discard_error_vm_status(VMStatus::error(StatusCode::INVALID_SIGNATURE, None));
        }

        // Revalidate the transaction.
        let txn_data = TransactionMetadata::new(txn);
        let mut session = self.0.new_session(resolver, SessionId::txn_meta(&txn_data));
        if let Err(err) =
            self.validate_simulated_transaction(&mut session, resolver, txn, &txn_data, log_context)
        {
            return discard_error_vm_status(err);
        };

        let gas_params = match self.0 .0.get_gas_parameters(log_context) {
            Err(err) => return discard_error_vm_status(err),
            Ok(s) => s,
        };
        let storage_gas_params = match self.0 .0.get_storage_gas_parameters(log_context) {
            Err(err) => return discard_error_vm_status(err),
            Ok(s) => s,
        };

        let mut gas_meter =
            MemoryTrackedGasMeter::new(StandardGasMeter::new(StandardGasAlgebra::new(
                self.0 .0.get_gas_feature_version(),
                gas_params.vm.clone(),
                storage_gas_params.clone(),
                txn_data.max_gas_amount(),
            )));

        let mut new_published_modules_loaded = false;
        let result = match txn.payload() {
            payload @ TransactionPayload::Script(_)
            | payload @ TransactionPayload::EntryFunction(_) => {
                self.0.execute_script_or_entry_function(
                    resolver,
                    session,
                    &mut gas_meter,
                    &txn_data,
                    payload,
                    log_context,
                    &mut new_published_modules_loaded,
                    &storage_gas_params.change_set_configs,
                )
            },
            TransactionPayload::Multisig(multisig) => {
                if let Some(payload) = multisig.transaction_payload.clone() {
                    match payload {
                        MultisigTransactionPayload::EntryFunction(entry_function) => {
                            aptos_try!({
                                return_on_failure!(self.0.execute_multisig_entry_function(
                                    &mut session,
                                    &mut gas_meter,
                                    multisig.multisig_address,
                                    &entry_function,
                                    &mut new_published_modules_loaded,
                                ));
                                // TODO: Deduplicate this against execute_multisig_transaction
                                // A bit tricky since we need to skip success/failure cleanups,
                                // which is in the middle. Introducing a boolean would make the code
                                // messier.
                                let respawned_session =
                                    self.0.charge_change_set_and_respawn_session(
                                        session,
                                        resolver,
                                        &mut gas_meter,
                                        &storage_gas_params.change_set_configs,
                                        &txn_data,
                                    )?;

                                self.0.success_transaction_cleanup(
                                    respawned_session,
                                    &mut gas_meter,
                                    &txn_data,
                                    log_context,
                                    &storage_gas_params.change_set_configs,
                                )
                            })
                        },
                    }
                } else {
                    Err(VMStatus::error(StatusCode::MISSING_DATA, None))
                }
            },

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundle(m) => self.0.execute_modules(
                resolver,
                session,
                &mut gas_meter,
                &txn_data,
                m,
                log_context,
                &mut new_published_modules_loaded,
                &storage_gas_params.change_set_configs,
            ),
        };

        match result {
            Ok(output) => output,
            Err(err) => {
                // Invalidate the loader cache in case there was a new module loaded from a module
                // publish request that failed.
                // This ensures the loader cache is flushed later to align storage with the cache.
                // None of the modules in the bundle will be committed to storage,
                // but some of them may have ended up in the cache.
                if new_published_modules_loaded {
                    self.0 .0.mark_loader_cache_as_invalid();
                };
                let txn_status = TransactionStatus::from_vm_status(
                    err.clone(),
                    self.0
                         .0
                        .get_features()
                        .is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION),
                );
                if txn_status.is_discarded() {
                    discard_error_vm_status(err)
                } else {
                    let (vm_status, output) = self.0.failed_transaction_cleanup_and_keep_vm_status(
                        err,
                        &mut gas_meter,
                        &txn_data,
                        resolver,
                        log_context,
                        &storage_gas_params.change_set_configs,
                    );
                    (vm_status, output)
                }
            },
        }
    }
}
