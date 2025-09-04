// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::native_config::NATIVE_EXECUTOR_POOL;
use velor_block_executor::{
    counters::BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK, txn_provider::default::DefaultTxnProvider,
};
use velor_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::StateView,
    transaction::{
        block_epilogue::BlockEndInfo, signature_verified_transaction::SignatureVerifiedTransaction,
        AuxiliaryInfo, BlockOutput, Transaction, TransactionOutput,
    },
    vm_status::VMStatus,
};
use velor_vm::{VelorVM, VMBlockExecutor};
use velor_vm_environment::environment::VelorEnvironment;
use velor_vm_logging::log_schema::AdapterLogSchema;
use velor_vm_types::module_and_script_storage::AsVelorCodeStorage;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

pub struct VelorVMParallelUncoordinatedBlockExecutor;

impl VMBlockExecutor for VelorVMParallelUncoordinatedBlockExecutor {
    fn new() -> Self {
        Self
    }

    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
        state_view: &(impl StateView + Sync),
        _onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<SignatureVerifiedTransaction, TransactionOutput>, VMStatus> {
        let _timer = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.start_timer();

        // let features = Features::fetch_config(&state_view).unwrap_or_default();

        let env = VelorEnvironment::new(state_view);
        let vm = VelorVM::new(&env, state_view);

        let block_epilogue_txn = Transaction::block_epilogue_v0(
            transaction_slice_metadata
                .append_state_checkpoint_to_block()
                .unwrap(),
            BlockEndInfo::new_empty(),
        );

        let transaction_outputs = NATIVE_EXECUTOR_POOL.install(|| {
            txn_provider
                .get_txns()
                .par_iter()
                .chain(vec![block_epilogue_txn.clone().into()].par_iter())
                .enumerate()
                .map(|(txn_idx, txn)| {
                    let log_context = AdapterLogSchema::new(state_view.id(), txn_idx);
                    let code_storage = state_view.as_velor_code_storage(&env);

                    vm.execute_single_transaction(
                        txn,
                        &vm.as_move_resolver(state_view),
                        &code_storage,
                        &log_context,
                        &AuxiliaryInfo::default(),
                    )
                    .map(|(_vm_status, vm_output)| {
                        vm_output
                            .try_materialize_into_transaction_output(state_view)
                            .unwrap()
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })?;

        Ok(BlockOutput::new(
            transaction_outputs,
            Some(block_epilogue_txn.into()),
        ))
    }
}
