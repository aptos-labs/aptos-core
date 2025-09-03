// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics,
    metrics::{EXECUTOR_ERRORS, OTHER_TIMERS},
};
use anyhow::{anyhow, ensure, Result};
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
#[cfg(feature = "consensus-only-perf-test")]
use aptos_block_executor::txn_provider::TxnProvider;
use aptos_crypto::HashValue;
use aptos_executor_service::{
    local_executor_helper::SHARDED_BLOCK_EXECUTOR,
    remote_executor_client::{get_remote_addresses, REMOTE_SHARDED_BLOCK_EXECUTOR},
};
use aptos_executor_types::{
    execution_output::ExecutionOutput,
    planned::Planned,
    should_forward_to_subscription_service,
    transactions_with_output::{TransactionsToKeep, TransactionsWithOutput},
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    state::LedgerState, state_view::cached_state_view::CachedStateView,
};
#[cfg(feature = "consensus-only-perf-test")]
use aptos_types::transaction::ExecutionStatus;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{ExecutableTransactions, PartitionedTransactions},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    epoch_state::EpochState,
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        TStateView,
    },
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        AuxiliaryInfoTrait, BlockOutput, PersistedAuxiliaryInfo, Transaction, TransactionOutput,
        TransactionStatus, Version,
    },
    write_set::{HotStateOp, TransactionWrite, WriteSet},
};
use aptos_vm::VMBlockExecutor;
use itertools::Itertools;
use std::sync::Arc;

pub struct DoGetExecutionOutput;

impl DoGetExecutionOutput {
    pub fn by_transaction_execution<V: VMBlockExecutor>(
        executor: &V,
        transactions: ExecutableTransactions,
        auxiliary_infos: Vec<AuxiliaryInfo>,
        parent_state: &LedgerState,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<ExecutionOutput> {
        let out = match transactions {
            ExecutableTransactions::Unsharded(txns) => {
                Self::by_transaction_execution_unsharded::<V>(
                    executor,
                    txns,
                    auxiliary_infos,
                    parent_state,
                    state_view,
                    onchain_config,
                    transaction_slice_metadata,
                )?
            },
            // TODO: Execution with auxiliary info is yet to be supported properly here for sharded transactions
            ExecutableTransactions::Sharded(txns) => Self::by_transaction_execution_sharded::<V>(
                txns,
                auxiliary_infos,
                parent_state,
                state_view,
                onchain_config,
                transaction_slice_metadata.append_state_checkpoint_to_block(),
            )?,
        };

        let ret = out.clone();
        THREAD_MANAGER.get_background_pool().spawn(move || {
            let _timer = OTHER_TIMERS.timer_with(&["async_update_counters__by_execution"]);
            for x in [&out.to_commit, &out.to_retry, &out.to_discard] {
                metrics::update_counters_for_processed_chunk(
                    &x.transactions,
                    &x.transaction_outputs,
                    "execution",
                )
            }
        });

        Ok(ret)
    }

    fn by_transaction_execution_unsharded<V: VMBlockExecutor>(
        executor: &V,
        transactions: Vec<SignatureVerifiedTransaction>,
        auxiliary_infos: Vec<AuxiliaryInfo>,
        parent_state: &LedgerState,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<ExecutionOutput> {
        let txn_provider = DefaultTxnProvider::new(transactions, auxiliary_infos.clone());
        let block_output = Self::execute_block::<V>(
            executor,
            &txn_provider,
            &state_view,
            onchain_config,
            transaction_slice_metadata,
        )?;
        let (mut transaction_outputs, block_epilogue_txn) = block_output.into_inner();
        let (transactions, mut auxiliary_infos) = txn_provider.into_inner();
        let mut transactions = transactions
            .into_iter()
            .map(|t| t.into_inner())
            .collect_vec();
        if let Some(block_epilogue_txn) = block_epilogue_txn {
            transactions.push(block_epilogue_txn.into_inner());

            // Check if all existing auxiliary infos are None to maintain consistency
            let all_auxiliary_infos_are_none = auxiliary_infos
                .iter()
                .all(|info| matches!(info.persisted_info(), PersistedAuxiliaryInfo::None));

            let block_epilogue_aux_info = if all_auxiliary_infos_are_none {
                // If all other auxiliary infos are None, use None for consistency (version 0 behavior)
                AuxiliaryInfo::new(PersistedAuxiliaryInfo::None, None)
            } else {
                // Otherwise, use the standard function (version 1 behavior)
                AuxiliaryInfo::auxiliary_info_at_txn_index(transactions.len() as u32 - 1)
            };

            auxiliary_infos.push(block_epilogue_aux_info);
        }

        // Manually create hotness write sets for block epilogue transaction(s), based on the block
        // end info saved. Note that even if we are re-executing transactions during a state sync,
        // the block end info is not re-computed and has to come from the previous execution.
        //
        // If the input transactions are from a normal block, the last one should be the epilogue.
        // If they are from a chunk (i.e. we are re-executing transactions during state sync), then
        // there could be zero or more block epilogue transactions, and we need to handle all of
        // them.
        //
        // TODO(HotState): it might be better to do this in AptosVM::execute_single_transaction,
        // but we need to figure out how to properly construct `VMOutput` from block end info.
        for (transaction, output) in transactions.iter().zip_eq(transaction_outputs.iter_mut()) {
            if let Transaction::BlockEpilogue(payload) = transaction {
                info!(
                    "epilogue payload. #promotions: {}. #evictions: {}",
                    payload.try_get_slots_to_make_hot().map_or(0, |t| t.len()),
                    payload.try_get_keys_to_evict().map_or(0, |t| t.len()),
                );
                assert!(output.status().is_kept(), "Block epilogue must be kept");
                let promotions = payload
                    .try_get_slots_to_make_hot()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(key, slot)| (key, HotStateOp::make_hot(slot)));
                let evictions = payload
                    .try_get_keys_to_evict()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(key, slot)| (key, HotStateOp::eviction(slot)));
                output.add_hotness(promotions.chain(evictions).collect());
            }
        }

        Parser::parse(
            state_view.next_version(),
            transactions,
            transaction_outputs,
            auxiliary_infos,
            parent_state,
            state_view,
            false, // prime_state_cache
            transaction_slice_metadata
                .append_state_checkpoint_to_block()
                .is_some(),
        )
    }

    pub fn by_transaction_execution_sharded<V: VMBlockExecutor>(
        transactions: PartitionedTransactions,
        auxiliary_infos: Vec<AuxiliaryInfo>,
        parent_state: &LedgerState,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ExecutionOutput> {
        let state_view_arc = Arc::new(state_view);
        let transaction_outputs = Self::execute_block_sharded::<V>(
            transactions.clone(),
            state_view_arc.clone(),
            onchain_config,
        )?;

        // TODO(Manu): Handle state checkpoint here.

        // TODO(skedia) add logic to emit counters per shard instead of doing it globally.

        // Unwrapping here is safe because the execution has finished and it is guaranteed that
        // the state view is not used anymore.
        let state_view = Arc::try_unwrap(state_view_arc).unwrap();
        Parser::parse(
            state_view.next_version(),
            PartitionedTransactions::flatten(transactions)
                .into_iter()
                .map(|t| t.into_txn().into_inner())
                .collect(),
            transaction_outputs,
            auxiliary_infos,
            parent_state,
            state_view,
            false, // prime_state_cache
            append_state_checkpoint_to_block.is_some(),
        )
    }

    pub fn by_transaction_output(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        auxiliary_infos: Vec<AuxiliaryInfo>,
        parent_state: &LedgerState,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        let out = Parser::parse(
            state_view.next_version(),
            transactions,
            transaction_outputs,
            auxiliary_infos,
            parent_state,
            state_view,
            true,  // prime state cache
            false, // is_block
        )?;

        let ret = out.clone();
        THREAD_MANAGER.get_background_pool().spawn(move || {
            let _timer = OTHER_TIMERS.timer_with(&["async_update_counters__by_output"]);
            metrics::update_counters_for_processed_chunk(
                &out.to_commit.transactions,
                &out.to_commit.transaction_outputs,
                "output",
            )
        });

        Ok(ret)
    }

    fn execute_block_sharded<V: VMBlockExecutor>(
        partitioned_txns: PartitionedTransactions,
        state_view: Arc<CachedStateView>,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<Vec<TransactionOutput>> {
        if !get_remote_addresses().is_empty() {
            Ok(V::execute_block_sharded(
                &REMOTE_SHARDED_BLOCK_EXECUTOR.lock(),
                partitioned_txns,
                state_view,
                onchain_config,
            )?)
        } else {
            Ok(V::execute_block_sharded(
                &SHARDED_BLOCK_EXECUTOR.lock(),
                partitioned_txns,
                state_view,
                onchain_config,
            )?)
        }
    }

    /// Executes the block of [Transaction]s using the [VMBlockExecutor] and returns
    /// a vector of [TransactionOutput]s.
    #[cfg(not(feature = "consensus-only-perf-test"))]
    fn execute_block<V: VMBlockExecutor>(
        executor: &V,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
        state_view: &CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<SignatureVerifiedTransaction, TransactionOutput>> {
        let _timer = OTHER_TIMERS.timer_with(&["vm_execute_block"]);
        Ok(executor.execute_block(
            txn_provider,
            state_view,
            onchain_config,
            transaction_slice_metadata,
        )?)
    }

    /// In consensus-only mode, executes the block of [Transaction]s using the
    /// [VMBlockExecutor] only if its a genesis block. In all other cases, this
    /// method returns an [TransactionOutput] with an empty [WriteSet], constant
    /// gas and a [ExecutionStatus::Success] for each of the [Transaction]s.
    #[cfg(feature = "consensus-only-perf-test")]
    fn execute_block<V: VMBlockExecutor>(
        executor: &V,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<TransactionOutput>> {
        use aptos_types::{
            state_store::{StateViewId, TStateView},
            transaction::TransactionAuxiliaryData,
            write_set::WriteSet,
        };

        let transaction_outputs = match state_view.id() {
            // this state view ID implies a genesis block in non-test cases.
            StateViewId::Miscellaneous => executor.execute_block(
                txn_provider,
                state_view,
                onchain_config,
                transaction_slice_metadata,
            )?,
            _ => BlockOutput::new(
                (0..txn_provider.num_txns())
                    .map(|_| {
                        TransactionOutput::new(
                            WriteSet::default(),
                            Vec::new(),
                            0, // Keep gas zero to match with StateCheckpoint txn output
                            TransactionStatus::Keep(ExecutionStatus::Success),
                            TransactionAuxiliaryData::None,
                        )
                    })
                    .collect::<Vec<_>>(),
                None,
            ),
        };
        Ok(transaction_outputs)
    }
}

struct Parser;

impl Parser {
    fn parse(
        first_version: Version,
        mut transactions: Vec<Transaction>,
        mut transaction_outputs: Vec<TransactionOutput>,
        auxiliary_infos: Vec<AuxiliaryInfo>,
        parent_state: &LedgerState,
        base_state_view: CachedStateView,
        prime_state_cache: bool,
        is_block: bool,
    ) -> Result<ExecutionOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output"]);

        // Collect all statuses.
        let mut statuses_for_input_txns = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__all_statuses"]);
            transaction_outputs
                .iter()
                .map(|t| t.status())
                .cloned()
                .collect_vec()
        };

        let mut persisted_auxiliary_infos = auxiliary_infos
            .into_iter()
            .map(|info| info.into_persisted_info())
            .collect();

        // Isolate retries and discards.
        let (to_retry, to_discard, has_reconfig) = Self::extract_retries_and_discards(
            &mut transactions,
            &mut transaction_outputs,
            &mut persisted_auxiliary_infos,
        );

        let mut block_end_info = None;
        if is_block && !has_reconfig {
            if let Some(Transaction::BlockEpilogue(payload)) = transactions.last() {
                block_end_info = payload.try_as_block_end_info().cloned();
                ensure!(statuses_for_input_txns.pop().is_some());
            }
        }

        // The rest is to be committed, attach block epilogue as needed and optionally get next EpochState.
        let to_commit = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__to_commit"]);
            let to_commit = TransactionsWithOutput::new(
                transactions,
                transaction_outputs,
                persisted_auxiliary_infos,
            );
            TransactionsToKeep::index(first_version, to_commit, has_reconfig)
        };
        let next_epoch_state = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__next_epoch_state"]);
            has_reconfig
                .then(|| Self::ensure_next_epoch_state(&to_commit))
                .transpose()?
        };

        if prime_state_cache {
            base_state_view.prime_cache(to_commit.state_update_refs())?;
        }

        let result_state = parent_state.update_with_memorized_reads(
            Arc::clone(&base_state_view.hot),
            base_state_view.persisted_state(),
            to_commit.state_update_refs(),
            base_state_view.memorized_reads(),
        );
        let state_reads = base_state_view.into_memorized_reads();

        let out = ExecutionOutput::new(
            is_block,
            first_version,
            statuses_for_input_txns,
            to_commit,
            to_discard,
            to_retry,
            result_state,
            state_reads,
            block_end_info,
            next_epoch_state,
            Planned::place_holder(),
        );
        let ret = out.clone();
        ret.subscribable_events
            .plan(THREAD_MANAGER.get_non_exe_cpu_pool(), move || {
                Self::get_subscribable_events(&out)
            });
        Ok(ret)
    }

    fn get_subscribable_events(out: &ExecutionOutput) -> Vec<ContractEvent> {
        out.to_commit
            .transaction_outputs
            .iter()
            .flat_map(TransactionOutput::events)
            .filter(|e| should_forward_to_subscription_service(e))
            .cloned()
            .collect_vec()
    }

    fn extract_retries_and_discards(
        transactions: &mut Vec<Transaction>,
        transaction_outputs: &mut Vec<TransactionOutput>,
        persisted_auxiliary_infos: &mut Vec<PersistedAuxiliaryInfo>,
    ) -> (TransactionsWithOutput, TransactionsWithOutput, bool) {
        let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__retries_and_discards"]);

        let last_non_retry = transaction_outputs
            .iter()
            .rposition(|t| !t.status().is_retry());
        let is_reconfig = if let Some(idx) = last_non_retry {
            transaction_outputs[idx].has_new_epoch_event()
        } else {
            false
        };

        let mut to_discard = TransactionsWithOutput::new_empty();
        let mut to_retry = TransactionsWithOutput::new_empty();

        let mut num_keep_txns = 0;

        for idx in 0..transactions.len() {
            match transaction_outputs[idx].status() {
                TransactionStatus::Keep(_) => {
                    if num_keep_txns != idx {
                        transactions[num_keep_txns] = transactions[idx].clone();
                        transaction_outputs[num_keep_txns] = transaction_outputs[idx].clone();
                        persisted_auxiliary_infos[num_keep_txns] = persisted_auxiliary_infos[idx];
                    }
                    num_keep_txns += 1;
                },
                TransactionStatus::Retry => to_retry.push(
                    transactions[idx].clone(),
                    transaction_outputs[idx].clone(),
                    persisted_auxiliary_infos[idx],
                ),
                TransactionStatus::Discard(_) => to_discard.push(
                    transactions[idx].clone(),
                    transaction_outputs[idx].clone(),
                    persisted_auxiliary_infos[idx],
                ),
            }
        }

        transactions.truncate(num_keep_txns);
        transaction_outputs.truncate(num_keep_txns);
        persisted_auxiliary_infos.truncate(num_keep_txns);

        // Sanity check transactions with the Discard status:
        to_discard.iter().for_each(|(t, o, _)| {
            // In case a new status other than Retry, Keep and Discard is added:
            if !matches!(o.status(), TransactionStatus::Discard(_)) {
                error!("Status other than Retry, Keep or Discard; Transaction discarded.");
            }
            // VM shouldn't have output anything for discarded transactions, log if it did.
            if !o.write_set().is_empty() || !o.events().is_empty() {
                error!(
                    "Discarded transaction has non-empty write set or events. \
                        Transaction: {:?}. Status: {:?}.",
                    t,
                    o.status(),
                );
                EXECUTOR_ERRORS.inc();
            }
        });

        (to_retry, to_discard, is_reconfig)
    }

    fn ensure_next_epoch_state(to_commit: &TransactionsWithOutput) -> Result<EpochState> {
        let last_write_set = to_commit
            .transaction_outputs
            .last()
            .ok_or_else(|| anyhow!("to_commit is empty."))?
            .write_set();

        let write_set_view = WriteSetStateView {
            write_set: last_write_set,
        };

        let validator_set = ValidatorSet::fetch_config(&write_set_view)
            .ok_or_else(|| anyhow!("ValidatorSet not touched on epoch change"))?;
        let configuration = ConfigurationResource::fetch_config(&write_set_view)
            .ok_or_else(|| anyhow!("Configuration resource not touched on epoch change"))?;

        Ok(EpochState::new(
            configuration.epoch(),
            (&validator_set).into(),
        ))
    }
}

struct WriteSetStateView<'a> {
    write_set: &'a WriteSet,
}

impl TStateView for WriteSetStateView<'_> {
    type Key = StateKey;

    fn get_state_value(
        &self,
        state_key: &Self::Key,
    ) -> aptos_types::state_store::StateViewResult<Option<StateValue>> {
        Ok(self
            .write_set
            .get_write_op(state_key)
            .and_then(|write_op| write_op.as_state_value()))
    }

    fn get_usage(&self) -> aptos_types::state_store::StateViewResult<StateStorageUsage> {
        unreachable!("Not supposed to be called on WriteSetStateView.")
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use aptos_storage_interface::state_store::{
        state::LedgerState, state_view::cached_state_view::CachedStateView,
    };
    use aptos_types::{
        contract_event::ContractEvent,
        transaction::{
            AuxiliaryInfo, ExecutionStatus, PersistedAuxiliaryInfo, Transaction,
            TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
        },
        vm_status::StatusCode,
        write_set::WriteSet,
    };

    #[test]
    fn should_filter_subscribable_events() {
        let event_0 =
            ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_1".to_vec());
        let event_1 = ContractEvent::new_v2_with_type_tag_str(
            "0x2345::random_module::RandomEvent",
            b"random_x".to_vec(),
        );
        let event_2 =
            ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_2".to_vec());

        let txns = vec![Transaction::dummy(), Transaction::dummy()];
        let txn_outs = vec![
            TransactionOutput::new(
                WriteSet::default(),
                vec![event_0.clone()],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            ),
            TransactionOutput::new(
                WriteSet::default(),
                vec![event_1.clone(), event_2.clone()],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            ),
        ];
        let auxiliary_infos = vec![
            AuxiliaryInfo::new(
                PersistedAuxiliaryInfo::V1 {
                    transaction_index: 0,
                },
                None,
            ),
            AuxiliaryInfo::new(
                PersistedAuxiliaryInfo::V1 {
                    transaction_index: 1,
                },
                None,
            ),
        ];
        let state = LedgerState::new_empty();
        let execution_output = Parser::parse(
            0,
            txns,
            txn_outs,
            auxiliary_infos,
            &state,
            CachedStateView::new_dummy(&state),
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            vec![event_0, event_2],
            *execution_output.subscribable_events
        );
    }

    #[test]
    fn test_extract_retry_and_discard_no_reconfig() {
        let mut txns = vec![
            Transaction::dummy(),
            Transaction::dummy(),
            Transaction::dummy(),
            Transaction::dummy(),
        ];
        let mut txn_outs = vec![
            TransactionOutput::new(
                WriteSet::default(),
                vec![],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            ),
            TransactionOutput::new(
                WriteSet::default(),
                vec![],
                0,
                TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_OLD),
                TransactionAuxiliaryData::default(),
            ),
            TransactionOutput::new(
                WriteSet::default(),
                vec![],
                0,
                TransactionStatus::Retry,
                TransactionAuxiliaryData::default(),
            ),
            TransactionOutput::new(
                WriteSet::default(),
                vec![],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            ),
        ];
        let mut auxiliary_infos = vec![
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 0,
            },
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 1,
            },
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 2,
            },
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 3,
            },
        ];
        let (to_retry, to_discard, is_reconfig) =
            Parser::extract_retries_and_discards(&mut txns, &mut txn_outs, &mut auxiliary_infos);
        assert!(!is_reconfig);
        assert_eq!(to_retry.len(), 1);
        assert_eq!(to_discard.len(), 1);
        assert_eq!(txns.len(), 2);
        assert_eq!(txn_outs.len(), 2);
        assert_eq!(auxiliary_infos.len(), 2);
        assert_eq!(
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 0
            },
            auxiliary_infos[0]
        );
        assert_eq!(
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 3
            },
            auxiliary_infos[1]
        );
    }

    #[test]
    fn test_extract_retry_and_discard_reconfig() {
        let reconfig_event = ContractEvent::new_v2_with_type_tag_str(
            "0x1::reconfiguration::NewEpochEvent",
            b"".to_vec(),
        );
        let mut txns = vec![
            Transaction::dummy(),
            Transaction::dummy(),
            Transaction::dummy(),
        ];
        let mut txn_outs = vec![
            TransactionOutput::new(
                WriteSet::default(),
                vec![reconfig_event],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            ),
            TransactionOutput::new(
                WriteSet::default(),
                vec![],
                0,
                TransactionStatus::Retry,
                TransactionAuxiliaryData::default(),
            ),
            TransactionOutput::new(
                WriteSet::default(),
                vec![],
                0,
                TransactionStatus::Retry,
                TransactionAuxiliaryData::default(),
            ),
        ];
        let mut auxiliary_infos = vec![
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 0,
            },
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 1,
            },
            PersistedAuxiliaryInfo::V1 {
                transaction_index: 2,
            },
        ];
        let (to_retry, to_discard, is_reconfig) =
            Parser::extract_retries_and_discards(&mut txns, &mut txn_outs, &mut auxiliary_infos);
        assert!(is_reconfig);
        assert_eq!(to_retry.len(), 2);
        assert_eq!(to_discard.len(), 0);
        assert_eq!(txns.len(), 1);
        assert_eq!(txn_outs.len(), 1);
        assert_eq!(auxiliary_infos.len(), 1);
    }
}
