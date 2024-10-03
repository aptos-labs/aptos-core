// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_executor::{AptosTransactionOutput, BlockAptosVM},
    counters::*,
    data_cache::{AsMoveResolver, StorageAdapter},
    errors::{discarded_output, expect_only_successful_execution},
    gas::{check_gas, get_gas_parameters, make_prod_gas_meter, ProdGasMeter},
    keyless_validation,
    move_vm_ext::{
        session::user_transaction_sessions::{
            abort_hook::AbortHookSession,
            epilogue::EpilogueSession,
            prologue::PrologueSession,
            session_change_sets::{SystemSessionChangeSet, UserSessionChangeSet},
            user::UserSession,
        },
        AptosMoveResolver, MoveVmExt, SessionExt, SessionId, UserTransactionContext,
    },
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    system_module_names::*,
    transaction_metadata::TransactionMetadata,
    transaction_validation, verifier,
    verifier::randomness::get_randomness_annotation,
    VMExecutor, VMValidator,
};
use anyhow::anyhow;
use aptos_block_executor::txn_commit_hook::NoOpTransactionCommitHook;
use aptos_crypto::HashValue;
use aptos_framework::{
    natives::{code::PublishRequest, randomness::RandomnessContext},
    RuntimeModuleMetadataV1,
};
use aptos_gas_algebra::{Gas, GasQuantity, NumBytes, Octa};
use aptos_gas_meter::{AptosGasMeter, GasAlgebra};
use aptos_gas_schedule::{AptosGasParameters, VMGasParameters};
use aptos_logger::{enabled, prelude::*, Level};
use aptos_metrics_core::TimerHelper;
#[cfg(any(test, feature = "testing"))]
use aptos_types::state_store::StateViewId;
use aptos_types::{
    account_config::{self, new_block_event_key, AccountResource},
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        partitioner::PartitionedTransactions,
    },
    block_metadata::BlockMetadata,
    block_metadata_ext::{BlockMetadataExt, BlockMetadataWithRandomness},
    chain_id::ChainId,
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{
        new_epoch_event_key, ApprovedExecutionHashes, ConfigStorage, FeatureFlag, Features,
        OnChainConfig, TimedFeatureFlag, TimedFeatures,
    },
    randomness::Randomness,
    state_store::{state_key::StateKey, StateView, TStateView},
    transaction::{
        authenticator::AnySignature, signature_verified_transaction::SignatureVerifiedTransaction,
        BlockOutput, EntryFunction, ExecutionError, ExecutionStatus, ModuleBundle, Multisig,
        MultisigTransactionPayload, Script, SignedTransaction, Transaction, TransactionArgument,
        TransactionAuxiliaryData, TransactionOutput, TransactionPayload, TransactionStatus,
        VMValidatorResult, ViewFunctionOutput, WriteSetPayload,
    },
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use aptos_utils::aptos_try;
use aptos_vm_logging::{log_schema::AdapterLogSchema, speculative_error, speculative_log};
use aptos_vm_types::{
    abstract_write_op::AbstractResourceWriteOp,
    change_set::{
        create_vm_change_set_with_module_write_set_when_delayed_field_optimization_disabled,
        ChangeSetInterface, VMChangeSet,
    },
    environment::Environment,
    module_write_set::ModuleWriteSet,
    output::VMOutput,
    resolver::{ExecutorView, ResourceGroupView},
    storage::{change_set_configs::ChangeSetConfigs, StorageGasParameters},
};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use claims::assert_err;
use fail::fail_point;
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    deserializer::DeserializerConfig,
    errors::{Location, PartialVMError, PartialVMResult, VMError, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
    transaction_argument::convert_txn_args,
    value::{serialize_values, MoveTypeLayout, MoveValue},
    vm_status::StatusType,
};
use move_vm_runtime::{
    logging::expect_no_verification_errors,
    module_traversal::{TraversalContext, TraversalStorage},
};
use move_vm_types::gas::{GasMeter, UnmeteredGasMeter};
use num_cpus;
use once_cell::sync::OnceCell;
use std::{
    cmp::{max, min},
    collections::{BTreeMap, BTreeSet},
    marker::Sync,
    sync::Arc,
};

static EXECUTION_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static NUM_EXECUTION_SHARD: OnceCell<usize> = OnceCell::new();
static NUM_PROOF_READING_THREADS: OnceCell<usize> = OnceCell::new();
static DISCARD_FAILED_BLOCKS: OnceCell<bool> = OnceCell::new();
static PROCESSED_TRANSACTIONS_DETAILED_COUNTERS: OnceCell<bool> = OnceCell::new();

macro_rules! deprecated_module_bundle {
    () => {
        VMStatus::error(
            StatusCode::FEATURE_UNDER_GATING,
            Some("Module bundle payload has been removed".to_string()),
        )
    };
}

macro_rules! unwrap_or_discard {
    ($res:expr) => {
        match $res {
            Ok(s) => s,
            Err(e) => {
                // covers both VMStatus itself and VMError which can convert to VMStatus
                let s: VMStatus = e.into();

                let o = discarded_output(s.status_code());
                return (s, o);
            },
        }
    };
}

pub(crate) fn get_system_transaction_output(
    session: SessionExt,
    change_set_configs: &ChangeSetConfigs,
) -> Result<VMOutput, VMStatus> {
    let (change_set, empty_module_write_set) = session.finish(change_set_configs)?;

    // System transactions can never publish modules! When we move publishing outside MoveVM, we do not
    // need to have this check here, as modules will only be visible in user session.
    empty_module_write_set
        .is_empty_or_invariant_violation()
        .map_err(|e| {
            e.with_message(
                "Non-empty module write set in when creating system transaction output".to_string(),
            )
            .finish(Location::Undefined)
            .into_vm_status()
        })?;

    Ok(VMOutput::new(
        change_set,
        ModuleWriteSet::empty(),
        FeeStatement::zero(),
        TransactionStatus::Keep(ExecutionStatus::Success),
        TransactionAuxiliaryData::default(),
    ))
}

pub(crate) fn get_or_vm_startup_failure<'a, T>(
    gas_params: &'a Result<T, String>,
    log_context: &AdapterLogSchema,
) -> Result<&'a T, VMStatus> {
    gas_params.as_ref().map_err(|err| {
        let msg = format!("VM Startup Failed. {}", err);
        speculative_error!(log_context, msg.clone());
        VMStatus::error(StatusCode::VM_STARTUP_FAILURE, Some(msg))
    })
}

/// Checks if a given transaction is a governance proposal by checking if it has one of the
/// approved execution hashes.
fn is_approved_gov_script(
    resolver: &impl ConfigStorage,
    txn: &SignedTransaction,
    txn_metadata: &TransactionMetadata,
) -> bool {
    match txn.payload() {
        TransactionPayload::Script(_script) => {
            match ApprovedExecutionHashes::fetch_config(resolver) {
                Some(approved_execution_hashes) => approved_execution_hashes
                    .entries
                    .iter()
                    .any(|(_, hash)| hash == &txn_metadata.script_hash),
                None => false,
            }
        },
        _ => false,
    }
}

pub struct AptosVM {
    is_simulation: bool,
    move_vm: MoveVmExt,
    pub(crate) gas_feature_version: u64,
    gas_params: Result<AptosGasParameters, String>,
    pub(crate) storage_gas_params: Result<StorageGasParameters, String>,
    /// For a new chain, or even mainnet, the VK might not necessarily be set.
    pvk: Option<PreparedVerifyingKey<Bn254>>,
}

impl AptosVM {
    /// Creates a new VM instance, initializing the runtime environment from the state.
    pub fn new(state_view: &impl StateView) -> Self {
        let env = Arc::new(Environment::new(state_view));
        Self::new_with_environment(env, state_view, false)
    }

    pub fn new_for_gov_sim(state_view: &impl StateView) -> Self {
        let env = Arc::new(Environment::new(state_view));
        Self::new_with_environment(env, state_view, true)
    }

    /// Creates a new VM instance based on the runtime environment, and used by block
    /// executor to create multiple tasks sharing the same execution configurations.
    // TODO: Passing `state_view` is not needed once we move keyless and gas-related
    //       configs to the environment.
    pub(crate) fn new_with_environment(
        env: Arc<Environment>,
        state_view: &impl StateView,
        inject_create_signer_for_gov_sim: bool,
    ) -> Self {
        let _timer = TIMER.timer_with(&["AptosVM::new"]);

        let (gas_params, storage_gas_params, gas_feature_version) =
            get_gas_parameters(env.features(), state_view);

        let resolver = state_view.as_move_resolver();
        let move_vm = MoveVmExt::new_with_extended_options(
            gas_feature_version,
            gas_params.as_ref(),
            env,
            None,
            inject_create_signer_for_gov_sim,
            &resolver,
        );

        // We use an `Option` to handle the VK not being set on-chain, or an incorrect VK being set
        // via governance (although, currently, we do check for that in `keyless_account.move`).
        let pvk = keyless_validation::get_groth16_vk_onchain(&resolver)
            .ok()
            .and_then(|vk| vk.try_into().ok());

        Self {
            is_simulation: false,
            move_vm,
            gas_feature_version,
            gas_params,
            storage_gas_params,
            pvk,
        }
    }

    pub fn new_session<'r, S: AptosMoveResolver>(
        &self,
        resolver: &'r S,
        session_id: SessionId,
        user_transaction_context_opt: Option<UserTransactionContext>,
    ) -> SessionExt<'r, '_> {
        self.move_vm
            .new_session(resolver, session_id, user_transaction_context_opt)
    }

    #[inline(always)]
    fn features(&self) -> &Features {
        self.move_vm.env.features()
    }

    #[inline(always)]
    fn timed_features(&self) -> &TimedFeatures {
        self.move_vm.env.timed_features()
    }

    #[inline(always)]
    fn deserializer_config(&self) -> &DeserializerConfig {
        &self.move_vm.env.vm_config().deserializer_config
    }

    #[inline(always)]
    fn chain_id(&self) -> ChainId {
        self.move_vm.env.chain_id()
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
    pub fn set_discard_failed_blocks(enable: bool) {
        // Only the first call succeeds, due to OnceCell semantics.
        DISCARD_FAILED_BLOCKS.set(enable).ok();
    }

    /// Get the discard failed blocks flag if already set, otherwise return default (false)
    pub fn get_discard_failed_blocks() -> bool {
        match DISCARD_FAILED_BLOCKS.get() {
            Some(enable) => *enable,
            None => false,
        }
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

    /// Sets additional details in counters when invoked the first time.
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

    /// Returns the internal gas schedule if it has been loaded, or an error if it hasn't.
    #[cfg(any(test, feature = "testing"))]
    pub fn gas_params(&self) -> Result<&AptosGasParameters, VMStatus> {
        let log_context = AdapterLogSchema::new(StateViewId::Miscellaneous, 0);
        get_or_vm_startup_failure(&self.gas_params, &log_context)
    }

    pub fn as_move_resolver<'r, R: ExecutorView>(
        &self,
        executor_view: &'r R,
    ) -> StorageAdapter<'r, R> {
        StorageAdapter::new_with_config(
            executor_view,
            self.gas_feature_version,
            self.features(),
            None,
        )
    }

    pub fn as_move_resolver_with_group_view<'r, R: ExecutorView + ResourceGroupView>(
        &self,
        executor_view: &'r R,
    ) -> StorageAdapter<'r, R> {
        StorageAdapter::new_with_config(
            executor_view,
            self.gas_feature_version,
            self.features(),
            Some(executor_view),
        )
    }

    fn fee_statement_from_gas_meter(
        txn_data: &TransactionMetadata,
        gas_meter: &impl AptosGasMeter,
        storage_fee_refund: u64,
    ) -> FeeStatement {
        let gas_used = Self::gas_used(txn_data.max_gas_amount(), gas_meter);
        FeeStatement::new(
            gas_used,
            u64::from(gas_meter.execution_gas_used()),
            u64::from(gas_meter.io_gas_used()),
            u64::from(gas_meter.storage_fee_used()),
            storage_fee_refund,
        )
    }

    pub(crate) fn failed_transaction_cleanup(
        &self,
        prologue_session_change_set: SystemSessionChangeSet,
        error_vm_status: VMStatus,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
    ) -> (VMStatus, VMOutput) {
        if self.gas_feature_version >= 12 {
            // Check if the gas meter's internal counters are consistent.
            //
            // Since we are already in the failure epilogue, there is not much we can do
            // other than logging the inconsistency.
            //
            // This is a tradeoff. We have to either
            //   1. Continue to calculate the gas cost based on the numbers we have.
            //   2. Discard the transaction.
            //
            // Option (2) does not work, since it would enable DoS attacks.
            // Option (1) is not ideal, but optimistically, it should allow the network
            // to continue functioning, less the transactions that run into this problem.
            if let Err(err) = gas_meter.algebra().check_consistency() {
                println!(
                    "[aptos-vm][gas-meter][failure-epilogue] {}",
                    err.message()
                        .unwrap_or("No message found -- this should not happen.")
                );
            }
        }

        let (txn_status, txn_aux_data) = TransactionStatus::from_vm_status(
            error_vm_status.clone(),
            self.features()
                .is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION),
            self.features(),
        );

        match txn_status {
            TransactionStatus::Keep(status) => {
                // The transaction should be kept. Run the appropriate post transaction workflows
                // including epilogue. This runs a new session that ignores any side effects that
                // might abort the execution (e.g., spending additional funds needed to pay for
                // gas). Even if the previous failure occurred while running the epilogue, it
                // should not fail now. If it somehow fails here, there is no choice but to
                // discard the transaction.
                let output = self
                    .finish_aborted_transaction(
                        prologue_session_change_set,
                        gas_meter,
                        txn_data,
                        resolver,
                        status,
                        txn_aux_data,
                        log_context,
                        change_set_configs,
                        traversal_context,
                    )
                    .unwrap_or_else(|status| discarded_output(status.status_code()));
                (error_vm_status, output)
            },
            TransactionStatus::Discard(status_code) => {
                let discarded_output = discarded_output(status_code);
                (error_vm_status, discarded_output)
            },
            TransactionStatus::Retry => unreachable!(),
        }
    }

    fn inject_abort_info_if_available(&self, status: ExecutionStatus) -> ExecutionStatus {
        match status {
            ExecutionStatus::MoveAbort {
                location: AbortLocation::Module(module),
                code,
                ..
            } => {
                let info = self
                    .extract_module_metadata(&module)
                    .and_then(|m| m.extract_abort_info(code));
                ExecutionStatus::MoveAbort {
                    location: AbortLocation::Module(module),
                    code,
                    info,
                }
            },
            _ => status,
        }
    }

    fn finish_aborted_transaction(
        &self,
        prologue_session_change_set: SystemSessionChangeSet,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl AptosMoveResolver,
        status: ExecutionStatus,
        txn_aux_data: TransactionAuxiliaryData,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
    ) -> Result<VMOutput, VMStatus> {
        // Storage refund is zero since no slots are deleted in aborted transactions.
        const ZERO_STORAGE_REFUND: u64 = 0;

        let is_account_init_for_sponsored_transaction =
            is_account_init_for_sponsored_transaction(txn_data, self.features(), resolver)?;

        let (previous_session_change_set, fee_statement) =
            if is_account_init_for_sponsored_transaction {
                let mut abort_hook_session =
                    AbortHookSession::new(self, txn_data, resolver, prologue_session_change_set);

                abort_hook_session.execute(|session| {
                    create_account_if_does_not_exist(
                        session,
                        gas_meter,
                        txn_data.sender(),
                        traversal_context,
                    )
                    // If this fails, it is likely due to out of gas, so we try again without metering
                    // and then validate below that we charged sufficiently.
                    .or_else(|_err| {
                        create_account_if_does_not_exist(
                            session,
                            &mut UnmeteredGasMeter,
                            txn_data.sender(),
                            traversal_context,
                        )
                    })
                    .map_err(expect_no_verification_errors)
                    .or_else(|err| {
                        expect_only_successful_execution(
                            err,
                            &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
                            log_context,
                        )
                    })
                })?;

                let mut abort_hook_session_change_set =
                    abort_hook_session.finish(change_set_configs)?;
                if let Err(err) = self.charge_change_set(
                    &mut abort_hook_session_change_set,
                    gas_meter,
                    txn_data,
                    resolver,
                ) {
                    info!(
                        *log_context,
                        "Failed during charge_change_set: {:?}. Most likely exceeded gas limited.",
                        err,
                    );
                };

                let fee_statement =
                    AptosVM::fee_statement_from_gas_meter(txn_data, gas_meter, ZERO_STORAGE_REFUND);

                // Verify we charged sufficiently for creating an account slot
                let gas_params = get_or_vm_startup_failure(&self.gas_params, log_context)?;
                let gas_unit_price = u64::from(txn_data.gas_unit_price());
                let gas_used = fee_statement.gas_used();
                let storage_fee = fee_statement.storage_fee_used();
                let storage_refund = fee_statement.storage_fee_refund();

                let actual = gas_used * gas_unit_price + storage_fee - storage_refund;
                let expected = u64::from(
                    gas_meter
                        .disk_space_pricing()
                        .hack_account_creation_fee_lower_bound(&gas_params.vm.txn),
                );
                if actual < expected {
                    expect_only_successful_execution(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message(
                                "Insufficient fee for storing account for sponsored transaction"
                                    .to_string(),
                            )
                            .finish(Location::Undefined),
                        &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
                        log_context,
                    )?;
                }
                (abort_hook_session_change_set, fee_statement)
            } else {
                let fee_statement =
                    AptosVM::fee_statement_from_gas_meter(txn_data, gas_meter, ZERO_STORAGE_REFUND);
                (prologue_session_change_set, fee_statement)
            };

        let mut epilogue_session = EpilogueSession::on_user_session_failure(
            self,
            txn_data,
            resolver,
            previous_session_change_set,
        );

        // Abort information is injected using the user defined error in the Move contract.
        //
        // DO NOT move abort info injection before we create an epilogue session, because if
        // there is a code publishing transaction that fails, it will invalidate VM loader
        // cache which is flushed ONLY WHEN THE NEXT SESSION IS CREATED!
        // Also, do not move this after we run failure epilogue below, because this will load
        // module, which alters abort info. We have a transaction at version 596888095 which
        // relies on this specific behavior...
        let status = self.inject_abort_info_if_available(status);

        epilogue_session.execute(|session| {
            transaction_validation::run_failure_epilogue(
                session,
                gas_meter.balance(),
                fee_statement,
                self.features(),
                txn_data,
                log_context,
                traversal_context,
                self.is_simulation,
            )
        })?;

        epilogue_session.finish(fee_statement, status, txn_aux_data, change_set_configs)
    }

    fn success_transaction_cleanup(
        &self,
        mut epilogue_session: EpilogueSession,
        gas_meter: &impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
        has_modules_published_to_special_address: bool,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        if self.gas_feature_version >= 12 {
            // Check if the gas meter's internal counters are consistent.
            //
            // It's better to fail the transaction due to invariant violation than to allow
            // potentially bogus states to be committed.
            if let Err(err) = gas_meter.algebra().check_consistency() {
                println!(
                    "[aptos-vm][gas-meter][success-epilogue] {}",
                    err.message()
                        .unwrap_or("No message found -- this should not happen.")
                );
                return Err(err.finish(Location::Undefined).into());
            }
        }

        let fee_statement = AptosVM::fee_statement_from_gas_meter(
            txn_data,
            gas_meter,
            u64::from(epilogue_session.get_storage_fee_refund()),
        );
        epilogue_session.execute(|session| {
            transaction_validation::run_success_epilogue(
                session,
                gas_meter.balance(),
                fee_statement,
                self.features(),
                txn_data,
                log_context,
                traversal_context,
                self.is_simulation,
            )
        })?;
        let output = epilogue_session.finish(
            fee_statement,
            ExecutionStatus::Success,
            TransactionAuxiliaryData::default(),
            change_set_configs,
        )?;

        // We mark module cache invalid if transaction is successfully executed and has
        // published modules. The reason is that epilogue loads the old version of code,
        // and so we need to make sure the next transaction sees the new code.
        // Note that we only do so for modules at special addresses - i.e., those that
        // could have actually been loaded in the epilogue.
        if has_modules_published_to_special_address {
            self.move_vm.mark_loader_cache_as_invalid();
        }

        Ok((VMStatus::Executed, output))
    }

    fn validate_and_execute_script(
        &self,
        session: &mut SessionExt,
        // Note: cannot use AptosGasMeter because it is not implemented for
        //       UnmeteredGasMeter.
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        senders: Vec<AccountAddress>,
        script: &Script,
    ) -> Result<(), VMStatus> {
        if !self
            .features()
            .is_enabled(FeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS)
        {
            for arg in script.args() {
                if let TransactionArgument::Serialized(_) = arg {
                    return Err(PartialVMError::new(StatusCode::FEATURE_UNDER_GATING)
                        .finish(Location::Script)
                        .into_vm_status());
                }
            }
        }

        // Note: Feature gating is needed here because the traversal of the dependencies could
        //       result in shallow-loading of the modules and therefore subtle changes in
        //       the error semantics.
        if self.gas_feature_version >= 15 {
            session.check_script_dependencies_and_check_gas(
                gas_meter,
                traversal_context,
                script.code(),
            )?;
        }

        let func = session.load_script(script.code(), script.ty_args())?;

        let compiled_script = match CompiledScript::deserialize_with_config(
            script.code(),
            self.deserializer_config(),
        ) {
            Ok(script) => script,
            Err(err) => {
                let msg = format!("[VM] deserializer for script returned error: {:?}", err);
                let partial_err = PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Script);
                return Err(partial_err.into_vm_status());
            },
        };

        // Check that unstable bytecode cannot be executed on mainnet
        if self
            .features()
            .is_enabled(FeatureFlag::REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT)
        {
            self.reject_unstable_bytecode_for_script(&compiled_script)?;
        }

        // TODO(Gerardo): consolidate the extended validation to verifier.
        verifier::event_validation::verify_no_event_emission_in_compiled_script(&compiled_script)?;

        let args = verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
            session,
            senders,
            convert_txn_args(script.args()),
            &func,
            self.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
        )?;

        session.execute_script(
            script.code(),
            script.ty_args().to_vec(),
            args,
            gas_meter,
            traversal_context,
        )?;
        Ok(())
    }

    fn validate_and_execute_entry_function(
        &self,
        resolver: &impl AptosMoveResolver,
        session: &mut SessionExt,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        senders: Vec<AccountAddress>,
        entry_fn: &EntryFunction,
        _txn_data: &TransactionMetadata,
    ) -> Result<(), VMStatus> {
        // Note: Feature gating is needed here because the traversal of the dependencies could
        //       result in shallow-loading of the modules and therefore subtle changes in
        //       the error semantics.
        if self.gas_feature_version >= 15 {
            let module_id = traversal_context
                .referenced_module_ids
                .alloc(entry_fn.module().clone());
            session.check_dependencies_and_charge_gas(gas_meter, traversal_context, [(
                module_id.address(),
                module_id.name(),
            )])?;
        }

        let function =
            session.load_function(entry_fn.module(), entry_fn.function(), entry_fn.ty_args())?;

        // Native entry function is forbidden.
        if self
            .features()
            .is_enabled(FeatureFlag::DISALLOW_USER_NATIVES)
            && function.is_native()
        {
            return Err(
                PartialVMError::new(StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED)
                    .with_message(
                        "Executing user defined native entry function is not allowed".to_string(),
                    )
                    .finish(Location::Module(entry_fn.module().clone()))
                    .into_vm_status(),
            );
        }

        // The `has_randomness_attribute()` should have been feature-gated in 1.11...
        if function.is_friend_or_private()
            && get_randomness_annotation(resolver, session, entry_fn)?.is_some()
        {
            let txn_context = session
                .get_native_extensions()
                .get_mut::<RandomnessContext>();
            txn_context.mark_unbiasable();
        }

        let struct_constructors_enabled =
            self.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS);
        let args = verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
            session,
            senders,
            entry_fn.args().to_vec(),
            &function,
            struct_constructors_enabled,
        )?;
        session.execute_entry_function(function, args, gas_meter, traversal_context)?;
        Ok(())
    }

    fn execute_script_or_entry_function<'a, 'r, 'l>(
        &'l self,
        resolver: &'r impl AptosMoveResolver,
        mut session: UserSession<'r, 'l>,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext<'a>,
        txn_data: &TransactionMetadata,
        payload: &'a TransactionPayload,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        fail_point!("aptos_vm::execute_script_or_entry_function", |_| {
            Err(VMStatus::Error {
                status_code: StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                sub_status: Some(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
                message: None,
            })
        });

        gas_meter.charge_intrinsic_gas_for_transaction(txn_data.transaction_size())?;
        if txn_data.is_keyless() {
            gas_meter.charge_keyless()?;
        }

        match payload {
            TransactionPayload::Script(script) => {
                session.execute(|session| {
                    self.validate_and_execute_script(
                        session,
                        gas_meter,
                        traversal_context,
                        txn_data.senders(),
                        script,
                    )
                })?;
            },
            TransactionPayload::EntryFunction(entry_fn) => {
                session.execute(|session| {
                    self.validate_and_execute_entry_function(
                        resolver,
                        session,
                        gas_meter,
                        traversal_context,
                        txn_data.senders(),
                        entry_fn,
                        txn_data,
                    )
                })?;
            },

            // Not reachable as this function should only be invoked for entry or script
            // transaction payload.
            _ => unreachable!("Only scripts or entry functions are executed"),
        };

        let user_session_change_set = self.resolve_pending_code_publish_and_finish_user_session(
            session,
            resolver,
            gas_meter,
            traversal_context,
            new_published_modules_loaded,
            change_set_configs,
        )?;
        let has_modules_published_to_special_address =
            user_session_change_set.has_modules_published_to_special_address();

        let epilogue_session = self.charge_change_set_and_respawn_session(
            user_session_change_set,
            resolver,
            gas_meter,
            txn_data,
        )?;

        self.success_transaction_cleanup(
            epilogue_session,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
            traversal_context,
            has_modules_published_to_special_address,
        )
    }

    fn charge_change_set(
        &self,
        change_set: &mut impl ChangeSetInterface,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl AptosMoveResolver,
    ) -> Result<GasQuantity<Octa>, VMStatus> {
        gas_meter.charge_io_gas_for_transaction(txn_data.transaction_size())?;
        for event in change_set.events_iter() {
            gas_meter.charge_io_gas_for_event(event)?;
        }
        for (key, op_size) in change_set.write_set_size_iter() {
            gas_meter.charge_io_gas_for_write(key, &op_size)?;
        }

        let mut storage_refund = gas_meter.process_storage_fee_for_all(
            change_set,
            txn_data.transaction_size,
            txn_data.gas_unit_price,
            resolver.as_executor_view(),
        )?;
        if !self.features().is_storage_deletion_refund_enabled() {
            storage_refund = 0.into();
        }

        Ok(storage_refund)
    }

    fn charge_change_set_and_respawn_session<'r, 'l>(
        &'l self,
        mut user_session_change_set: UserSessionChangeSet,
        resolver: &'r impl AptosMoveResolver,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &'l TransactionMetadata,
    ) -> Result<EpilogueSession<'r, 'l>, VMStatus> {
        let storage_refund =
            self.charge_change_set(&mut user_session_change_set, gas_meter, txn_data, resolver)?;

        // TODO[agg_v1](fix): Charge for aggregator writes
        Ok(EpilogueSession::on_user_session_success(
            self,
            txn_data,
            resolver,
            user_session_change_set,
            storage_refund,
        ))
    }

    fn simulate_multisig_transaction<'a, 'r, 'l>(
        &'l self,
        resolver: &'r impl AptosMoveResolver,
        session: UserSession<'r, 'l>,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext<'a>,
        txn_data: &TransactionMetadata,
        payload: &'a Multisig,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match &payload.transaction_payload {
            None => Err(VMStatus::error(StatusCode::MISSING_DATA, None)),
            Some(multisig_payload) => {
                match multisig_payload {
                    MultisigTransactionPayload::EntryFunction(entry_function) => {
                        aptos_try!({
                            let user_session_change_set = self.execute_multisig_entry_function(
                                resolver,
                                session,
                                gas_meter,
                                traversal_context,
                                payload.multisig_address,
                                entry_function,
                                new_published_modules_loaded,
                                txn_data,
                                change_set_configs,
                            )?;
                            let has_modules_published_to_special_address =
                                user_session_change_set.has_modules_published_to_special_address();

                            // TODO: Deduplicate this against execute_multisig_transaction
                            // A bit tricky since we need to skip success/failure cleanups,
                            // which is in the middle. Introducing a boolean would make the code
                            // messier.
                            let epilogue_session = self.charge_change_set_and_respawn_session(
                                user_session_change_set,
                                resolver,
                                gas_meter,
                                txn_data,
                            )?;

                            self.success_transaction_cleanup(
                                epilogue_session,
                                gas_meter,
                                txn_data,
                                log_context,
                                change_set_configs,
                                traversal_context,
                                has_modules_published_to_special_address,
                            )
                        })
                    },
                }
            },
        }
    }

    // Execute a multisig transaction:
    // 1. Obtain the payload of the transaction to execute. This could have been stored on chain
    // when the multisig transaction was created.
    // 2. Execute the target payload. If this fails, discard the session and keep the gas meter and
    // failure object. In case of success, keep the session and also do any necessary module publish
    // cleanup.
    // 3. Call post transaction cleanup function in multisig account module with the result from (2)
    fn execute_multisig_transaction<'r, 'l>(
        &'l self,
        resolver: &'r impl AptosMoveResolver,
        mut session: UserSession<'r, 'l>,
        prologue_session_change_set: &SystemSessionChangeSet,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
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
        if txn_data.is_keyless() {
            gas_meter.charge_keyless()?;
        }

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
            if self
                .features()
                .is_abort_if_multisig_payload_mismatch_enabled()
            {
                vec![]
            } else {
                bcs::to_bytes::<Vec<u8>>(&vec![]).map_err(|_| invariant_violation_error())?
            }
        };
        // Failures here will be propagated back.
        let payload_bytes: Vec<Vec<u8>> = session
            .execute(|session| {
                session.execute_function_bypass_visibility(
                    &MULTISIG_ACCOUNT_MODULE,
                    GET_NEXT_TRANSACTION_PAYLOAD,
                    vec![],
                    serialize_values(&vec![
                        MoveValue::Address(txn_payload.multisig_address),
                        MoveValue::vector_u8(provided_payload),
                    ]),
                    gas_meter,
                    traversal_context,
                )
            })?
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
                    resolver,
                    session,
                    gas_meter,
                    traversal_context,
                    txn_payload.multisig_address,
                    &entry_function,
                    new_published_modules_loaded,
                    txn_data,
                    change_set_configs,
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

        let (epilogue_session, has_modules_published_to_special_address) = match execution_result {
            Err(execution_error) => {
                // Invalidate the loader cache in case there was a new module loaded from a module
                // publish request that failed.
                // This is redundant with the logic in execute_user_transaction but unfortunately is
                // necessary here as executing the underlying call can fail without this function
                // returning an error to execute_user_transaction.
                if *new_published_modules_loaded {
                    self.move_vm.mark_loader_cache_as_invalid();
                };
                let epilogue_session = self.failure_multisig_payload_cleanup(
                    resolver,
                    prologue_session_change_set,
                    execution_error,
                    txn_data,
                    cleanup_args,
                    traversal_context,
                )?;
                (epilogue_session, false)
            },
            Ok(user_session_change_set) => {
                let has_modules_published_to_special_address =
                    user_session_change_set.has_modules_published_to_special_address();

                // Charge gas for write set before we do cleanup. This ensures we don't charge gas for
                // cleanup write set changes, which is consistent with outer-level success cleanup
                // flow. We also wouldn't need to worry that we run out of gas when doing cleanup.
                let mut epilogue_session = self.charge_change_set_and_respawn_session(
                    user_session_change_set,
                    resolver,
                    gas_meter,
                    txn_data,
                )?;
                epilogue_session.execute(|session| {
                    session
                        .execute_function_bypass_visibility(
                            &MULTISIG_ACCOUNT_MODULE,
                            SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP,
                            vec![],
                            cleanup_args,
                            &mut UnmeteredGasMeter,
                            traversal_context,
                        )
                        .map_err(|e| e.into_vm_status())
                })?;
                (epilogue_session, has_modules_published_to_special_address)
            },
        };

        // TODO(Gas): Charge for aggregator writes
        self.success_transaction_cleanup(
            epilogue_session,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
            traversal_context,
            has_modules_published_to_special_address,
        )
    }

    fn execute_or_simulate_multisig_transaction<'a, 'r, 'l>(
        &'l self,
        resolver: &'r impl AptosMoveResolver,
        session: UserSession<'r, 'l>,
        prologue_session_change_set: &SystemSessionChangeSet,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext<'a>,
        txn_data: &TransactionMetadata,
        payload: &'a Multisig,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        // Once `simulation_enhancement` is enabled, we use `execute_multisig_transaction` for simulation,
        // deprecating `simulate_multisig_transaction`.
        if self.is_simulation
            && !self
                .features()
                .is_transaction_simulation_enhancement_enabled()
        {
            self.simulate_multisig_transaction(
                resolver,
                session,
                gas_meter,
                traversal_context,
                txn_data,
                payload,
                log_context,
                new_published_modules_loaded,
                change_set_configs,
            )
        } else {
            self.execute_multisig_transaction(
                resolver,
                session,
                prologue_session_change_set,
                gas_meter,
                traversal_context,
                txn_data,
                payload,
                log_context,
                new_published_modules_loaded,
                change_set_configs,
            )
        }
    }

    fn execute_multisig_entry_function(
        &self,
        resolver: &impl AptosMoveResolver,
        mut session: UserSession<'_, '_>,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        multisig_address: AccountAddress,
        payload: &EntryFunction,
        new_published_modules_loaded: &mut bool,
        txn_data: &TransactionMetadata,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        // If txn args are not valid, we'd still consider the transaction as executed but
        // failed. This is primarily because it's unrecoverable at this point.
        session.execute(|session| {
            self.validate_and_execute_entry_function(
                resolver,
                session,
                gas_meter,
                traversal_context,
                vec![multisig_address],
                payload,
                txn_data,
            )
        })?;

        // Resolve any pending module publishes in case the multisig transaction is deploying
        // modules.
        self.resolve_pending_code_publish_and_finish_user_session(
            session,
            resolver,
            gas_meter,
            traversal_context,
            new_published_modules_loaded,
            change_set_configs,
        )
    }

    fn failure_multisig_payload_cleanup<'r, 'l>(
        &'l self,
        resolver: &'r impl AptosMoveResolver,
        prologue_session_change_set: &SystemSessionChangeSet,
        execution_error: VMStatus,
        txn_data: &'l TransactionMetadata,
        mut cleanup_args: Vec<Vec<u8>>,
        traversal_context: &mut TraversalContext,
    ) -> Result<EpilogueSession<'r, 'l>, VMStatus> {
        // Start a fresh session for running cleanup that does not contain any changes from
        // the inner function call earlier (since it failed).
        let mut epilogue_session = EpilogueSession::on_user_session_failure(
            self,
            txn_data,
            resolver,
            prologue_session_change_set.clone(),
        );
        let execution_error = ExecutionError::try_from(execution_error)
            .map_err(|_| VMStatus::error(StatusCode::UNREACHABLE, None))?;
        // Serialization is not expected to fail so we're using invariant_violation error here.
        cleanup_args.push(bcs::to_bytes(&execution_error).map_err(|_| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("MultiSig payload cleanup error.".to_string())
                .finish(Location::Undefined)
        })?);
        epilogue_session.execute(|session| {
            session
                .execute_function_bypass_visibility(
                    &MULTISIG_ACCOUNT_MODULE,
                    FAILED_TRANSACTION_EXECUTION_CLEANUP,
                    vec![],
                    cleanup_args,
                    &mut UnmeteredGasMeter,
                    traversal_context,
                )
                .map_err(|e| e.into_vm_status())
        })?;
        Ok(epilogue_session)
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
        traversal_context: &mut TraversalContext,
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
                        traversal_context,
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
        let mut result = vec![];
        for module_blob in modules.iter() {
            match CompiledModule::deserialize_with_config(
                module_blob.code(),
                self.deserializer_config(),
            ) {
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

    /// Resolve a pending code publish request registered via the NativeCodeContext.
    fn resolve_pending_code_publish_and_finish_user_session(
        &self,
        mut session: UserSession<'_, '_>,
        resolver: &impl AptosMoveResolver,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        session.execute(|session| {
            if let Some(publish_request) = session.extract_publish_request() {
                let PublishRequest {
                    destination,
                    bundle,
                    expected_modules,
                    allowed_deps,
                    check_compat: _,
                } = publish_request;

                let modules = self.deserialize_module_bundle(&bundle)?;
                let modules: &Vec<CompiledModule> =
                    traversal_context.referenced_module_bundles.alloc(modules);

                // Note: Feature gating is needed here because the traversal of the dependencies could
                //       result in shallow-loading of the modules and therefore subtle changes in
                //       the error semantics.
                if self.gas_feature_version >= 15 {
                    // Charge old versions of existing modules, in case of upgrades.
                    for module in modules.iter() {
                        let addr = module.self_addr();
                        let name = module.self_name();
                        let state_key = StateKey::module(addr, name);

                        // TODO: Allow the check of special addresses to be customized.
                        if addr.is_special()
                            || traversal_context.visited.insert((addr, name), ()).is_some()
                        {
                            continue;
                        }

                        let size_if_module_exists = resolver
                            .as_executor_view()
                            .get_module_state_value_size(&state_key)
                            .map_err(|e| e.finish(Location::Undefined))?;

                        if let Some(size) = size_if_module_exists {
                            gas_meter
                                .charge_dependency(false, addr, name, NumBytes::new(size))
                                .map_err(|err| {
                                    err.finish(Location::Module(ModuleId::new(
                                        *addr,
                                        name.to_owned(),
                                    )))
                                })?;
                        }
                    }

                    // Charge all modules in the bundle that is about to be published.
                    for (module, blob) in modules.iter().zip(bundle.iter()) {
                        let module_id = &module.self_id();
                        gas_meter
                            .charge_dependency(
                                true,
                                module_id.address(),
                                module_id.name(),
                                NumBytes::new(blob.code().len() as u64),
                            )
                            .map_err(|err| err.finish(Location::Undefined))?;
                    }

                    // Charge all dependencies.
                    //
                    // Must exclude the ones that are in the current bundle because they have not
                    // been published yet.
                    let module_ids_in_bundle = modules
                        .iter()
                        .map(|module| (module.self_addr(), module.self_name()))
                        .collect::<BTreeSet<_>>();

                    session.check_dependencies_and_charge_gas(
                        gas_meter,
                        traversal_context,
                        modules
                            .iter()
                            .flat_map(|module| {
                                module
                                    .immediate_dependencies_iter()
                                    .chain(module.immediate_friends_iter())
                            })
                            .filter(|addr_and_name| !module_ids_in_bundle.contains(addr_and_name)),
                    )?;

                    // TODO: Revisit the order of traversal. Consider switching to alphabetical order.
                }

                if self
                    .timed_features()
                    .is_enabled(TimedFeatureFlag::ModuleComplexityCheck)
                {
                    for (module, blob) in modules.iter().zip(bundle.iter()) {
                        // TODO(Gas): Make budget configurable.
                        let budget = 2048 + blob.code().len() as u64 * 20;
                        move_binary_format::check_complexity::check_module_complexity(
                            module, budget,
                        )
                        .map_err(|err| err.finish(Location::Undefined))?;
                    }
                }

                // Validate the module bundle
                self.validate_publish_request(session, modules, expected_modules, allowed_deps)?;

                // Check what modules exist before publishing.
                let mut exists = BTreeSet::new();
                for m in modules {
                    let id = m.self_id();
                    if session.exists_module(&id)? {
                        exists.insert(id);
                    }
                }

                // Publish the bundle and execute initializers
                // publish_module_bundle doesn't actually load the published module into
                // the loader cache. It only puts the module data in the data cache.
                session.publish_module_bundle_with_compat_config(
                    bundle.into_inner(),
                    destination,
                    gas_meter,
                    Compatibility::new(
                        true,
                        !self
                            .features()
                            .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
                    ),
                )?;

                self.execute_module_initialization(
                    session,
                    gas_meter,
                    modules,
                    exists,
                    &[destination],
                    new_published_modules_loaded,
                    traversal_context,
                )?;
            }
            Ok::<(), VMError>(())
        })?;
        session.finish(change_set_configs)
    }

    /// Validate a publish request.
    fn validate_publish_request(
        &self,
        session: &mut SessionExt,
        modules: &[CompiledModule],
        mut expected_modules: BTreeSet<String>,
        allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
    ) -> VMResult<()> {
        if self
            .features()
            .is_enabled(FeatureFlag::REJECT_UNSTABLE_BYTECODE)
        {
            self.reject_unstable_bytecode(modules)?;
        }

        if self
            .features()
            .is_enabled(FeatureFlag::DISALLOW_USER_NATIVES)
        {
            verifier::native_validation::validate_module_natives(modules)?;
        }

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
            aptos_framework::verify_module_metadata(m, self.features(), self.timed_features())
                .map_err(|err| Self::metadata_validation_error(&err.to_string()))?;
        }
        verifier::resource_groups::validate_resource_groups(
            session,
            modules,
            self.features()
                .is_enabled(FeatureFlag::SAFER_RESOURCE_GROUPS),
        )?;
        verifier::event_validation::validate_module_events(session, modules)?;

        if !expected_modules.is_empty() {
            return Err(Self::metadata_validation_error(
                "not all registered modules published",
            ));
        }
        Ok(())
    }

    /// Check whether the bytecode can be published to mainnet based on the unstable tag in the metadata
    fn reject_unstable_bytecode(&self, modules: &[CompiledModule]) -> VMResult<()> {
        if self.chain_id().is_mainnet() {
            for module in modules {
                if let Some(metadata) =
                    aptos_framework::get_compilation_metadata_from_compiled_module(module)
                {
                    if metadata.unstable {
                        return Err(PartialVMError::new(StatusCode::UNSTABLE_BYTECODE_REJECTED)
                            .with_message(
                                "code marked unstable is not published on mainnet".to_string(),
                            )
                            .finish(Location::Undefined));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check whether the script can be run on mainnet based on the unstable tag in the metadata
    pub fn reject_unstable_bytecode_for_script(&self, module: &CompiledScript) -> VMResult<()> {
        if self.chain_id().is_mainnet() {
            if let Some(metadata) =
                aptos_framework::get_compilation_metadata_from_compiled_script(module)
            {
                if metadata.unstable {
                    return Err(PartialVMError::new(StatusCode::UNSTABLE_BYTECODE_REJECTED)
                        .with_message("script marked unstable cannot be run on mainnet".to_string())
                        .finish(Location::Script));
                }
            }
        }
        Ok(())
    }

    fn metadata_validation_error(msg: &str) -> VMError {
        PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
            .with_message(format!("metadata and code bundle mismatch: {}", msg))
            .finish(Location::Undefined)
    }

    fn validate_signed_transaction(
        &self,
        session: &mut SessionExt,
        resolver: &impl AptosMoveResolver,
        transaction: &SignedTransaction,
        transaction_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        is_approved_gov_script: bool,
        traversal_context: &mut TraversalContext,
    ) -> Result<(), VMStatus> {
        // Check transaction format.
        if transaction.contains_duplicate_signers() {
            return Err(VMStatus::error(
                StatusCode::SIGNERS_CONTAIN_DUPLICATES,
                None,
            ));
        }

        let keyless_authenticators = aptos_types::keyless::get_authenticators(transaction)
            .map_err(|_| VMStatus::error(StatusCode::INVALID_SIGNATURE, None))?;

        // If there are keyless TXN authenticators, validate them all.
        if !keyless_authenticators.is_empty() && !self.is_simulation {
            keyless_validation::validate_authenticators(
                &self.pvk,
                &keyless_authenticators,
                self.features(),
                resolver,
            )?;
        }

        // The prologue MUST be run AFTER any validation. Otherwise you may run prologue and hit
        // SEQUENCE_NUMBER_TOO_NEW if there is more than one transaction from the same sender and
        // end up skipping validation.
        self.run_prologue_with_payload(
            session,
            resolver,
            transaction.payload(),
            transaction_data,
            log_context,
            is_approved_gov_script,
            traversal_context,
        )
    }

    // Called when the execution of the user transaction fails, in order to discard the
    // transaction, or clean up the failed state.
    fn on_user_transaction_execution_failure(
        &self,
        prologue_session_change_set: SystemSessionChangeSet,
        err: VMStatus,
        resolver: &impl AptosMoveResolver,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        gas_meter: &mut impl AptosGasMeter,
        change_set_configs: &ChangeSetConfigs,
        new_published_modules_loaded: bool,
        traversal_context: &mut TraversalContext,
    ) -> (VMStatus, VMOutput) {
        // Invalidate the loader cache in case there was a new module loaded from a module
        // publish request that failed.
        // This ensures the loader cache is flushed later to align storage with the cache.
        // None of the modules in the bundle will be committed to storage,
        // but some of them may have ended up in the cache.
        if new_published_modules_loaded {
            self.move_vm.mark_loader_cache_as_invalid();
        };

        self.failed_transaction_cleanup(
            prologue_session_change_set,
            err,
            gas_meter,
            txn_data,
            resolver,
            log_context,
            change_set_configs,
            traversal_context,
        )
    }

    fn execute_user_transaction_impl(
        &self,
        resolver: &impl AptosMoveResolver,
        txn: &SignedTransaction,
        txn_data: TransactionMetadata,
        is_approved_gov_script: bool,
        gas_meter: &mut impl AptosGasMeter,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, VMOutput) {
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        // Revalidate the transaction.
        let mut prologue_session = PrologueSession::new(self, &txn_data, resolver);
        let exec_result = prologue_session.execute(|session| {
            self.validate_signed_transaction(
                session,
                resolver,
                txn,
                &txn_data,
                log_context,
                is_approved_gov_script,
                &mut traversal_context,
            )
        });
        unwrap_or_discard!(exec_result);
        let storage_gas_params = unwrap_or_discard!(get_or_vm_startup_failure(
            &self.storage_gas_params,
            log_context
        ));
        let change_set_configs = &storage_gas_params.change_set_configs;
        let (prologue_change_set, mut user_session) = unwrap_or_discard!(prologue_session
            .into_user_session(
                self,
                &txn_data,
                resolver,
                self.gas_feature_version,
                change_set_configs,
            ));

        let is_account_init_for_sponsored_transaction = unwrap_or_discard!(
            is_account_init_for_sponsored_transaction(&txn_data, self.features(), resolver)
        );
        if is_account_init_for_sponsored_transaction {
            unwrap_or_discard!(
                user_session.execute(|session| create_account_if_does_not_exist(
                    session,
                    gas_meter,
                    txn.sender(),
                    &mut traversal_context,
                ))
            );
        }

        // We keep track of whether any newly published modules are loaded into the Vm's loader
        // cache as part of executing transactions. This would allow us to decide whether the cache
        // should be flushed later.
        let mut new_published_modules_loaded = false;
        let result = match txn.payload() {
            payload @ TransactionPayload::Script(_)
            | payload @ TransactionPayload::EntryFunction(_) => self
                .execute_script_or_entry_function(
                    resolver,
                    user_session,
                    gas_meter,
                    &mut traversal_context,
                    &txn_data,
                    payload,
                    log_context,
                    &mut new_published_modules_loaded,
                    change_set_configs,
                ),
            TransactionPayload::Multisig(payload) => self.execute_or_simulate_multisig_transaction(
                resolver,
                user_session,
                &prologue_change_set,
                gas_meter,
                &mut traversal_context,
                &txn_data,
                payload,
                log_context,
                &mut new_published_modules_loaded,
                change_set_configs,
            ),

            // Deprecated. We cannot make this `unreachable!` because a malicious
            // validator can craft this transaction and cause the node to panic.
            TransactionPayload::ModuleBundle(_) => {
                unwrap_or_discard!(Err(deprecated_module_bundle!()))
            },
        };

        let gas_usage = txn_data
            .max_gas_amount()
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount set");
        TXN_GAS_USAGE.observe(u64::from(gas_usage) as f64);

        result.unwrap_or_else(|err| {
            self.on_user_transaction_execution_failure(
                prologue_change_set,
                err,
                resolver,
                &txn_data,
                log_context,
                gas_meter,
                change_set_configs,
                new_published_modules_loaded,
                &mut traversal_context,
            )
        })
    }

    /// Main entrypoint for executing a user transaction that also allows the customization of the
    /// gas meter to be used.
    pub fn execute_user_transaction_with_custom_gas_meter<G, F>(
        &self,
        resolver: &impl AptosMoveResolver,
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
        make_gas_meter: F,
    ) -> Result<(VMStatus, VMOutput, G), VMStatus>
    where
        G: AptosGasMeter,
        F: FnOnce(u64, VMGasParameters, StorageGasParameters, bool, Gas) -> G,
    {
        let txn_metadata = TransactionMetadata::new(txn);

        let is_approved_gov_script = is_approved_gov_script(resolver, txn, &txn_metadata);

        let balance = txn.max_gas_amount().into();
        let mut gas_meter = make_gas_meter(
            self.gas_feature_version,
            get_or_vm_startup_failure(&self.gas_params, log_context)?
                .vm
                .clone(),
            get_or_vm_startup_failure(&self.storage_gas_params, log_context)?.clone(),
            is_approved_gov_script,
            balance,
        );
        let (status, output) = self.execute_user_transaction_impl(
            resolver,
            txn,
            txn_metadata,
            is_approved_gov_script,
            &mut gas_meter,
            log_context,
        );

        Ok((status, output, gas_meter))
    }

    /// Alternative entrypoint for user transaction execution that allows customization based on
    /// the production gas meter.
    ///
    /// This can be useful for off-chain applications that wants to perform additional
    /// measurements or analysis while preserving the production gas behavior.
    pub fn execute_user_transaction_with_modified_gas_meter<G, F>(
        &self,
        resolver: &impl AptosMoveResolver,
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
        modify_gas_meter: F,
    ) -> Result<(VMStatus, VMOutput, G), VMStatus>
    where
        F: FnOnce(ProdGasMeter) -> G,
        G: AptosGasMeter,
    {
        self.execute_user_transaction_with_custom_gas_meter(
            resolver,
            txn,
            log_context,
            |gas_feature_version,
             vm_gas_params,
             storage_gas_params,
             is_approved_gov_script,
             meter_balance| {
                modify_gas_meter(make_prod_gas_meter(
                    gas_feature_version,
                    vm_gas_params,
                    storage_gas_params,
                    is_approved_gov_script,
                    meter_balance,
                ))
            },
        )
    }

    /// Executes a user transaction using the production gas meter.
    pub fn execute_user_transaction(
        &self,
        resolver: &impl AptosMoveResolver,
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, VMOutput) {
        match self.execute_user_transaction_with_custom_gas_meter(
            resolver,
            txn,
            log_context,
            make_prod_gas_meter,
        ) {
            Ok((vm_status, vm_output, _gas_meter)) => (vm_status, vm_output),
            Err(vm_status) => {
                let vm_output = discarded_output(vm_status.status_code());
                (vm_status, vm_output)
            },
        }
    }

    fn execute_write_set(
        &self,
        resolver: &impl AptosMoveResolver,
        write_set_payload: &WriteSetPayload,
        txn_sender: Option<AccountAddress>,
        session_id: SessionId,
    ) -> Result<(VMChangeSet, ModuleWriteSet), VMStatus> {
        match write_set_payload {
            WriteSetPayload::Direct(change_set) => {
                // this transaction is never delayed field capable.
                // it requires restarting execution afterwards,
                // which allows it to be used as last transaction in delayed_field_enabled context.
                let (change_set, module_write_set) =
                    create_vm_change_set_with_module_write_set_when_delayed_field_optimization_disabled(
                        change_set.clone(),
                    );

                // validate_waypoint_change_set checks that this is true, so we only log here.
                if !Self::should_restart_execution(change_set.events()) {
                    // This invariant needs to hold irrespectively, so we log error always.
                    // but if we are in delayed_field_optimization_capable context, we cannot execute any transaction after this.
                    // as transaction afterwards would be executed assuming delayed fields are exchanged and
                    // resource groups are split, but WriteSetPayload::Direct has materialized writes,
                    // and so after executing this transaction versioned state is inconsistent.
                    error!(
                        "[aptos_vm] direct write set finished without requiring should_restart_execution");
                }

                Ok((change_set, module_write_set))
            },
            WriteSetPayload::Script { script, execute_as } => {
                let mut tmp_session = self.new_session(resolver, session_id, None);
                let senders = match txn_sender {
                    None => vec![*execute_as],
                    Some(sender) => vec![sender, *execute_as],
                };

                let traversal_storage = TraversalStorage::new();
                let mut traversal_context = TraversalContext::new(&traversal_storage);

                self.validate_and_execute_script(
                    &mut tmp_session,
                    &mut UnmeteredGasMeter,
                    &mut traversal_context,
                    senders,
                    script,
                )?;

                let change_set_configs =
                    ChangeSetConfigs::unlimited_at_gas_feature_version(self.gas_feature_version);

                // TODO(George): This session should not publish modules, and should be using native
                //               code context instead.
                let (change_set, module_write_set) = tmp_session.finish(&change_set_configs)?;
                Ok((change_set, module_write_set))
            },
        }
    }

    fn read_change_set(
        &self,
        executor_view: &dyn ExecutorView,
        resource_group_view: &dyn ResourceGroupView,
        change_set: &VMChangeSet,
        module_write_set: &ModuleWriteSet,
    ) -> PartialVMResult<()> {
        assert!(
            change_set.aggregator_v1_write_set().is_empty(),
            "Waypoint change set should not have any aggregator writes."
        );

        // All Move executions satisfy the read-before-write property. Thus we need to read each
        // access path that the write set is going to update.
        for state_key in module_write_set.write_ops().keys() {
            executor_view.get_module_state_value(state_key)?;
        }
        for (state_key, write_op) in change_set.resource_write_set().iter() {
            executor_view.get_resource_state_value(state_key, None)?;
            if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write_op {
                for (tag, (_, maybe_layout)) in group_write.inner_ops() {
                    resource_group_view.get_resource_from_group(
                        state_key,
                        tag,
                        maybe_layout.as_deref(),
                    )?;
                }
            }
        }

        Ok(())
    }

    fn validate_waypoint_change_set(
        events: &[(ContractEvent, Option<MoveTypeLayout>)],
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let has_new_block_event = events
            .iter()
            .any(|(e, _)| e.event_key() == Some(&new_block_event_key()));
        let has_new_epoch_event = events
            .iter()
            .any(|(e, _)| e.event_key() == Some(&new_epoch_event_key()));
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
        resolver: &impl AptosMoveResolver,
        write_set_payload: WriteSetPayload,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        // TODO: user specified genesis id to distinguish different genesis write sets
        let genesis_id = HashValue::zero();
        let (change_set, module_write_set) = self.execute_write_set(
            resolver,
            &write_set_payload,
            Some(account_config::reserved_vm_address()),
            SessionId::genesis(genesis_id),
        )?;

        Self::validate_waypoint_change_set(change_set.events(), log_context)?;
        self.read_change_set(
            resolver.as_executor_view(),
            resolver.as_resource_group_view(),
            &change_set,
            &module_write_set,
        )
        .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;

        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = VMOutput::new(
            change_set,
            module_write_set,
            FeeStatement::zero(),
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        );
        Ok((VMStatus::Executed, output))
    }

    fn process_block_prologue(
        &self,
        resolver: &impl AptosMoveResolver,
        block_metadata: BlockMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        fail_point!("move_adapter::process_block_prologue", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, SessionId::block_meta(&block_metadata), None);

        let args = serialize_values(
            &block_metadata.get_prologue_move_args(account_config::reserved_vm_address()),
        );

        let storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &BLOCK_MODULE,
                BLOCK_PROLOGUE,
                vec![],
                args,
                &mut gas_meter,
                &mut TraversalContext::new(&storage),
            )
            .map(|_return_vals| ())
            .or_else(|e| {
                expect_only_successful_execution(e, BLOCK_PROLOGUE.as_str(), log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = get_system_transaction_output(
            session,
            &get_or_vm_startup_failure(&self.storage_gas_params, log_context)?.change_set_configs,
        )?;
        Ok((VMStatus::Executed, output))
    }

    fn process_block_prologue_ext(
        &self,
        resolver: &impl AptosMoveResolver,
        block_metadata_ext: BlockMetadataExt,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        fail_point!("move_adapter::process_block_prologue_ext", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(
            resolver,
            SessionId::block_meta_ext(&block_metadata_ext),
            None,
        );

        let block_metadata_with_randomness = match block_metadata_ext {
            BlockMetadataExt::V0(_) => unreachable!(),
            BlockMetadataExt::V1(v1) => v1,
        };

        let BlockMetadataWithRandomness {
            id,
            epoch,
            round,
            proposer,
            previous_block_votes_bitvec,
            failed_proposer_indices,
            timestamp_usecs,
            randomness,
        } = block_metadata_with_randomness;

        let args = vec![
            MoveValue::Signer(AccountAddress::ZERO), // Run as 0x0
            MoveValue::Address(AccountAddress::from_bytes(id.to_vec()).unwrap()),
            MoveValue::U64(epoch),
            MoveValue::U64(round),
            MoveValue::Address(proposer),
            failed_proposer_indices
                .into_iter()
                .map(|i| i as u64)
                .collect::<Vec<_>>()
                .as_move_value(),
            previous_block_votes_bitvec.as_move_value(),
            MoveValue::U64(timestamp_usecs),
            randomness
                .as_ref()
                .map(Randomness::randomness_cloned)
                .as_move_value(),
        ];

        let storage = TraversalStorage::new();

        session
            .execute_function_bypass_visibility(
                &BLOCK_MODULE,
                BLOCK_PROLOGUE_EXT,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&storage),
            )
            .map(|_return_vals| ())
            .or_else(|e| {
                expect_only_successful_execution(e, BLOCK_PROLOGUE_EXT.as_str(), log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = get_system_transaction_output(
            session,
            &get_or_vm_startup_failure(&self.storage_gas_params, log_context)?.change_set_configs,
        )?;
        Ok((VMStatus::Executed, output))
    }

    fn extract_module_metadata(&self, module: &ModuleId) -> Option<Arc<RuntimeModuleMetadataV1>> {
        if self.features().is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6) {
            aptos_framework::get_vm_metadata(&self.move_vm, module)
        } else {
            aptos_framework::get_vm_metadata_v0(&self.move_vm, module)
        }
    }

    pub fn execute_view_function(
        state_view: &impl StateView,
        module_id: ModuleId,
        func_name: Identifier,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
        max_gas_amount: u64,
    ) -> ViewFunctionOutput {
        let vm = AptosVM::new(state_view);

        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let vm_gas_params = match get_or_vm_startup_failure(&vm.gas_params, &log_context) {
            Ok(gas_params) => gas_params.vm.clone(),
            Err(err) => {
                return ViewFunctionOutput::new(Err(anyhow::Error::msg(format!("{}", err))), 0)
            },
        };
        let storage_gas_params =
            match get_or_vm_startup_failure(&vm.storage_gas_params, &log_context) {
                Ok(gas_params) => gas_params.clone(),
                Err(err) => {
                    return ViewFunctionOutput::new(Err(anyhow::Error::msg(format!("{}", err))), 0)
                },
            };

        let mut gas_meter = make_prod_gas_meter(
            vm.gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            /* is_approved_gov_script */ false,
            max_gas_amount.into(),
        );

        let resolver = state_view.as_move_resolver();
        let mut session = vm.new_session(&resolver, SessionId::Void, None);
        let execution_result = Self::execute_view_function_in_vm(
            &mut session,
            &vm,
            module_id,
            func_name,
            type_args,
            arguments,
            &mut gas_meter,
        );
        let gas_used = Self::gas_used(max_gas_amount.into(), &gas_meter);
        match execution_result {
            Ok(result) => ViewFunctionOutput::new(Ok(result), gas_used),
            Err(e) => ViewFunctionOutput::new(Err(e), gas_used),
        }
    }

    fn gas_used(max_gas_amount: Gas, gas_meter: &impl AptosGasMeter) -> u64 {
        max_gas_amount
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount")
            .into()
    }

    fn execute_view_function_in_vm(
        session: &mut SessionExt,
        vm: &AptosVM,
        module_id: ModuleId,
        func_name: Identifier,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
        gas_meter: &mut impl AptosGasMeter,
    ) -> anyhow::Result<Vec<Vec<u8>>> {
        let func = session.load_function(&module_id, &func_name, &type_args)?;
        let metadata = vm.extract_module_metadata(&module_id);
        let arguments = verifier::view_function::validate_view_function(
            session,
            arguments,
            func_name.as_ident_str(),
            &func,
            metadata.as_ref().map(Arc::as_ref),
            vm.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
        )?;

        let storage = TraversalStorage::new();

        Ok(session
            .execute_function_bypass_visibility(
                &module_id,
                func_name.as_ident_str(),
                type_args,
                arguments,
                gas_meter,
                &mut TraversalContext::new(&storage),
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
        resolver: &impl AptosMoveResolver,
        payload: &TransactionPayload,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        is_approved_gov_script: bool,
        traversal_context: &mut TraversalContext,
    ) -> Result<(), VMStatus> {
        check_gas(
            get_or_vm_startup_failure(&self.gas_params, log_context)?,
            self.gas_feature_version,
            resolver,
            txn_data,
            self.features(),
            is_approved_gov_script,
            log_context,
        )?;

        match payload {
            TransactionPayload::Script(_) | TransactionPayload::EntryFunction(_) => {
                transaction_validation::run_script_prologue(
                    session,
                    txn_data,
                    self.features(),
                    log_context,
                    traversal_context,
                    self.is_simulation,
                )
            },
            TransactionPayload::Multisig(multisig_payload) => {
                // Still run script prologue for multisig transaction to ensure the same tx
                // validations are still run for this multisig execution tx, which is submitted by
                // one of the owners.
                transaction_validation::run_script_prologue(
                    session,
                    txn_data,
                    self.features(),
                    log_context,
                    traversal_context,
                    self.is_simulation,
                )?;
                // Once "simulation_enhancement" is enabled, the simulation path also validates the
                // multisig transaction by running the multisig prologue.
                if !self.is_simulation
                    || self
                        .features()
                        .is_transaction_simulation_enhancement_enabled()
                {
                    transaction_validation::run_multisig_prologue(
                        session,
                        txn_data,
                        multisig_payload,
                        self.features(),
                        log_context,
                        traversal_context,
                    )
                } else {
                    Ok(())
                }
            },

            // Deprecated.
            TransactionPayload::ModuleBundle(_) => Err(deprecated_module_bundle!()),
        }
    }

    pub fn should_restart_execution(events: &[(ContractEvent, Option<MoveTypeLayout>)]) -> bool {
        let new_epoch_event_key = new_epoch_event_key();
        events
            .iter()
            .any(|(event, _)| event.event_key() == Some(&new_epoch_event_key))
    }

    /// Executes a single transaction (including user transactions, block
    /// metadata and state checkpoint, etc.).
    /// *Precondition:* VM has to be instantiated in execution mode.
    pub fn execute_single_transaction(
        &self,
        txn: &SignatureVerifiedTransaction,
        resolver: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        assert!(!self.is_simulation, "VM has to be created for execution");

        if let SignatureVerifiedTransaction::Invalid(_) = txn {
            let vm_status = VMStatus::error(StatusCode::INVALID_SIGNATURE, None);
            let discarded_output = discarded_output(vm_status.status_code());
            return Ok((vm_status, discarded_output));
        }

        Ok(match txn.expect_valid() {
            Transaction::BlockMetadata(block_metadata) => {
                fail_point!("aptos_vm::execution::block_metadata");
                let (vm_status, output) =
                    self.process_block_prologue(resolver, block_metadata.clone(), log_context)?;
                (vm_status, output)
            },
            Transaction::BlockMetadataExt(block_metadata_ext) => {
                fail_point!("aptos_vm::execution::block_metadata_ext");
                let (vm_status, output) = self.process_block_prologue_ext(
                    resolver,
                    block_metadata_ext.clone(),
                    log_context,
                )?;
                (vm_status, output)
            },
            Transaction::GenesisTransaction(write_set_payload) => {
                let (vm_status, output) = self.process_waypoint_change_set(
                    resolver,
                    write_set_payload.clone(),
                    log_context,
                )?;
                (vm_status, output)
            },
            Transaction::UserTransaction(txn) => {
                fail_point!("aptos_vm::execution::user_transaction");
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
                                    bcs::to_bytes::<SignedTransaction>(txn),
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
                                bcs::to_bytes::<SignedTransaction>(txn),
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
                                bcs::to_bytes::<SignedTransaction>(txn),
                                vm_status,
                            );
                            },
                        // Ignore DelayedFields speculative errors as it can be intentionally triggered by parallel execution.
                        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR => (),
                        // We will log the rest of invariant violation directly with regular logger as they shouldn't happen.
                        //
                        // TODO: Add different counters for the error categories here.
                        _ => {
                            error!(
                                *log_context,
                                "[aptos_vm] Transaction breaking invariant violation. txn: {:?}, status: {:?}",
                                bcs::to_bytes::<SignedTransaction>(txn),
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
                (vm_status, output)
            },
            Transaction::StateCheckpoint(_) => {
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                let output = VMOutput::empty_with_status(status);
                (VMStatus::Executed, output)
            },
            Transaction::BlockEpilogue(_) => {
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                let output = VMOutput::empty_with_status(status);
                (VMStatus::Executed, output)
            },
            Transaction::ValidatorTransaction(txn) => {
                let (vm_status, output) =
                    self.process_validator_transaction(resolver, txn.clone(), log_context)?;
                (vm_status, output)
            },
        })
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
        transactions: &[SignatureVerifiedTransaction],
        state_view: &(impl StateView + Sync),
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
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
            transactions,
            state_view,
            BlockExecutorConfig {
                local: BlockExecutorLocalConfig {
                    concurrency_level: Self::get_concurrency_level(),
                    allow_fallback: true,
                    discard_failed_blocks: Self::get_discard_failed_blocks(),
                },
                onchain: onchain_config,
            },
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
        onchain_config: BlockExecutorConfigFromOnchain,
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
            onchain_config,
        );
        if ret.is_ok() {
            // Record the histogram count for transactions per block.
            BLOCK_TRANSACTION_COUNT.observe(count as f64);
        }
        ret
    }
}

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

        if !self
            .features()
            .is_enabled(FeatureFlag::SINGLE_SENDER_AUTHENTICATOR)
        {
            if let aptos_types::transaction::authenticator::TransactionAuthenticator::SingleSender{ .. } = transaction.authenticator_ref() {
                return VMValidatorResult::error(StatusCode::FEATURE_UNDER_GATING);
            }
        }

        if !self.features().is_enabled(FeatureFlag::WEBAUTHN_SIGNATURE) {
            if let Ok(sk_authenticators) = transaction
                .authenticator_ref()
                .to_single_key_authenticators()
            {
                for authenticator in sk_authenticators {
                    if let AnySignature::WebAuthn { .. } = authenticator.signature() {
                        return VMValidatorResult::error(StatusCode::FEATURE_UNDER_GATING);
                    }
                }
            } else {
                return VMValidatorResult::error(StatusCode::INVALID_SIGNATURE);
            }
        }

        if !self
            .features()
            .is_enabled(FeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS)
        {
            if let TransactionPayload::Script(script) = transaction.payload() {
                for arg in script.args() {
                    if let TransactionArgument::Serialized(_) = arg {
                        return VMValidatorResult::error(StatusCode::FEATURE_UNDER_GATING);
                    }
                }
            }
        }

        let txn = match transaction.check_signature() {
            Ok(t) => t,
            _ => {
                return VMValidatorResult::error(StatusCode::INVALID_SIGNATURE);
            },
        };
        let txn_data = TransactionMetadata::new(&txn);

        let resolver = self.as_move_resolver(&state_view);
        let is_approved_gov_script = is_approved_gov_script(&resolver, &txn, &txn_data);

        let mut session = self.new_session(
            &resolver,
            SessionId::prologue_meta(&txn_data),
            Some(txn_data.as_user_transaction_context()),
        );

        let storage = TraversalStorage::new();

        // Increment the counter for transactions verified.
        let (counter_label, result) = match self.validate_signed_transaction(
            &mut session,
            &resolver,
            &txn,
            &txn_data,
            &log_context,
            is_approved_gov_script,
            &mut TraversalContext::new(&storage),
        ) {
            Err(err) if err.status_code() != StatusCode::SEQUENCE_NUMBER_TOO_NEW => (
                "failure",
                VMValidatorResult::new(Some(err.status_code()), 0),
            ),
            _ => (
                "success",
                VMValidatorResult::new(None, txn.gas_unit_price()),
            ),
        };

        TRANSACTIONS_VALIDATED
            .with_label_values(&[counter_label])
            .inc();

        result
    }
}

// Ensure encapsulation of AptosVM APIs by using a wrapper.
pub struct AptosSimulationVM(AptosVM);

impl AptosSimulationVM {
    pub fn new(state_view: &impl StateView) -> Self {
        let mut vm = AptosVM::new(state_view);
        vm.is_simulation = true;
        Self(vm)
    }

    /// Simulates a signed transaction (i.e., executes it without performing
    /// signature verification) on a newly created VM instance.
    /// *Precondition:* the transaction must **not** have a valid signature.
    pub fn create_vm_and_simulate_signed_transaction(
        transaction: &SignedTransaction,
        state_view: &impl StateView,
    ) -> (VMStatus, TransactionOutput) {
        assert_err!(
            transaction.verify_signature(),
            "Simulated transaction should not have a valid signature"
        );

        let vm = Self::new(state_view);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let resolver = state_view.as_move_resolver();
        let (vm_status, vm_output) =
            vm.0.execute_user_transaction(&resolver, transaction, &log_context);
        let txn_output = vm_output
            .try_materialize_into_transaction_output(&resolver)
            .expect("Materializing aggregator V1 deltas should never fail");
        (vm_status, txn_output)
    }
}

fn create_account_if_does_not_exist(
    session: &mut SessionExt,
    gas_meter: &mut impl GasMeter,
    account: AccountAddress,
    traversal_context: &mut TraversalContext,
) -> VMResult<()> {
    session
        .execute_function_bypass_visibility(
            &ACCOUNT_MODULE,
            CREATE_ACCOUNT_IF_DOES_NOT_EXIST,
            vec![],
            serialize_values(&vec![MoveValue::Address(account)]),
            gas_meter,
            traversal_context,
        )
        .map(|_return_vals| ())
}

/// Signals that the transaction should trigger the flow for creating an account as part of a
/// sponsored transaction. This occurs when:
/// * The feature gate is enabled SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION
/// * There is fee payer
/// * The sequence number is 0
/// * There is no account resource for the account
pub(crate) fn is_account_init_for_sponsored_transaction(
    txn_data: &TransactionMetadata,
    features: &Features,
    resolver: &impl AptosMoveResolver,
) -> VMResult<bool> {
    Ok(
        features.is_enabled(FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION)
            && txn_data.fee_payer.is_some()
            && txn_data.sequence_number == 0
            && resolver
                .get_resource_bytes_with_metadata_and_layout(
                    &txn_data.sender(),
                    &AccountResource::struct_tag(),
                    &resolver.get_module_metadata(&AccountResource::struct_tag().module_id()),
                    None,
                )
                .map(|(data, _)| data.is_none())
                .map_err(|e| e.finish(Location::Undefined))?,
    )
}

#[test]
fn vm_thread_safe() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<AptosVM>();
    assert_sync::<AptosVM>();
    assert_send::<MoveVmExt>();
    assert_sync::<MoveVmExt>();
}
