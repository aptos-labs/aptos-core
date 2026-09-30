// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    errors::{DiscardReason, ExecutorCreationError, NoEffectsReason},
    outcome::TxnOutcome,
    symbols::FrameworkSymbols,
};
use aptos_types::{
    state_store::state_storage_usage::StateStorageUsage,
    transaction::{AuxiliaryInfo, Transaction},
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_core::{storage::module_provider::ModuleProvider, ResourceProvider};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::ProductionNativeRegistry;

/// The Aptos transaction executor on MonoMove (the legacy AptosVM's role).
///
/// It is designed to be a transient object borrowing various contexts from the block coordinator,
/// for transaction execution (most likely just a single one). The execution
/// entry points live with their transaction kinds: `user_txn` and
/// `system_txns`.
pub struct AptosTransactionExecutor<'a> {
    pub(crate) guard: &'a ExecutionGuard<'a>,
    /// The framework symbols preinstalled into `guard`'s context.
    pub(crate) symbols: &'a FrameworkSymbols,
    /// All native functions available for this executor.
    pub(crate) natives: &'a ProductionNativeRegistry,
    pub(crate) module_provider: &'a dyn ModuleProvider,
    pub(crate) data_provider: &'a dyn ResourceProvider,
    /// The on-chain configs (features, gas schedule) at the current state.
    //
    // TODO(cleanup): reusing the legacy VM's environment type is transitional;
    // revisit once the executor grows its own config surface.
    pub(crate) env: &'a AptosEnvironment,
    /// State storage usage at the current epoch's beginning.
    pub(crate) usage: StateStorageUsage,
    /// If set, the payload runs unmetered and the epilogue charges a zero
    /// fee.
    pub(crate) unmetered: bool,
}

impl<'a> AptosTransactionExecutor<'a> {
    /// Constructs a new `AptosTransactionExecutor`. The borrowed contexts are
    /// built and owned by the block coordinator, which reuses them across the
    /// executors it creates; `guard`'s context must have been prepared with
    /// `preinstall`.
    pub fn new(
        guard: &'a ExecutionGuard<'a>,
        natives: &'a ProductionNativeRegistry,
        module_provider: &'a dyn ModuleProvider,
        data_provider: &'a dyn ResourceProvider,
        env: &'a AptosEnvironment,
        usage: StateStorageUsage,
    ) -> Result<Self, ExecutorCreationError> {
        let symbols = guard
            .preinstalled::<FrameworkSymbols>()
            .ok_or(ExecutorCreationError::ContextNotPrepared)?;
        Ok(Self {
            guard,
            symbols,
            natives,
            module_provider,
            data_provider,
            env,
            usage,
            unmetered: false,
        })
    }

    /// Runs the payload unmetered, making the epilogue charge a zero fee.
    /// For replay- and simulation-style callers that need gas-independent
    /// outputs.
    pub fn without_metering(mut self) -> Self {
        self.unmetered = true;
        self
    }

    /// Preinstalls into `ctx` what every executor running in it reads. Call
    /// once per context before execution, and again after its arenas are
    /// reset.
    pub fn preinstall(ctx: &mut GlobalContext) {
        let symbols = FrameworkSymbols::new(
            &ctx.try_execution_context(0)
                .expect("no execution is in progress"),
        );
        ctx.preinstall(symbols);
    }

    /// Executes any transaction, dispatching to its kind's entry point.
    //
    // TODO(completeness): genesis and validator transactions; some may stay
    // the block coordinator's job.
    pub fn execute_transaction(&self, txn: &Transaction, aux_info: &AuxiliaryInfo) -> TxnOutcome {
        match txn {
            Transaction::UserTransaction(txn) => self.execute_user_transaction(txn, aux_info),
            Transaction::BlockMetadata(block_metadata) => {
                self.execute_block_metadata_transaction(block_metadata, aux_info)
            },
            Transaction::BlockMetadataExt(block_metadata_ext) => {
                self.execute_block_metadata_ext_transaction(block_metadata_ext, aux_info)
            },
            Transaction::BlockEpilogue(block_epilogue) => {
                self.execute_block_epilogue_transaction(block_epilogue)
            },
            Transaction::GenesisTransaction(_) => TxnOutcome::Discarded {
                reason: DiscardReason::Unsupported("genesis transactions"),
                effects: None,
            },
            // A state checkpoint runs nothing on-chain; it only marks a point
            // for the executor to checkpoint the state tree at.
            Transaction::StateCheckpoint(_) => {
                TxnOutcome::ExecutedNoEffects(NoEffectsReason::NothingToExecute)
            },
            Transaction::ValidatorTransaction(_) => TxnOutcome::Discarded {
                reason: DiscardReason::Unsupported("validator transactions"),
                effects: None,
            },
        }
    }
}
