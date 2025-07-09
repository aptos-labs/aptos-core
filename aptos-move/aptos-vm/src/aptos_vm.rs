// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_executor::{AptosTransactionOutput, AptosVMBlockExecutorWrapper},
    counters::*,
    data_cache::{AsMoveResolver, StorageAdapter},
    errors::{discarded_output, expect_only_successful_execution},
    gas::{check_gas, make_prod_gas_meter, ProdGasMeter},
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
    transaction_validation,
    transaction_validation::{run_scheduled_txn_cleanup, run_scheduled_txn_epilogue},
    verifier::{
        event_validation, native_validation, resource_groups, transaction_arg_validation,
        view_function,
    },
    VMBlockExecutor, VMValidator,
};
use anyhow::anyhow;
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager,
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::{default::DefaultTxnProvider, TxnProvider},
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_framework::natives::code::PublishRequest;
use aptos_gas_algebra::{Gas, GasQuantity, NumBytes, Octa};
use aptos_gas_meter::{AptosGasMeter, GasAlgebra};
use aptos_gas_schedule::{
    gas_feature_versions::{RELEASE_V1_10, RELEASE_V1_27},
    AptosGasParameters, VMGasParameters,
};
use aptos_logger::{enabled, prelude::*, Level};
#[cfg(any(test, feature = "testing"))]
use aptos_types::state_store::StateViewId;
use aptos_types::{
    account_config::{self, new_block_event_key, AccountResource},
    block_executor::{
        config::{
            BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig,
            BlockExecutorModuleCacheLocalConfig,
        },
        partitioner::PartitionedTransactions,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    block_metadata::BlockMetadata,
    block_metadata_ext::{BlockMetadataExt, BlockMetadataWithRandomness},
    chain_id::ChainId,
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    function_info::FunctionInfo,
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{
        ApprovedExecutionHashes, ConfigStorage, FeatureFlag, Features, OnChainConfig,
        TimedFeatureFlag, TimedFeatures,
    },
    randomness::Randomness,
    state_store::{StateView, TStateView},
    transaction::{
        authenticator::{AbstractionAuthData, AnySignature, AuthenticationProof},
        block_epilogue::{BlockEpiloguePayload, FeeDistribution},
        scheduled_txn::{ScheduledTransactionInfoWithKey, SCHEDULED_TRANSACTIONS_MODULE_INFO},
        signature_verified_transaction::SignatureVerifiedTransaction,
        BlockOutput, EntryFunction, ExecutionError, ExecutionStatus, ModuleBundle,
        MultisigTransactionPayload, ReplayProtector, Script, SignedTransaction, Transaction,
        TransactionArgument, TransactionExecutableRef, TransactionExtraConfig, TransactionOutput,
        TransactionPayload, TransactionStatus, VMValidatorResult, ViewFunctionOutput,
        WriteSetPayload,
    },
    vm::module_metadata::{
        get_compilation_metadata, get_metadata, get_randomness_annotation_for_entry_function,
        verify_module_metadata_for_module_publishing,
    },
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{log_schema::AdapterLogSchema, speculative_error, speculative_log};
use aptos_vm_types::{
    abstract_write_op::AbstractResourceWriteOp,
    change_set::{
        create_vm_change_set_with_module_write_set_when_delayed_field_optimization_disabled,
        ChangeSetInterface, VMChangeSet,
    },
    module_and_script_storage::{
        code_storage::AptosCodeStorage, module_storage::AptosModuleStorage, AsAptosCodeStorage,
    },
    module_write_set::ModuleWriteSet,
    output::VMOutput,
    resolver::{
        BlockSynchronizationKillSwitch, ExecutorView, NoopBlockSynchronizationKillSwitch,
        ResourceGroupView,
    },
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
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
    transaction_argument::convert_txn_args,
    value::{serialize_values, MoveTypeLayout, MoveValue},
    vm_status::{
        StatusCode::{ACCOUNT_AUTHENTICATION_GAS_LIMIT_EXCEEDED, OUT_OF_GAS},
        StatusType,
    },
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_runtime::{
    check_dependencies_and_charge_gas, check_script_dependencies_and_check_gas,
    check_type_tag_dependencies_and_charge_gas,
    logging::expect_no_verification_errors,
    module_traversal::{TraversalContext, TraversalStorage},
    ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
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

pub(crate) struct SerializedSigners {
    senders: Vec<Vec<u8>>,
    fee_payer: Option<Vec<u8>>,
}

impl SerializedSigners {
    pub fn new(senders: Vec<Vec<u8>>, fee_payer: Option<Vec<u8>>) -> Self {
        Self { senders, fee_payer }
    }

    pub fn sender(&self) -> Vec<u8> {
        self.senders[0].clone()
    }

    pub fn senders(&self) -> Vec<Vec<u8>> {
        self.senders.clone()
    }

    pub fn fee_payer(&self) -> Option<Vec<u8>> {
        self.fee_payer.clone()
    }
}

pub(crate) fn serialized_signer(account_address: &AccountAddress) -> Vec<u8> {
    MoveValue::Signer(*account_address)
        .simple_serialize()
        .unwrap()
}

pub(crate) fn get_system_transaction_output(
    session: SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    change_set_configs: &ChangeSetConfigs,
) -> Result<VMOutput, VMStatus> {
    let change_set = session.finish(change_set_configs, module_storage)?;

    Ok(VMOutput::new(
        change_set,
        ModuleWriteSet::empty(),
        FeeStatement::zero(),
        TransactionStatus::Keep(ExecutionStatus::Success),
    ))
}

pub(crate) fn get_sched_txn_output(
    session: SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    change_set_configs: &ChangeSetConfigs,
    fee_statement: FeeStatement,
) -> Result<VMOutput, VMStatus> {
    let change_set = session.finish(change_set_configs, module_storage)?;

    Ok(VMOutput::new(
        change_set,
        ModuleWriteSet::empty(), // todo: is this empty always? If so, do we explicitly limit the sched txn ?
        fee_statement,
        TransactionStatus::Keep(ExecutionStatus::Success),
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
    if let Ok(TransactionExecutableRef::Script(_script)) = txn.payload().executable_ref() {
        match ApprovedExecutionHashes::fetch_config(resolver) {
            Some(approved_execution_hashes) => approved_execution_hashes
                .entries
                .iter()
                .any(|(_, hash)| hash == &txn_metadata.script_hash),
            None => false,
        }
    } else {
        false
    }
}

pub struct AptosVM {
    is_simulation: bool,
    move_vm: MoveVmExt,
    /// For a new chain, or even mainnet, the VK might not necessarily be set.
    pvk: Option<PreparedVerifyingKey<Bn254>>,
}

impl AptosVM {
    /// Creates a new VM instance based on the runtime environment. The VM can then be used by
    /// block executor to create multiple tasks sharing the same execution configurations extracted
    /// from the environment.
    // TODO: Passing `state_view` is not needed once we move keyless configs to the environment.
    pub fn new(env: &AptosEnvironment, state_view: &impl StateView) -> Self {
        let resolver = state_view.as_move_resolver();
        let module_storage = state_view.as_aptos_code_storage(env);

        // We use an `Option` to handle the VK not being set on-chain, or an incorrect VK being set
        // via governance (although, currently, we do check for that in `keyless_account.move`).
        let pvk = keyless_validation::get_groth16_vk_onchain(&resolver, &module_storage)
            .ok()
            .and_then(|vk| vk.try_into().ok());

        Self {
            is_simulation: false,
            move_vm: MoveVmExt::new(env),
            pvk,
        }
    }

    pub fn new_session<'r, R: AptosMoveResolver>(
        &self,
        resolver: &'r R,
        session_id: SessionId,
        user_transaction_context_opt: Option<UserTransactionContext>,
    ) -> SessionExt<'r, R> {
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

    #[inline(always)]
    pub(crate) fn gas_feature_version(&self) -> u64 {
        self.move_vm.env.gas_feature_version()
    }

    #[inline(always)]
    pub(crate) fn gas_params(
        &self,
        log_context: &AdapterLogSchema,
    ) -> Result<&AptosGasParameters, VMStatus> {
        get_or_vm_startup_failure(self.move_vm.env.gas_params(), log_context)
    }

    #[inline(always)]
    pub(crate) fn storage_gas_params(
        &self,
        log_context: &AdapterLogSchema,
    ) -> Result<&StorageGasParameters, VMStatus> {
        get_or_vm_startup_failure(self.move_vm.env.storage_gas_params(), log_context)
    }

    #[inline(always)]
    pub fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.move_vm.env.runtime_environment()
    }

    #[inline(always)]
    pub fn environment(&self) -> AptosEnvironment {
        self.move_vm.env.clone()
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
    pub fn gas_params_for_test(&self) -> Result<&AptosGasParameters, VMStatus> {
        let log_context = AdapterLogSchema::new(StateViewId::Miscellaneous, 0);
        self.gas_params(&log_context)
    }

    pub fn as_move_resolver<'r, R: ExecutorView>(
        &self,
        executor_view: &'r R,
    ) -> StorageAdapter<'r, R> {
        StorageAdapter::new_with_config(
            executor_view,
            self.gas_feature_version(),
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
            self.gas_feature_version(),
            self.features(),
            Some(executor_view),
        )
    }

    fn fee_statement_from_gas_meter(
        max_gas_units: Gas,
        gas_meter: &impl AptosGasMeter,
        storage_fee_refund: u64,
    ) -> FeeStatement {
        let gas_used = Self::gas_used(max_gas_units, gas_meter);
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
        module_storage: &impl AptosModuleStorage,
        serialized_signers: &SerializedSigners,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
    ) -> (VMStatus, VMOutput) {
        if self.gas_feature_version() >= 12 {
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

        let txn_status = TransactionStatus::from_vm_status(
            error_vm_status.clone(),
            self.features()
                .is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION),
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
                        module_storage,
                        serialized_signers,
                        status,
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

    fn inject_abort_info_if_available(
        &self,
        module_storage: &impl AptosModuleStorage,
        status: ExecutionStatus,
    ) -> ExecutionStatus {
        if let ExecutionStatus::MoveAbort {
            location: AbortLocation::Module(module_id),
            code,
            ..
        } = status
        {
            let info = module_storage
                .fetch_module_metadata(module_id.address(), module_id.name())
                .ok()
                .flatten()
                .and_then(|metadata| get_metadata(&metadata))
                .and_then(|m| m.extract_abort_info(code));
            ExecutionStatus::MoveAbort {
                location: AbortLocation::Module(module_id),
                code,
                info,
            }
        } else {
            status
        }
    }

    fn finish_aborted_transaction(
        &self,
        prologue_session_change_set: SystemSessionChangeSet,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        serialized_signers: &SerializedSigners,
        status: ExecutionStatus,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
    ) -> Result<VMOutput, VMStatus> {
        // Storage refund is zero since no slots are deleted in aborted transactions.
        const ZERO_STORAGE_REFUND: u64 = 0;

        let should_create_account_resource =
            should_create_account_resource(txn_data, self.features(), resolver, module_storage)?;

        let (previous_session_change_set, fee_statement) = if should_create_account_resource {
            let mut abort_hook_session =
                AbortHookSession::new(self, txn_data, resolver, prologue_session_change_set);

            abort_hook_session.execute(|session| {
                create_account_if_does_not_exist(
                    session,
                    module_storage,
                    gas_meter,
                    txn_data.sender(),
                    traversal_context,
                )
                // If this fails, it is likely due to out of gas, so we try again without metering
                // and then validate below that we charged sufficiently.
                .or_else(|_err| {
                    create_account_if_does_not_exist(
                        session,
                        module_storage,
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
                abort_hook_session.finish(change_set_configs, module_storage)?;
            if let Err(err) = self.charge_change_set(
                &mut abort_hook_session_change_set,
                gas_meter,
                txn_data,
                resolver,
                module_storage,
            ) {
                info!(
                    *log_context,
                    "Failed during charge_change_set: {:?}. Most likely exceeded gas limited.", err,
                );
            };

            let fee_statement = AptosVM::fee_statement_from_gas_meter(
                txn_data.max_gas_amount(),
                gas_meter,
                ZERO_STORAGE_REFUND,
            );

            // Verify we charged sufficiently for creating an account slot
            let gas_params = self.gas_params(log_context)?;
            let gas_unit_price = u64::from(txn_data.gas_unit_price());
            if gas_unit_price != 0 || !self.features().is_default_account_resource_enabled() {
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
                                "Insufficient fee for storing account for lazy account creation"
                                    .to_string(),
                            )
                            .finish(Location::Undefined),
                        &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
                        log_context,
                    )?;
                }
            }
            (abort_hook_session_change_set, fee_statement)
        } else {
            let fee_statement = AptosVM::fee_statement_from_gas_meter(
                txn_data.max_gas_amount(),
                gas_meter,
                ZERO_STORAGE_REFUND,
            );
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
        let status = self.inject_abort_info_if_available(module_storage, status);
        epilogue_session.execute(|session| {
            transaction_validation::run_failure_epilogue(
                session,
                module_storage,
                serialized_signers,
                gas_meter.balance(),
                fee_statement,
                self.features(),
                txn_data,
                log_context,
                traversal_context,
                self.is_simulation,
            )
        })?;
        epilogue_session.finish(fee_statement, status, change_set_configs, module_storage)
    }

    fn success_transaction_cleanup(
        &self,
        mut epilogue_session: EpilogueSession,
        module_storage: &impl AptosModuleStorage,
        serialized_signers: &SerializedSigners,
        gas_meter: &impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        if self.gas_feature_version() >= 12 {
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
            txn_data.max_gas_amount(),
            gas_meter,
            u64::from(epilogue_session.get_storage_fee_refund()),
        );
        epilogue_session.execute(|session| {
            transaction_validation::run_success_epilogue(
                session,
                module_storage,
                serialized_signers,
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
            change_set_configs,
            module_storage,
        )?;

        Ok((VMStatus::Executed, output))
    }

    fn validate_and_execute_script<'a>(
        &self,
        session: &mut SessionExt<impl AptosMoveResolver>,
        serialized_signers: &SerializedSigners,
        code_storage: &impl AptosCodeStorage,
        // Note: cannot use AptosGasMeter because it is not implemented for
        //       UnmeteredGasMeter.
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext<'a>,
        serialized_script: &'a Script,
    ) -> Result<(), VMStatus> {
        if !self
            .features()
            .is_enabled(FeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS)
        {
            for arg in serialized_script.args() {
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
        if self.gas_feature_version() >= RELEASE_V1_10 {
            check_script_dependencies_and_check_gas(
                code_storage,
                gas_meter,
                traversal_context,
                serialized_script.code(),
            )?;
        }
        if self.gas_feature_version() >= RELEASE_V1_27 {
            check_type_tag_dependencies_and_charge_gas(
                code_storage,
                gas_meter,
                traversal_context,
                serialized_script.ty_args(),
            )?;
        }

        let func =
            code_storage.load_script(serialized_script.code(), serialized_script.ty_args())?;

        // Check that unstable bytecode cannot be executed on mainnet and verify events.
        let script = func.owner_as_script()?;
        self.reject_unstable_bytecode_for_script(script)?;
        event_validation::verify_no_event_emission_in_compiled_script(script)?;

        let args = transaction_arg_validation::validate_combine_signer_and_txn_args(
            session,
            code_storage,
            serialized_signers,
            convert_txn_args(serialized_script.args()),
            &func,
            self.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
        )?;

        session.execute_loaded_function(func, args, gas_meter, traversal_context, code_storage)?;
        Ok(())
    }

    fn validate_and_execute_entry_function(
        &self,
        module_storage: &impl AptosModuleStorage,
        session: &mut SessionExt<impl AptosMoveResolver>,
        serialized_signers: &SerializedSigners,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        entry_fn: &EntryFunction,
    ) -> Result<(), VMStatus> {
        // Note: Feature gating is needed here because the traversal of the dependencies could
        //       result in shallow-loading of the modules and therefore subtle changes in
        //       the error semantics.
        if self.gas_feature_version() >= RELEASE_V1_10 {
            let module_id = traversal_context
                .referenced_module_ids
                .alloc(entry_fn.module().clone());
            check_dependencies_and_charge_gas(module_storage, gas_meter, traversal_context, [(
                module_id.address(),
                module_id.name(),
            )])?;
        }

        if self.gas_feature_version() >= RELEASE_V1_27 {
            check_type_tag_dependencies_and_charge_gas(
                module_storage,
                gas_meter,
                traversal_context,
                entry_fn.ty_args(),
            )?;
        }

        let function = module_storage.load_function(
            entry_fn.module(),
            entry_fn.function(),
            entry_fn.ty_args(),
        )?;

        // Native entry function is forbidden.
        if function.is_native() {
            return Err(
                PartialVMError::new(StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED)
                    .with_message(
                        "Executing user defined native entry function is not allowed".to_string(),
                    )
                    .finish(Location::Module(entry_fn.module().clone()))
                    .into_vm_status(),
            );
        }

        // The check below should have been feature-gated in 1.11...
        if function.is_friend_or_private() {
            let maybe_randomness_annotation = get_randomness_annotation_for_entry_function(
                entry_fn,
                &function.owner_as_module()?.metadata,
            );
            if maybe_randomness_annotation.is_some() {
                session.mark_unbiasable();
            }
        }

        let struct_constructors_enabled =
            self.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS);
        let args = transaction_arg_validation::validate_combine_signer_and_txn_args(
            session,
            module_storage,
            serialized_signers,
            entry_fn.args().to_vec(),
            &function,
            struct_constructors_enabled,
        )?;

        // Execute the function. The function also must be an entry function!
        function.is_entry_or_err()?;
        session.execute_loaded_function(
            function,
            args,
            gas_meter,
            traversal_context,
            module_storage,
        )?;
        Ok(())
    }

    fn execute_script_or_entry_function<'a, 'r>(
        &self,
        resolver: &'r impl AptosMoveResolver,
        code_storage: &impl AptosCodeStorage,
        mut session: UserSession<'r>,
        serialized_signers: &SerializedSigners,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext<'a>,
        txn_data: &TransactionMetadata,
        executable: TransactionExecutableRef<'a>, // TODO[Orderless]: Check what's the right lifetime to use here.
        log_context: &AdapterLogSchema,
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

        match executable {
            TransactionExecutableRef::Script(script) => {
                session.execute(|session| {
                    self.validate_and_execute_script(
                        session,
                        serialized_signers,
                        code_storage,
                        gas_meter,
                        traversal_context,
                        script,
                    )
                })?;
            },
            TransactionExecutableRef::EntryFunction(entry_fn) => {
                session.execute(|session| {
                    self.validate_and_execute_entry_function(
                        code_storage,
                        session,
                        serialized_signers,
                        gas_meter,
                        traversal_context,
                        entry_fn,
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
            code_storage,
            gas_meter,
            traversal_context,
            change_set_configs,
        )?;

        let epilogue_session = self.charge_change_set_and_respawn_session(
            user_session_change_set,
            resolver,
            code_storage,
            gas_meter,
            txn_data,
        )?;

        // ============= Gas fee cannot change after this line =============

        self.success_transaction_cleanup(
            epilogue_session,
            code_storage,
            serialized_signers,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
            traversal_context,
        )
    }

    fn charge_change_set(
        &self,
        change_set: &mut impl ChangeSetInterface,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
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
            module_storage,
        )?;
        if !self.features().is_storage_deletion_refund_enabled() {
            storage_refund = 0.into();
        }

        Ok(storage_refund)
    }

    fn charge_change_set_and_respawn_session<'r>(
        &self,
        mut user_session_change_set: UserSessionChangeSet,
        resolver: &'r impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
    ) -> Result<EpilogueSession<'r>, VMStatus> {
        let storage_refund = self.charge_change_set(
            &mut user_session_change_set,
            gas_meter,
            txn_data,
            resolver,
            module_storage,
        )?;

        // TODO[agg_v1](fix): Charge for aggregator writes
        Ok(EpilogueSession::on_user_session_success(
            self,
            txn_data,
            resolver,
            user_session_change_set,
            storage_refund,
        ))
    }

    // Execute a multisig transaction:
    // 1. Obtain the payload of the transaction to execute. This could have been stored on chain
    // when the multisig transaction was created.
    // 2. Execute the target payload. If this fails, discard the session and keep the gas meter and
    // failure object. In case of success, keep the session and also do any necessary module publish
    // cleanup.
    // 3. Call post transaction cleanup function in multisig account module with the result from (2)
    fn execute_multisig_transaction<'r>(
        &self,
        resolver: &'r impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        mut session: UserSession<'r>,
        serialized_signers: &SerializedSigners,
        prologue_session_change_set: &SystemSessionChangeSet,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        txn_data: &TransactionMetadata,
        executable: TransactionExecutableRef,
        multisig_address: AccountAddress,
        log_context: &AdapterLogSchema,
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
        let provided_payload = match executable {
            TransactionExecutableRef::EntryFunction(entry_func) => {
                // TODO[Orderless]: For backward compatibility reasons, still using `MultisigTransactionPayload` here.
                // Find a way to deprecate this.
                bcs::to_bytes(&MultisigTransactionPayload::EntryFunction(
                    entry_func.clone(),
                ))
                .map_err(|_| invariant_violation_error())?
            },
            TransactionExecutableRef::Empty => {
                // Default to empty bytes if payload is not provided.
                if self
                    .features()
                    .is_abort_if_multisig_payload_mismatch_enabled()
                {
                    vec![]
                } else {
                    bcs::to_bytes::<Vec<u8>>(&vec![]).map_err(|_| invariant_violation_error())?
                }
            },
            TransactionExecutableRef::Script(_) => {
                let s = VMStatus::error(
                    StatusCode::FEATURE_UNDER_GATING,
                    Some("Multisig transaction does not support script payload".to_string()),
                );
                return Ok((s, discarded_output(StatusCode::FEATURE_UNDER_GATING)));
            },
        };
        // Failures here will be propagated back.
        let payload_bytes: Vec<Vec<u8>> = session
            .execute(|session| {
                session.execute_function_bypass_visibility(
                    &MULTISIG_ACCOUNT_MODULE,
                    GET_NEXT_TRANSACTION_PAYLOAD,
                    vec![],
                    serialize_values(&vec![
                        MoveValue::Address(multisig_address),
                        MoveValue::vector_u8(provided_payload),
                    ]),
                    gas_meter,
                    traversal_context,
                    module_storage,
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
        let deserialization_error = || {
            PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
                .finish(Location::Undefined)
        };
        let payload_bytes =
            bcs::from_bytes::<Vec<u8>>(payload_bytes).map_err(|_| deserialization_error())?;
        let payload = bcs::from_bytes::<MultisigTransactionPayload>(&payload_bytes)
            .map_err(|_| deserialization_error())?;

        // Step 2: Execute the target payload. Transaction failure here is tolerated. In case of any
        // failures, we'll discard the session and start a new one. This ensures that any data
        // changes are not persisted.
        // The multisig transaction would still be considered executed even if execution fails.
        let execution_result = match payload {
            MultisigTransactionPayload::EntryFunction(entry_function) => self
                .execute_multisig_entry_function(
                    resolver,
                    module_storage,
                    session,
                    gas_meter,
                    traversal_context,
                    multisig_address,
                    &entry_function,
                    change_set_configs,
                ),
        };

        // Step 3: Call post transaction cleanup function in multisig account module with the result
        // from Step 2.
        // Note that we don't charge execution or writeset gas for cleanup routines. This is
        // consistent with the high-level success/failure cleanup routines for user transactions.
        let cleanup_args = serialize_values(&vec![
            MoveValue::Address(txn_data.sender),
            MoveValue::Address(multisig_address),
            MoveValue::vector_u8(payload_bytes),
        ]);

        let epilogue_session = match execution_result {
            Err(execution_error) => self.failure_multisig_payload_cleanup(
                resolver,
                module_storage,
                prologue_session_change_set,
                execution_error,
                txn_data,
                cleanup_args,
                traversal_context,
            )?,
            Ok(user_session_change_set) => {
                // Charge gas for write set before we do cleanup. This ensures we don't charge gas for
                // cleanup write set changes, which is consistent with outer-level success cleanup
                // flow. We also wouldn't need to worry that we run out of gas when doing cleanup.
                let mut epilogue_session = self.charge_change_set_and_respawn_session(
                    user_session_change_set,
                    resolver,
                    module_storage,
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
                            module_storage,
                        )
                        .map_err(|e| e.into_vm_status())
                })?;
                epilogue_session
            },
        };

        // TODO(Gas): Charge for aggregator writes
        self.success_transaction_cleanup(
            epilogue_session,
            module_storage,
            serialized_signers,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
            traversal_context,
        )
    }

    fn execute_multisig_entry_function(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        mut session: UserSession,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        multisig_address: AccountAddress,
        payload: &EntryFunction,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        // If txn args are not valid, we'd still consider the transaction as executed but
        // failed. This is primarily because it's unrecoverable at this point.
        session.execute(|session| {
            self.validate_and_execute_entry_function(
                module_storage,
                session,
                &SerializedSigners::new(vec![serialized_signer(&multisig_address)], None),
                gas_meter,
                traversal_context,
                payload,
            )
        })?;

        // Resolve any pending module publishes in case the multisig transaction is deploying
        // modules.
        self.resolve_pending_code_publish_and_finish_user_session(
            session,
            resolver,
            module_storage,
            gas_meter,
            traversal_context,
            change_set_configs,
        )
    }

    fn failure_multisig_payload_cleanup<'r>(
        &self,
        resolver: &'r impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        prologue_session_change_set: &SystemSessionChangeSet,
        execution_error: VMStatus,
        txn_data: &TransactionMetadata,
        mut cleanup_args: Vec<Vec<u8>>,
        traversal_context: &mut TraversalContext,
    ) -> Result<EpilogueSession<'r>, VMStatus> {
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
                    module_storage,
                )
                .map_err(|e| e.into_vm_status())
        })?;
        Ok(epilogue_session)
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
        mut session: UserSession,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        let maybe_publish_request = session.execute(|session| session.extract_publish_request());
        if maybe_publish_request.is_none() {
            let change_set = session.finish(change_set_configs, module_storage)?;
            return UserSessionChangeSet::new(
                change_set,
                ModuleWriteSet::empty(),
                change_set_configs,
            );
        }

        let PublishRequest {
            destination,
            bundle,
            expected_modules,
            allowed_deps,
            check_compat: _,
        } = maybe_publish_request.expect("Publish request exists");

        let modules = self.deserialize_module_bundle(&bundle)?;
        let modules: &Vec<CompiledModule> =
            traversal_context.referenced_module_bundles.alloc(modules);

        // Note: Feature gating is needed here because the traversal of the dependencies could
        //       result in shallow-loading of the modules and therefore subtle changes in
        //       the error semantics.
        if self.gas_feature_version() >= RELEASE_V1_10 {
            // Charge old versions of existing modules, in case of upgrades.
            for module in modules.iter() {
                let addr = module.self_addr();
                let name = module.self_name();

                if !traversal_context.visit_if_not_special_address(addr, name) {
                    continue;
                }

                let size_if_old_module_exists = module_storage
                    .fetch_module_size_in_bytes(addr, name)?
                    .map(|v| v as u64);
                if let Some(old_size) = size_if_old_module_exists {
                    gas_meter
                        .charge_dependency(false, addr, name, NumBytes::new(old_size))
                        .map_err(|err| {
                            err.finish(Location::Module(ModuleId::new(*addr, name.to_owned())))
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

            check_dependencies_and_charge_gas(
                module_storage,
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

        for (module, blob) in modules.iter().zip(bundle.iter()) {
            // TODO(Gas): Make budget configurable.
            let budget = 2048 + blob.code().len() as u64 * 20;
            move_binary_format::check_complexity::check_module_complexity(module, budget)
                .map_err(|err| err.finish(Location::Undefined))?;
        }

        self.validate_publish_request(
            module_storage,
            traversal_context,
            gas_meter,
            modules,
            expected_modules,
            allowed_deps,
        )?;

        let check_struct_layout = true;
        let check_friend_linking = !self
            .features()
            .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE);
        let compatibility_checks = Compatibility::new(
            check_struct_layout,
            check_friend_linking,
            self.timed_features()
                .is_enabled(TimedFeatureFlag::EntryCompatibility),
        );

        session.finish_with_module_publishing_and_initialization(
            resolver,
            module_storage,
            gas_meter,
            traversal_context,
            self.features(),
            self.gas_feature_version(),
            change_set_configs,
            destination,
            bundle,
            modules,
            compatibility_checks,
        )
    }

    /// Validate a publish request.
    fn validate_publish_request(
        &self,
        module_storage: &impl AptosModuleStorage,
        traversal_context: &TraversalContext,
        gas_meter: &mut impl GasMeter,
        modules: &[CompiledModule],
        mut expected_modules: BTreeSet<String>,
        allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
    ) -> VMResult<()> {
        self.reject_unstable_bytecode(modules)?;
        native_validation::validate_module_natives(modules)?;

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
            verify_module_metadata_for_module_publishing(m, self.features())
                .map_err(|err| Self::metadata_validation_error(&err.to_string()))?;
        }

        resource_groups::validate_resource_groups(
            self.features(),
            module_storage,
            traversal_context,
            gas_meter,
            modules,
        )?;
        event_validation::validate_module_events(
            self.features(),
            self.gas_feature_version(),
            module_storage,
            traversal_context,
            modules,
        )?;

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
                if let Some(metadata) = get_compilation_metadata(module) {
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
    pub fn reject_unstable_bytecode_for_script(&self, script: &CompiledScript) -> VMResult<()> {
        if self.chain_id().is_mainnet() {
            if let Some(metadata) = get_compilation_metadata(script) {
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
        session: &mut SessionExt<impl AptosMoveResolver>,
        module_storage: &impl ModuleStorage,
        transaction: &SignedTransaction,
        transaction_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        is_approved_gov_script: bool,
        traversal_context: &mut TraversalContext,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<SerializedSigners, VMStatus> {
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
                session.resolver,
                module_storage,
            )?;
        }

        // Account Abstraction dispatchable authentication.
        let senders = transaction_data.senders();
        let proofs = transaction_data.authentication_proofs();

        // Add fee payer.
        let fee_payer_signer = if let Some(fee_payer) = transaction_data.fee_payer {
            Some(match &transaction_data.fee_payer_authentication_proof {
                Some(AuthenticationProof::Abstraction {
                    function_info,
                    auth_data,
                }) => {
                    let enabled = match auth_data {
                        AbstractionAuthData::V1 { .. } => {
                            self.features().is_account_abstraction_enabled()
                        },
                        AbstractionAuthData::DerivableV1 { .. } => {
                            self.features().is_derivable_account_abstraction_enabled()
                        },
                    };
                    if enabled {
                        dispatchable_authenticate(
                            session,
                            gas_meter,
                            fee_payer,
                            function_info.clone(),
                            auth_data,
                            traversal_context,
                            module_storage,
                        )
                        .map_err(|mut vm_error| {
                            if vm_error.major_status() == OUT_OF_GAS {
                                vm_error
                                    .set_major_status(ACCOUNT_AUTHENTICATION_GAS_LIMIT_EXCEEDED);
                            }
                            vm_error.into_vm_status()
                        })
                    } else {
                        return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
                    }
                },
                _ => Ok(serialized_signer(&fee_payer)),
            }?)
        } else {
            None
        };
        let sender_signers = itertools::zip_eq(senders, proofs)
            .map(|(sender, proof)| match proof {
                AuthenticationProof::Abstraction {
                    function_info,
                    auth_data,
                } => {
                    let enabled = match auth_data {
                        AbstractionAuthData::V1 { .. } => {
                            self.features().is_account_abstraction_enabled()
                        },
                        AbstractionAuthData::DerivableV1 { .. } => {
                            self.features().is_derivable_account_abstraction_enabled()
                        },
                    };
                    if enabled {
                        dispatchable_authenticate(
                            session,
                            gas_meter,
                            sender,
                            function_info.clone(),
                            auth_data,
                            traversal_context,
                            module_storage,
                        )
                        .map_err(|mut vm_error| {
                            if vm_error.major_status() == OUT_OF_GAS {
                                vm_error
                                    .set_major_status(ACCOUNT_AUTHENTICATION_GAS_LIMIT_EXCEEDED);
                            }
                            vm_error.into_vm_status()
                        })
                    } else {
                        Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None))
                    }
                },
                _ => Ok(serialized_signer(&sender)),
            })
            .collect::<Result<_, _>>()?;

        let serialized_signers = SerializedSigners::new(sender_signers, fee_payer_signer);

        if matches!(transaction.payload(), TransactionPayload::Payload(_))
            && !self.features().is_transaction_payload_v2_enabled()
        {
            return Err(VMStatus::error(
                StatusCode::FEATURE_UNDER_GATING,
                Some(
                    "User transactions with TransactionPayloadInner variant are not yet supported"
                        .to_string(),
                ),
            ));
        }

        if !self.features().is_orderless_txns_enabled() {
            if let ReplayProtector::Nonce(_) = transaction.replay_protector() {
                return Err(VMStatus::error(
                    StatusCode::FEATURE_UNDER_GATING,
                    Some("Orderless transactions are not yet supported".to_string()),
                ));
            }
        }

        // The prologue MUST be run AFTER any validation. Otherwise you may run prologue and hit
        // SEQUENCE_NUMBER_TOO_NEW if there is more than one transaction from the same sender and
        // end up skipping validation.
        let executable = transaction
            .executable_ref()
            .map_err(|_| deprecated_module_bundle!())?;
        let extra_config = transaction.extra_config();
        self.run_prologue_with_payload(
            session,
            module_storage,
            &serialized_signers,
            executable,
            extra_config,
            transaction_data,
            log_context,
            is_approved_gov_script,
            traversal_context,
        )?;
        Ok(serialized_signers)
    }

    // Called when the execution of the user transaction fails, in order to discard the
    // transaction, or clean up the failed state.
    fn on_user_transaction_execution_failure(
        &self,
        prologue_session_change_set: SystemSessionChangeSet,
        err: VMStatus,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        serialized_signers: &SerializedSigners,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        gas_meter: &mut impl AptosGasMeter,
        change_set_configs: &ChangeSetConfigs,
        traversal_context: &mut TraversalContext,
    ) -> (VMStatus, VMOutput) {
        self.failed_transaction_cleanup(
            prologue_session_change_set,
            err,
            gas_meter,
            txn_data,
            resolver,
            module_storage,
            serialized_signers,
            log_context,
            change_set_configs,
            traversal_context,
        )
    }

    fn execute_user_transaction_impl(
        &self,
        resolver: &impl AptosMoveResolver,
        code_storage: &impl AptosCodeStorage,
        txn: &SignedTransaction,
        txn_data: TransactionMetadata,
        is_approved_gov_script: bool,
        log_context: &AdapterLogSchema,
        gas_meter: &mut impl AptosGasMeter,
    ) -> (VMStatus, VMOutput) {
        let _timer = VM_TIMER.timer_with_label("AptosVM::execute_user_transaction_impl");

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        // Revalidate the transaction.
        let mut prologue_session = PrologueSession::new(self, &txn_data, resolver);
        let initial_gas = gas_meter.balance();
        let serialized_signers = unwrap_or_discard!(prologue_session.execute(|session| {
            self.validate_signed_transaction(
                session,
                code_storage,
                txn,
                &txn_data,
                log_context,
                is_approved_gov_script,
                &mut traversal_context,
                gas_meter,
            )
        }));

        if self.features().is_account_abstraction_enabled()
            || self.features().is_derivable_account_abstraction_enabled()
        {
            let max_aa_gas = unwrap_or_discard!(self.gas_params(log_context))
                .vm
                .txn
                .max_aa_gas;
            if max_aa_gas < txn_data.max_gas_amount() {
                // Reset initial gas after validation with max_aa_gas.
                unwrap_or_discard!(gas_meter
                    .inject_balance(txn_data.max_gas_amount().checked_sub(max_aa_gas).unwrap()));
            }
        } else {
            assert_eq!(initial_gas, gas_meter.balance());
        }

        let storage_gas_params = unwrap_or_discard!(self.storage_gas_params(log_context));
        let change_set_configs = &storage_gas_params.change_set_configs;
        let (prologue_change_set, mut user_session) = unwrap_or_discard!(prologue_session
            .into_user_session(self, &txn_data, resolver, change_set_configs, code_storage,));

        let should_create_account_resource_timer =
            VM_TIMER.timer_with_label("AptosVM::create_account_resource_lazily");
        let should_create_account_resource = unwrap_or_discard!(should_create_account_resource(
            &txn_data,
            self.features(),
            resolver,
            code_storage
        ));
        if should_create_account_resource {
            unwrap_or_discard!(
                user_session.execute(|session| create_account_if_does_not_exist(
                    session,
                    code_storage,
                    gas_meter,
                    txn.sender(),
                    &mut traversal_context,
                ))
            );
        }
        drop(should_create_account_resource_timer);

        let payload_timer =
            VM_TIMER.timer_with_label("AptosVM::execute_user_transaction_impl [payload]");

        // `validate_signed_transaction` function already discards the transactions with `TransactionPayloadInner` type payload if the
        // corresponding feature flag (`TransactionPayloadV2`) is disabled. Therefore, we don't need to check the feature flag here again.
        let executable = match txn.executable_ref() {
            Ok(executable) => executable,
            Err(_) => return unwrap_or_discard!(Err(deprecated_module_bundle!())),
        };
        let multisig_address = txn.multisig_address();
        let result = if let Some(multisig_address) = multisig_address {
            self.execute_multisig_transaction(
                resolver,
                code_storage,
                user_session,
                &serialized_signers,
                &prologue_change_set,
                gas_meter,
                &mut traversal_context,
                &txn_data,
                executable,
                multisig_address,
                log_context,
                change_set_configs,
            )
        } else {
            self.execute_script_or_entry_function(
                resolver,
                code_storage,
                user_session,
                &serialized_signers,
                gas_meter,
                &mut traversal_context,
                &txn_data,
                executable,
                log_context,
                change_set_configs,
            )
        };
        drop(payload_timer);

        let gas_usage = txn_data
            .max_gas_amount()
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount set");
        TXN_GAS_USAGE.observe(u64::from(gas_usage) as f64);

        let (vm_status, output) = result.unwrap_or_else(|err| {
            self.on_user_transaction_execution_failure(
                prologue_change_set,
                err,
                resolver,
                code_storage,
                &serialized_signers,
                &txn_data,
                log_context,
                gas_meter,
                change_set_configs,
                &mut traversal_context,
            )
        });
        (vm_status, output)
    }

    /// Main entrypoint for executing a user transaction that also allows the customization of the
    /// gas meter to be used.
    pub fn execute_user_transaction_with_custom_gas_meter<'a, C, G, F>(
        &self,
        resolver: &'a impl AptosMoveResolver,
        code_storage: &'a C,
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
        make_gas_meter: F,
    ) -> Result<(VMStatus, VMOutput, G), VMStatus>
    where
        C: AptosCodeStorage + BlockSynchronizationKillSwitch,
        G: AptosGasMeter,
        F: FnOnce(u64, VMGasParameters, StorageGasParameters, bool, Gas, &'a C) -> G,
    {
        let txn_metadata = TransactionMetadata::new(txn);

        let is_approved_gov_script = is_approved_gov_script(resolver, txn, &txn_metadata);

        let vm_params = self.gas_params(log_context)?.vm.clone();

        let initial_balance = if self.features().is_account_abstraction_enabled()
            || self.features().is_derivable_account_abstraction_enabled()
        {
            vm_params.txn.max_aa_gas.min(txn.max_gas_amount().into())
        } else {
            txn.max_gas_amount().into()
        };

        let mut gas_meter = make_gas_meter(
            self.gas_feature_version(),
            vm_params,
            self.storage_gas_params(log_context)?.clone(),
            is_approved_gov_script,
            initial_balance,
            code_storage,
        );

        let (status, output) = self.execute_user_transaction_impl(
            resolver,
            code_storage,
            txn,
            txn_metadata,
            is_approved_gov_script,
            log_context,
            &mut gas_meter,
        );

        Ok((status, output, gas_meter))
    }

    /// Alternative entrypoint for user transaction execution that allows customization based on
    /// the production gas meter.
    ///
    /// This can be useful for off-chain applications that wants to perform additional
    /// measurements or analysis while preserving the production gas behavior.
    pub fn execute_user_transaction_with_modified_gas_meter<'a, G, F>(
        &self,
        resolver: &'a impl AptosMoveResolver,
        code_storage: &'a (impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
        modify_gas_meter: F,
    ) -> Result<(VMStatus, VMOutput, G), VMStatus>
    where
        F: FnOnce(ProdGasMeter<'a, NoopBlockSynchronizationKillSwitch>) -> G,
        G: AptosGasMeter,
    {
        self.execute_user_transaction_with_custom_gas_meter(
            resolver,
            code_storage,
            txn,
            log_context,
            |gas_feature_version,
             vm_gas_params,
             storage_gas_params,
             is_approved_gov_script,
             meter_balance,
             _maybe_block_synchronization_kill_switch| {
                modify_gas_meter(make_prod_gas_meter(
                    gas_feature_version,
                    vm_gas_params,
                    storage_gas_params,
                    is_approved_gov_script,
                    meter_balance,
                    &NoopBlockSynchronizationKillSwitch {},
                ))
            },
        )
    }

    /// Executes a user transaction using the production gas meter.
    pub fn execute_user_transaction(
        &self,
        resolver: &impl AptosMoveResolver,
        code_storage: &(impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, VMOutput) {
        match self.execute_user_transaction_with_custom_gas_meter(
            resolver,
            code_storage,
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
        code_storage: &impl AptosCodeStorage,
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
                    None => vec![serialized_signer(execute_as)],
                    Some(sender) => vec![serialized_signer(&sender), serialized_signer(execute_as)],
                };

                let traversal_storage = TraversalStorage::new();
                let mut traversal_context = TraversalContext::new(&traversal_storage);

                self.validate_and_execute_script(
                    &mut tmp_session,
                    &SerializedSigners::new(senders, None),
                    code_storage,
                    &mut UnmeteredGasMeter,
                    &mut traversal_context,
                    script,
                )?;

                let change_set_configs =
                    ChangeSetConfigs::unlimited_at_gas_feature_version(self.gas_feature_version());
                let change_set = tmp_session.finish(&change_set_configs, code_storage)?;

                // While scripts should be able to publish modules, this should be done through
                // native context, and so the module write set must always be empty.
                Ok((change_set, ModuleWriteSet::empty()))
            },
        }
    }

    fn read_change_set(
        &self,
        executor_view: &dyn ExecutorView,
        resource_group_view: &dyn ResourceGroupView,
        module_storage: &impl AptosModuleStorage,
        change_set: &VMChangeSet,
        module_write_set: &ModuleWriteSet,
    ) -> PartialVMResult<()> {
        assert!(
            change_set.aggregator_v1_write_set().is_empty(),
            "Waypoint change set should not have any aggregator writes."
        );

        // All Move executions satisfy the read-before-write property. Thus, we need to read each
        // access path that the write set is going to update.
        for write in module_write_set.writes().values() {
            // It is sufficient to simply get the size in order to enforce read-before-write.
            module_storage
                .fetch_module_size_in_bytes(write.module_address(), write.module_name())
                .map_err(|e| e.to_partial())?;
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
        let has_new_epoch_event = events.iter().any(|(e, _)| e.is_new_epoch_event());
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
        code_storage: &impl AptosCodeStorage,
        write_set_payload: WriteSetPayload,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        // TODO: user specified genesis id to distinguish different genesis write sets
        let genesis_id = HashValue::zero();
        let (change_set, module_write_set) = self.execute_write_set(
            resolver,
            code_storage,
            &write_set_payload,
            Some(account_config::reserved_vm_address()),
            SessionId::genesis(genesis_id),
        )?;

        Self::validate_waypoint_change_set(change_set.events(), log_context)?;
        self.read_change_set(
            resolver.as_executor_view(),
            resolver.as_resource_group_view(),
            code_storage,
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
        );
        Ok((VMStatus::Executed, output))
    }

    fn process_block_prologue(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
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
                module_storage,
            )
            .map(|_return_vals| ())
            .or_else(|e| {
                expect_only_successful_execution(e, BLOCK_PROLOGUE.as_str(), log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self.storage_gas_params(log_context)?.change_set_configs,
        )?;
        Ok((VMStatus::Executed, output))
    }

    fn process_block_prologue_ext(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
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
                module_storage,
            )
            .map(|_return_vals| ())
            .or_else(|e| {
                expect_only_successful_execution(e, BLOCK_PROLOGUE_EXT.as_str(), log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self.storage_gas_params(log_context)?.change_set_configs,
        )?;
        Ok((VMStatus::Executed, output))
    }

    fn process_block_epilogue(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        block_epilogue: BlockEpiloguePayload,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let (block_id, fee_distribution) = match block_epilogue {
            BlockEpiloguePayload::V0 { .. } => {
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                let output = VMOutput::empty_with_status(status);
                return Ok((VMStatus::Executed, output));
            },
            BlockEpiloguePayload::V1 {
                block_id,
                fee_distribution,
                ..
            } => (block_id, fee_distribution),
        };

        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, SessionId::block_epilogue(block_id), None);

        let (validator_indices, amounts) = match fee_distribution {
            FeeDistribution::V0 { amount } => amount
                .into_iter()
                .map(|(validator_index, amount)| {
                    (MoveValue::U64(validator_index), MoveValue::U64(amount))
                })
                .unzip(),
        };

        let args = vec![
            MoveValue::Signer(AccountAddress::ZERO), // Run as 0x0
            MoveValue::Vector(validator_indices),
            MoveValue::Vector(amounts),
        ];

        let storage = TraversalStorage::new();

        let output = match session
            .execute_function_bypass_visibility(
                &BLOCK_MODULE,
                BLOCK_EPILOGUE,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&storage),
                module_storage,
            )
            .map(|_return_vals| ())
            .or_else(|e| expect_only_successful_execution(e, BLOCK_EPILOGUE.as_str(), log_context))
        {
            Ok(_) => get_system_transaction_output(
                session,
                module_storage,
                &self.storage_gas_params(log_context)?.change_set_configs,
            )?,
            Err(e) => {
                error!(
                    "Unexpected error from BlockEpilogue txn: {e:?}, fallback to return success."
                );
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                VMOutput::empty_with_status(status)
            },
        };

        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        // TODO(HotState): generate an output according to the block end info in the
        //   transaction. (maybe resort to the move resolver, but for simplicity I would
        //   just include the full slot in both the transaction and the output).
        Ok((VMStatus::Executed, output))
    }

    pub fn execute_system_function_no_gas_meter(
        state_view: &impl StateView,
        module_id: &ModuleId,
        function_name: &Identifier,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        block_id: HashValue,
    ) -> Result<Vec<Vec<u8>>, VMStatus> {
        // Create VM instance with environment
        let env = AptosEnvironment::new(state_view);
        let vm = AptosVM::new(&env, state_view);

        // Create a new session
        let resolver = state_view.as_move_resolver();
        let mut session = vm.new_session(&resolver, SessionId::system_txn(block_id), None);

        // Set up gas meter and traversal context
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        // Get code storage adapter and ensure it's properly referenced
        let code_storage = state_view.as_aptos_code_storage(&env);
        let code_storage_ref = &code_storage;

        // Execute the function
        let result = session.execute_function_bypass_visibility(
            module_id,
            function_name,
            type_args,
            args,
            &mut gas_meter,
            &mut traversal_context,
            code_storage_ref,
        )?;

        Ok(result.return_values.into_iter().map(|v| v.0).collect())
    }

    pub fn execute_view_function(
        state_view: &impl StateView,
        module_id: ModuleId,
        func_name: Identifier,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
        max_gas_amount: u64,
    ) -> ViewFunctionOutput {
        let env = AptosEnvironment::new(state_view);
        let vm = AptosVM::new(&env, state_view);

        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let vm_gas_params = match vm.gas_params(&log_context) {
            Ok(gas_params) => gas_params.vm.clone(),
            Err(err) => {
                return ViewFunctionOutput::new(Err(anyhow::Error::msg(format!("{}", err))), 0)
            },
        };
        let storage_gas_params = match vm.storage_gas_params(&log_context) {
            Ok(gas_params) => gas_params.clone(),
            Err(err) => {
                return ViewFunctionOutput::new(Err(anyhow::Error::msg(format!("{}", err))), 0)
            },
        };

        let mut gas_meter = make_prod_gas_meter(
            vm.gas_feature_version(),
            vm_gas_params,
            storage_gas_params,
            /* is_approved_gov_script */ false,
            max_gas_amount.into(),
            &NoopBlockSynchronizationKillSwitch {},
        );

        let resolver = state_view.as_move_resolver();
        let module_storage = state_view.as_aptos_code_storage(&env);

        let mut session = vm.new_session(&resolver, SessionId::Void, None);
        let execution_result = Self::execute_view_function_in_vm(
            &mut session,
            &vm,
            module_id,
            func_name,
            type_args,
            arguments,
            &mut gas_meter,
            &module_storage,
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
        session: &mut SessionExt<impl AptosMoveResolver>,
        vm: &AptosVM,
        module_id: ModuleId,
        func_name: Identifier,
        ty_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
    ) -> anyhow::Result<Vec<Vec<u8>>> {
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let func = module_storage.load_function(&module_id, &func_name, &ty_args)?;
        let metadata = get_metadata(&func.owner_as_module()?.metadata);

        let arguments = view_function::validate_view_function(
            session,
            module_storage,
            arguments,
            func_name.as_ident_str(),
            &func,
            metadata.as_ref().map(Arc::as_ref),
            vm.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
        )?;

        Ok(session
            .execute_loaded_function(
                func,
                arguments,
                gas_meter,
                &mut traversal_context,
                module_storage,
            )
            .map_err(|err| anyhow!("Failed to execute function: {:?}", err))?
            .return_values
            .into_iter()
            .map(|(bytes, _ty)| bytes)
            .collect::<Vec<_>>())
    }

    fn run_prologue_with_payload(
        &self,
        session: &mut SessionExt<impl AptosMoveResolver>,
        module_storage: &impl ModuleStorage,
        serialized_signers: &SerializedSigners,
        executable: TransactionExecutableRef,
        extra_config: TransactionExtraConfig,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        is_approved_gov_script: bool,
        traversal_context: &mut TraversalContext,
    ) -> Result<(), VMStatus> {
        check_gas(
            self.gas_params(log_context)?,
            self.gas_feature_version(),
            session.resolver,
            module_storage,
            txn_data,
            self.features(),
            is_approved_gov_script,
            log_context,
        )?;
        if executable.is_empty() && !extra_config.is_multisig() {
            return Err(VMStatus::error(
                StatusCode::EMPTY_PAYLOAD_PROVIDED,
                Some("Empty provided for a non-multisig transaction".to_string()),
            ));
        }

        if executable.is_script() && extra_config.is_multisig() {
            return Err(VMStatus::error(
                StatusCode::FEATURE_UNDER_GATING,
                Some("Script payload not yet supported for multisig transactions".to_string()),
            ));
        }

        // Runs script prologue for all transaction types including multisig
        transaction_validation::run_script_prologue(
            session,
            module_storage,
            serialized_signers,
            txn_data,
            self.features(),
            log_context,
            traversal_context,
            self.is_simulation,
        )?;

        if let Some(multisig_address) = extra_config.multisig_address() {
            // Once "simulation_enhancement" is enabled, the simulation path also validates the
            // multisig transaction by running the multisig prologue.
            if !self.is_simulation
                || self
                    .features()
                    .is_transaction_simulation_enhancement_enabled()
            {
                transaction_validation::run_multisig_prologue(
                    session,
                    module_storage,
                    txn_data,
                    executable,
                    multisig_address,
                    self.features(),
                    log_context,
                    traversal_context,
                )?
            }
        }
        Ok(())
    }

    pub fn should_restart_execution(events: &[(ContractEvent, Option<MoveTypeLayout>)]) -> bool {
        events.iter().any(|(event, _)| event.is_new_epoch_event())
    }

    /// Executes a single transaction (including user transactions, block
    /// metadata and state checkpoint, etc.).
    /// *Precondition:* VM has to be instantiated in execution mode.
    pub fn execute_single_transaction(
        &self,
        txn: &SignatureVerifiedTransaction,
        resolver: &impl AptosMoveResolver,
        code_storage: &(impl AptosCodeStorage + BlockSynchronizationKillSwitch),
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
                let (vm_status, output) = self.process_block_prologue(
                    resolver,
                    code_storage,
                    block_metadata.clone(),
                    log_context,
                )?;
                (vm_status, output)
            },
            Transaction::BlockMetadataExt(block_metadata_ext) => {
                fail_point!("aptos_vm::execution::block_metadata_ext");
                let (vm_status, output) = self.process_block_prologue_ext(
                    resolver,
                    code_storage,
                    block_metadata_ext.clone(),
                    log_context,
                )?;
                (vm_status, output)
            },
            Transaction::GenesisTransaction(write_set_payload) => {
                let (vm_status, output) = self.process_waypoint_change_set(
                    resolver,
                    code_storage,
                    write_set_payload.clone(),
                    log_context,
                )?;
                (vm_status, output)
            },
            Transaction::UserTransaction(txn) => {
                fail_point!("aptos_vm::execution::user_transaction");
                let _timer = TXN_TOTAL_SECONDS.start_timer();
                let (vm_status, output) =
                    self.execute_user_transaction(resolver, code_storage, txn, log_context);

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
            Transaction::BlockEpilogue(block_epilogue) => self.process_block_epilogue(
                resolver,
                code_storage,
                block_epilogue.clone(),
                log_context,
            )?,
            Transaction::ValidatorTransaction(txn) => {
                let (vm_status, output) = self.process_validator_transaction(
                    resolver,
                    code_storage,
                    // TODO: Remove this clone operation
                    txn.clone(),
                    log_context,
                )?;
                (vm_status, output)
            },
            Transaction::ScheduledTransaction(txn) => {
                let traversal_storage = TraversalStorage::new();
                let mut traversal_context = TraversalContext::new(&traversal_storage);
                let (vm_status, output) = self.process_scheduled_transaction(
                    resolver,
                    txn.clone(),
                    &mut traversal_context,
                    code_storage,
                    log_context,
                )?;
                (vm_status, output)
            },
        })
    }

    pub(crate) fn process_scheduled_transaction(
        &self,
        resolver: &impl AptosMoveResolver,
        txn: ScheduledTransactionInfoWithKey,
        traversal_context: &mut TraversalContext,
        code_storage: &(impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let balance = txn.max_gas_amount;
        let storage_gas = self.storage_gas_params(log_context)?;
        let mut gas_meter = make_prod_gas_meter(
            self.gas_feature_version(),
            self.gas_params(log_context)?.vm.clone(),
            storage_gas.clone(),
            false,
            balance.into(),
            code_storage,
        );

        // no need of scheduled txn prologue for now.
        let args = vec![
            MoveValue::Signer(txn.sender_handle),
            txn.key.as_move_value(),
        ];

        /* todo: check if we indeed need this
        let user_txn_ctx = UserTransactionContext::new(
            txn.sender_handle,
            [].to_vec(),
            txn.sender_handle,
            txn.max_gas_amount,
            txn.gas_unit_price_charged,
            1, // todo: need to get this from somewhere
            None,
            None,
        );*/
        let mut session =
            self.new_session(resolver, SessionId::scheduled_txn(txn.key.hash()), None);
        let user_func_status = session.execute_function_bypass_visibility(
            &SCHEDULED_TRANSACTIONS_MODULE_INFO.module_id(),
            &SCHEDULED_TRANSACTIONS_MODULE_INFO.execute_user_function_wrapper_name,
            vec![],
            serialize_values(&args),
            &mut gas_meter,
            traversal_context,
            code_storage,
        );
        match user_func_status {
            Ok(_) => {},
            Err(err) => {
                // If the user function execution fails, we return the error status and an empty output.
                let error_vm_status = err.into_vm_status();
                let txn_status = TransactionStatus::from_vm_status(
                    error_vm_status.clone(),
                    self.features()
                        .is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION),
                );
                match txn_status {
                    TransactionStatus::Keep(_) => {
                        // In this case, we will run the epilogue and charge the gas used.
                        warn!(
                            "Scheduled txn user function execution failed: {:?}",
                            error_vm_status
                        );
                    },
                    TransactionStatus::Discard(status_code) => {
                        error!(
                            "Discarding scheduled txn; user function execution failed: {:?}",
                            error_vm_status
                        );
                        let discarded_output = discarded_output(status_code);
                        // todo: should we run_scheduled_txn_cleanup() here ?
                        //       otherwise, the scheduled transaction will remain in the queue and
                        //       will be retried in subsequent blocks.
                        return Ok((error_vm_status, discarded_output));
                    },
                    TransactionStatus::Retry => {
                        unreachable!("We can't retry scheduled transactions");
                    },
                }
            },
        };

        let fee_statement =
            Self::fee_statement_from_gas_meter(txn.max_gas_amount.into(), &gas_meter, 0);

        // Run epilogue but store result instead of propagating error
        match run_scheduled_txn_epilogue(
            &mut session,
            &txn,
            gas_meter.balance(),
            fee_statement,
            traversal_context,
            code_storage,
        ) {
            Ok(()) => {},
            Err(e) => {
                warn!(
                    "Scheduled transaction epilogue failed: {:?}, txn: {:?}; trying to just remove the txn from scheduled queue",
                    e, txn
                );
                let mut cleanup_session =
                    self.new_session(resolver, SessionId::scheduled_txn(txn.key.hash()), None);
                match run_scheduled_txn_cleanup(
                    &mut cleanup_session,
                    &txn,
                    traversal_context,
                    code_storage,
                ) {
                    Ok(_) => {},
                    Err(cleanup_err) => {
                        error!(
                            "Scheduled transaction cleanup failed after epilogue failure: {:?}",
                            cleanup_err
                        );
                    },
                }
            },
        };

        // Irrespective of whether the epilogue succeeded or failed, fee statement is included in
        // the output if user function was successfully executed
        let output = get_sched_txn_output(
            session,
            code_storage,
            &self.storage_gas_params(log_context)?.change_set_configs,
            fee_statement,
        )?;

        Ok((VMStatus::Executed, output))
    }
}

// TODO - move out from this file?

/// Production implementation of VMBlockExecutor.
///
/// Transaction execution: AptosVM
/// Executing conflicts: in the input order, via BlockSTM,
/// State: BlockSTM-provided MVHashMap-based view with caching
pub struct AptosVMBlockExecutor {
    /// Manages module cache and execution environment of this block executor. Users of executor
    /// must use manager's API to ensure the correct state of caches.
    module_cache_manager: AptosModuleCacheManager,
}

impl AptosVMBlockExecutor {
    /// Executes transactions with the specified [BlockExecutorConfig] and returns output for each
    /// one of them.
    pub fn execute_block_with_config(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &(impl StateView + Sync),
        config: BlockExecutorConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        fail_point!("aptos_vm_block_executor::execute_block_with_config", |_| {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let num_txns = txn_provider.num_txns();
        info!(
            log_context,
            "Executing block, transaction count: {}", num_txns
        );

        let result = AptosVMBlockExecutorWrapper::execute_block::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
            DefaultTxnProvider<SignatureVerifiedTransaction>,
        >(
            txn_provider,
            state_view,
            &self.module_cache_manager,
            config,
            transaction_slice_metadata,
            None,
        );
        if result.is_ok() {
            // Record the histogram count for transactions per block.
            BLOCK_TRANSACTION_COUNT.observe(num_txns as f64);
        }
        result
    }
}

impl VMBlockExecutor for AptosVMBlockExecutor {
    fn new() -> Self {
        Self {
            module_cache_manager: AptosModuleCacheManager::new(),
        }
    }

    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &(impl StateView + Sync),
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        let config = BlockExecutorConfig {
            local: BlockExecutorLocalConfig {
                concurrency_level: AptosVM::get_concurrency_level(),
                allow_fallback: true,
                discard_failed_blocks: AptosVM::get_discard_failed_blocks(),
                module_cache_config: BlockExecutorModuleCacheLocalConfig::default(),
            },
            onchain: onchain_config,
        };
        self.execute_block_with_config(txn_provider, state_view, config, transaction_slice_metadata)
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
        module_storage: &impl ModuleStorage,
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
            if let Ok(TransactionExecutableRef::Script(script)) =
                transaction.payload().executable_ref()
            {
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

        let vm_params = match self.gas_params(&log_context) {
            Ok(vm_params) => vm_params.vm.clone(),
            Err(err) => {
                return VMValidatorResult::new(Some(err.status_code()), 0);
            },
        };
        let storage_gas_params = match self.storage_gas_params(&log_context) {
            Ok(storage_params) => storage_params.clone(),
            Err(err) => {
                return VMValidatorResult::new(Some(err.status_code()), 0);
            },
        };

        let initial_balance = if self.features().is_account_abstraction_enabled()
            || self.features().is_derivable_account_abstraction_enabled()
        {
            vm_params.txn.max_aa_gas.min(txn_data.max_gas_amount())
        } else {
            txn_data.max_gas_amount()
        };

        let mut gas_meter = make_prod_gas_meter(
            self.gas_feature_version(),
            vm_params,
            storage_gas_params,
            is_approved_gov_script,
            initial_balance,
            &NoopBlockSynchronizationKillSwitch {},
        );
        let storage = TraversalStorage::new();

        // Increment the counter for transactions verified.
        let (counter_label, result) = match self.validate_signed_transaction(
            &mut session,
            module_storage,
            &txn,
            &txn_data,
            &log_context,
            is_approved_gov_script,
            &mut TraversalContext::new(&storage),
            &mut gas_meter,
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
pub struct AptosSimulationVM;

impl AptosSimulationVM {
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

        let env = AptosEnvironment::new(state_view);
        let mut vm = AptosVM::new(&env, state_view);
        vm.is_simulation = true;

        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let resolver = state_view.as_move_resolver();
        let code_storage = state_view.as_aptos_code_storage(&env);

        let (vm_status, vm_output) =
            vm.execute_user_transaction(&resolver, &code_storage, transaction, &log_context);
        let txn_output = vm_output
            .try_materialize_into_transaction_output(&resolver)
            .expect("Materializing aggregator V1 deltas should never fail");
        (vm_status, txn_output)
    }
}

fn create_account_if_does_not_exist(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    gas_meter: &mut impl GasMeter,
    account: AccountAddress,
    traversal_context: &mut TraversalContext,
) -> VMResult<()> {
    session.execute_function_bypass_visibility(
        &ACCOUNT_MODULE,
        CREATE_ACCOUNT_IF_DOES_NOT_EXIST,
        vec![],
        serialize_values(&vec![MoveValue::Address(account)]),
        gas_meter,
        traversal_context,
        module_storage,
    )?;
    Ok(())
}

fn dispatchable_authenticate(
    session: &mut SessionExt<impl AptosMoveResolver>,
    gas_meter: &mut impl GasMeter,
    account: AccountAddress,
    function_info: FunctionInfo,
    auth_data: &AbstractionAuthData,
    traversal_context: &mut TraversalContext,
    module_storage: &impl ModuleStorage,
) -> VMResult<Vec<u8>> {
    let auth_data = bcs::to_bytes(auth_data).expect("from rust succeeds");
    let mut params = serialize_values(&vec![
        MoveValue::Signer(account),
        function_info.as_move_value(),
    ]);
    params.push(auth_data);
    session
        .execute_function_bypass_visibility(
            &ACCOUNT_ABSTRACTION_MODULE,
            AUTHENTICATE,
            vec![],
            params,
            gas_meter,
            traversal_context,
            module_storage,
        )
        .map(|mut return_vals| {
            assert!(
                return_vals.mutable_reference_outputs.is_empty()
                    && return_vals.return_values.len() == 1,
                "Abstraction authentication function must only have 1 return value"
            );
            let (signer_data, signer_layout) = return_vals.return_values.pop().expect("Must exist");
            assert_eq!(
                signer_layout,
                MoveTypeLayout::Signer,
                "Abstraction authentication function returned non-signer."
            );
            signer_data
        })
}

/// Determines if an account should be automatically created as part of a sponsored transaction.
/// This function checks several conditions that must all be met:
///
/// 1. Feature flag check: Either DEFAULT_ACCOUNT_RESOURCE or SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION is enabled
/// 2. For SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION: Transaction has a fee payer (for sponsored transactions)
/// 3. Transaction sequence number is 0 (indicating a new account)
/// 4. Account resource does not already exist for the sender address
///
/// This is used to support automatic account creation for sponsored transactions or after enabling default account
/// resource feature, allowing new accounts to be created without requiring an explicit account creation transaction.
pub(crate) fn should_create_account_resource(
    txn_data: &TransactionMetadata,
    features: &Features,
    resolver: &impl AptosMoveResolver,
    module_storage: &impl ModuleStorage,
) -> VMResult<bool> {
    if (features.is_enabled(FeatureFlag::DEFAULT_ACCOUNT_RESOURCE)
        || (features.is_enabled(FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION)
            && txn_data.fee_payer.is_some()))
        && txn_data.replay_protector == ReplayProtector::SequenceNumber(0)
    {
        let account_tag = AccountResource::struct_tag();
        let metadata = module_storage
            .fetch_existing_module_metadata(&account_tag.address, &account_tag.module)?;
        let (maybe_bytes, _) = resolver
            .get_resource_bytes_with_metadata_and_layout(
                &txn_data.sender(),
                &account_tag,
                &metadata,
                None,
            )
            .map_err(|e| e.finish(Location::Undefined))?;
        return Ok(maybe_bytes.is_none());
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::{move_vm_ext::MoveVmExt, AptosVM};
    use aptos_types::{
        account_address::AccountAddress,
        account_config::{NEW_EPOCH_EVENT_MOVE_TYPE_TAG, NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG},
        contract_event::ContractEvent,
        event::EventKey,
    };

    #[test]
    fn vm_thread_safe() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<AptosVM>();
        assert_sync::<AptosVM>();
        assert_send::<MoveVmExt>();
        assert_sync::<MoveVmExt>();
    }

    #[test]
    fn should_restart_execution_on_new_epoch() {
        let new_epoch_event = ContractEvent::new_v1(
            EventKey::new(0, AccountAddress::ONE),
            0,
            NEW_EPOCH_EVENT_MOVE_TYPE_TAG.clone(),
            vec![],
        )
        .unwrap();
        let new_epoch_event_v2 =
            ContractEvent::new_v2(NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.clone(), vec![]).unwrap();
        assert!(AptosVM::should_restart_execution(&[(
            new_epoch_event,
            None
        )]));
        assert!(AptosVM::should_restart_execution(&[(
            new_epoch_event_v2,
            None
        )]));
    }
}
