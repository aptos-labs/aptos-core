// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::is_approved_gov_script,
    counters::TXN_TOTAL_SECONDS,
    gas::{make_prod_gas_meter, ProdGasMeter},
    move_vm_ext::{session::make_aptos_extensions, AptosMoveResolver, SessionId},
    system_module_names::{BLOCK_EPILOGUE, BLOCK_MODULE, BLOCK_PROLOGUE, BLOCK_PROLOGUE_EXT},
    transaction_metadata::TransactionMetadata,
    v2::{
        data_cache::TransactionDataCache,
        session::{Session, UserTransactionSession},
    },
};
use aptos_gas_algebra::Gas;
use aptos_gas_meter::AptosGasMeter;
use aptos_gas_schedule::VMGasParameters;
use aptos_logger::error;
use aptos_types::{
    block_metadata_ext::BlockMetadataExt,
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    state_store::state_value::StateValueMetadata,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction,
        user_transaction_context::UserTransactionContext, BlockEpiloguePayload, ExecutionStatus,
        FeeDistribution, SignedTransaction, Transaction, TransactionStatus,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::{
        code_storage::AptosCodeStorage, module_storage::AptosModuleStorage,
    },
    output::VMOutput,
    resolver::{BlockSynchronizationKillSwitch, NoopBlockSynchronizationKillSwitch},
    storage::StorageGasParameters,
};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    value::MoveValue,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};

pub struct AptosVMv2 {
    pub(crate) is_simulation: bool,
    pub(crate) environment: AptosEnvironment,
    #[allow(dead_code)]
    pvk: Option<PreparedVerifyingKey<Bn254>>,
}

impl AptosVMv2 {
    /// Creates a new VM instance based on the runtime environment. The VM can then be used by the
    /// block executor to create multiple tasks sharing the same execution configurations extracted
    /// from the environment.
    ///
    /// New VM by default is created for non-simulation execution and with unset verification key
    /// for keyless validation. The verification key for keyless will be set lazily in case it is
    /// needed.
    pub fn new(environment: &AptosEnvironment) -> Self {
        Self {
            is_simulation: false,
            environment: environment.clone(),
            pvk: None,
        }
    }

    #[allow(dead_code)]
    pub fn execute_single_transaction(
        &self,
        txn: &SignatureVerifiedTransaction,
        data_view: &impl AptosMoveResolver,
        code_view: &(impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        log_context: &AdapterLogSchema,
    ) -> Result<VMOutput, VMStatus> {
        assert!(!self.is_simulation, "VM has to be created for execution");

        if !txn.is_valid() {
            return Ok(VMOutput::discarded(StatusCode::INVALID_SIGNATURE));
        }
        let txn = txn.expect_valid();

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        match txn {
            Transaction::BlockMetadata(block_metadata) => self.execute_block_prologue(
                data_view,
                code_view,
                log_context,
                &mut traversal_context,
                BlockMetadataExt::V0(block_metadata.clone()),
            ),
            Transaction::BlockMetadataExt(block_metadata) => {
                assert!(matches!(block_metadata, BlockMetadataExt::V1(_)));
                self.execute_block_prologue(
                    data_view,
                    code_view,
                    log_context,
                    &mut traversal_context,
                    block_metadata.clone(),
                )
            },
            Transaction::BlockEpilogue(block_epilogue_payload) => self.execute_block_epilogue(
                data_view,
                code_view,
                log_context,
                &mut traversal_context,
                block_epilogue_payload,
            ),
            Transaction::GenesisTransaction(_) => {
                // TODO: support this.
                unimplemented!()
            },
            Transaction::StateCheckpoint(_) => {
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                Ok(VMOutput::empty_with_status(status))
            },
            Transaction::ValidatorTransaction(_) => {
                // TODO: support this.
                unimplemented!()
            },
            Transaction::UserTransaction(txn) => {
                let _timer = TXN_TOTAL_SECONDS.start_timer();

                self.execute_user_transaction_with_custom_gas_meter(
                    data_view,
                    code_view,
                    log_context,
                    &mut traversal_context,
                    txn,
                    make_prod_gas_meter,
                )
                .map(|(output, _)| output)
            },
        }
    }

    /// Alternative entrypoint for user transaction execution that allows customization based on
    /// the production gas meter.
    ///
    /// This can be useful for off-chain applications that wants to perform additional
    /// measurements or analysis while preserving the production gas behavior.
    #[allow(dead_code)]
    pub fn execute_user_transaction_with_modified_gas_meter<'a, G, F>(
        &'a self,
        data_view: &'a impl AptosMoveResolver,
        code_view: &'a (impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        log_context: &AdapterLogSchema,
        txn: &'a SignedTransaction,
        modify_gas_meter: F,
    ) -> Result<(VMOutput, G), VMStatus>
    where
        F: FnOnce(ProdGasMeter<'a, NoopBlockSynchronizationKillSwitch>) -> G,
        G: AptosGasMeter,
    {
        let make_gas_meter = |gas_feature_version,
                              vm_gas_params,
                              storage_gas_params,
                              is_approved_gov_script,
                              meter_balance,
                              _| {
            modify_gas_meter(make_prod_gas_meter(
                gas_feature_version,
                vm_gas_params,
                storage_gas_params,
                is_approved_gov_script,
                meter_balance,
                &NoopBlockSynchronizationKillSwitch {},
            ))
        };

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        self.execute_user_transaction_with_custom_gas_meter(
            data_view,
            code_view,
            log_context,
            &mut traversal_context,
            txn,
            make_gas_meter,
        )
    }
}

impl AptosVMv2 {
    fn new_session<'a, DataView, CodeView>(
        &'a self,
        data_view: &'a DataView,
        code_view: &'a CodeView,
        log_context: &'a AdapterLogSchema,
        traversal_context: &'a mut TraversalContext<'a>,
        session_id: SessionId,
        user_transaction_context: Option<UserTransactionContext>,
    ) -> Result<Session<'a, DataView, CodeView>, VMStatus>
    where
        DataView: AptosMoveResolver,
        CodeView: AptosModuleStorage,
    {
        let gas_params = self.environment.gas_params().as_ref().map_err(|err| {
            let msg = format!(
                "VM failed to create a session, gas parameters not found: {}",
                err
            );
            error!("[aptos-vm] {}", msg);
            VMStatus::error(StatusCode::VM_STARTUP_FAILURE, Some(msg))
        })?;
        let storage_gas_params = self
            .environment
            .storage_gas_params()
            .as_ref()
            .map_err(|err| {
                let msg = format!(
                    "VM failed to create a session, storage gas parameters not found: {}",
                    err
                );
                error!("[aptos-vm] {}", msg);
                VMStatus::error(StatusCode::VM_STARTUP_FAILURE, Some(msg))
            })?;

        let extensions = make_aptos_extensions(
            data_view,
            self.environment.chain_id(),
            self.environment.vm_config(),
            session_id,
            user_transaction_context,
        );

        let mut new_slot_metadata: Option<StateValueMetadata> = None;
        if let Some(current_time) = CurrentTimeMicroseconds::fetch_config(data_view) {
            // The deposit on the metadata is a placeholder (0), it will be updated later when
            // storage fee is charged.
            new_slot_metadata = Some(StateValueMetadata::placeholder(&current_time));
        }

        Ok(Session {
            data_view,
            code_view,
            log_context,
            traversal_context,
            data_cache: TransactionDataCache::empty(),
            extensions,
            features: self.environment.features(),
            chain_id: self.environment.chain_id(),
            gas_feature_version: self.environment.gas_feature_version(),
            vm_config: self.environment.vm_config(),
            new_slot_metadata,
            gas_params,
            storage_gas_params,
        })
    }

    pub(crate) fn new_system_session<'a, DataView, CodeView>(
        &'a self,
        data_view: &'a DataView,
        code_view: &'a CodeView,
        log_context: &'a AdapterLogSchema,
        traversal_context: &'a mut TraversalContext<'a>,
        session_id: SessionId,
    ) -> Result<Session<'a, DataView, CodeView>, VMStatus>
    where
        DataView: AptosMoveResolver,
        CodeView: AptosModuleStorage,
    {
        self.new_session(
            data_view,
            code_view,
            log_context,
            traversal_context,
            session_id,
            None,
        )
    }

    pub(crate) fn new_user_transaction_session<'a, DataView, CodeView>(
        &'a self,
        data_view: &'a DataView,
        code_view: &'a CodeView,
        log_context: &'a AdapterLogSchema,
        traversal_context: &'a mut TraversalContext<'a>,
        txn: &'a SignedTransaction,
    ) -> Result<UserTransactionSession<'a, DataView, CodeView>, VMStatus>
    where
        DataView: AptosMoveResolver,
        CodeView: AptosModuleStorage,
    {
        let txn_metadata = TransactionMetadata::new(txn);
        let is_approved_gov_script = is_approved_gov_script(data_view, txn, &txn_metadata);

        let session_id = SessionId::prologue_meta(&txn_metadata);
        let user_transaction_context = txn_metadata.as_user_transaction_context();
        let session = self.new_session(
            data_view,
            code_view,
            log_context,
            traversal_context,
            session_id,
            Some(user_transaction_context),
        )?;

        let executable = txn.executable_ref().map_err(|err| {
            VMStatus::error(
                StatusCode::FEATURE_UNDER_GATING,
                Some(format!("Unable to get executable for transaction: {err}")),
            )
        })?;

        Ok(UserTransactionSession {
            session,
            txn,
            txn_metadata,
            txn_extra_config: txn.extra_config(),
            executable,
            is_approved_gov_script,
            is_simulation: self.is_simulation,
            storage_refund: 0.into(),
            serialized_signers: None,
            module_write_set: None,
        })
    }

    fn execute_block_prologue<'a>(
        &'a self,
        data_view: &'a impl AptosMoveResolver,
        code_view: &'a (impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        log_context: &'a AdapterLogSchema,
        traversal_context: &'a mut TraversalContext<'a>,
        block_metadata: BlockMetadataExt,
    ) -> Result<VMOutput, VMStatus> {
        let session_id = block_metadata_session_id(&block_metadata);
        let session = self.new_system_session(
            data_view,
            code_view,
            log_context,
            traversal_context,
            session_id,
        )?;

        let prologue_name = block_metadata_prologue_name(&block_metadata);
        let args = block_metadata.get_prologue_move_args();
        session.execute_unmetered_system_function_once(&BLOCK_MODULE, prologue_name, args)
    }

    fn execute_block_epilogue<'a>(
        &'a self,
        data_view: &'a impl AptosMoveResolver,
        code_view: &'a (impl AptosCodeStorage + BlockSynchronizationKillSwitch),
        log_context: &'a AdapterLogSchema,
        traversal_context: &'a mut TraversalContext<'a>,
        block_epilogue_payload: &BlockEpiloguePayload,
    ) -> Result<VMOutput, VMStatus> {
        let (block_id, fee_distribution) = match block_epilogue_payload {
            BlockEpiloguePayload::V0 { .. } => {
                let status = TransactionStatus::Keep(ExecutionStatus::Success);
                return Ok(VMOutput::empty_with_status(status));
            },
            BlockEpiloguePayload::V1 {
                block_id,
                fee_distribution,
                ..
            } => (block_id, fee_distribution),
        };

        let session_id = SessionId::block_epilogue(*block_id);
        let session = self.new_system_session(
            data_view,
            code_view,
            log_context,
            traversal_context,
            session_id,
        )?;

        let (validator_indices, amounts) = match fee_distribution {
            FeeDistribution::V0 { amount } => amount
                .iter()
                .map(|(validator_index, amount)| {
                    (MoveValue::U64(*validator_index), MoveValue::U64(*amount))
                })
                .unzip(),
        };
        let args = vec![
            MoveValue::Signer(AccountAddress::ZERO), // Run as 0x0
            MoveValue::Vector(validator_indices),
            MoveValue::Vector(amounts),
        ];

        // TODO:
        //   Cross-check with block epilogue in AptosVM: there we do some extra logging and
        //   fallback to return success (why?). Also, check why hot state is important there.
        session.execute_unmetered_system_function_once(&BLOCK_MODULE, BLOCK_EPILOGUE, args)
    }

    pub(crate) fn execute_user_transaction_with_custom_gas_meter<'a, DataView, CodeView, G, F>(
        &'a self,
        data_view: &'a DataView,
        code_view: &'a CodeView,
        log_context: &'a AdapterLogSchema,
        traversal_context: &'a mut TraversalContext<'a>,
        txn: &'a SignedTransaction,
        make_gas_meter: F,
    ) -> Result<(VMOutput, G), VMStatus>
    where
        DataView: AptosMoveResolver,
        CodeView: AptosCodeStorage + BlockSynchronizationKillSwitch,
        G: AptosGasMeter,
        F: FnOnce(u64, VMGasParameters, StorageGasParameters, bool, Gas, &'a CodeView) -> G,
    {
        let mut session = self.new_user_transaction_session(
            data_view,
            code_view,
            log_context,
            traversal_context,
            txn,
        )?;
        let mut gas_meter = session.build_gas_meter(make_gas_meter);
        let output = session.execute_user_transaction(&mut gas_meter);
        Ok((output, gas_meter))
    }
}

fn block_metadata_session_id(block_metadata: &BlockMetadataExt) -> SessionId {
    match block_metadata {
        BlockMetadataExt::V0(block_metadata) => SessionId::block_meta(block_metadata),
        BlockMetadataExt::V1(_) => SessionId::block_meta_ext(block_metadata),
    }
}

fn block_metadata_prologue_name(block_metadata: &BlockMetadataExt) -> &'static IdentStr {
    match block_metadata {
        BlockMetadataExt::V0(_) => BLOCK_PROLOGUE,
        BlockMetadataExt::V1(_) => BLOCK_PROLOGUE_EXT,
    }
}
