// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics,
    metrics::{EXECUTOR_ERRORS, OTHER_TIMERS},
};
use anyhow::{anyhow, Result};
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
use aptos_storage_interface::state_store::state_view::cached_state_view::{
    CachedStateView, StateCache,
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
        signature_verified_transaction::SignatureVerifiedTransaction, BlockEndInfo, BlockOutput,
        Transaction, TransactionOutput, TransactionStatus, Version,
    },
    write_set::{TransactionWrite, WriteSet},
};
use aptos_vm::VMBlockExecutor;
use itertools::Itertools;
use std::sync::Arc;

pub struct DoGetExecutionOutput;

impl DoGetExecutionOutput {
    pub fn by_transaction_execution<V: VMBlockExecutor>(
        executor: &V,
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<ExecutionOutput> {
        let out = match transactions {
            ExecutableTransactions::Unsharded(txns) => {
                Self::by_transaction_execution_unsharded::<V>(
                    executor,
                    txns,
                    state_view,
                    onchain_config,
                    transaction_slice_metadata,
                )?
            },
            ExecutableTransactions::Sharded(txns) => Self::by_transaction_execution_sharded::<V>(
                txns,
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
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<ExecutionOutput> {
        let append_state_checkpoint_to_block =
            transaction_slice_metadata.append_state_checkpoint_to_block();
        let txn_provider = DefaultTxnProvider::new(transactions);
        let block_output = Self::execute_block::<V>(
            executor,
            &txn_provider,
            &state_view,
            onchain_config,
            transaction_slice_metadata,
        )?;
        let (transaction_outputs, block_end_info) = block_output.into_inner();

        Parser::parse(
            state_view.next_version(),
            txn_provider
                .txns
                .into_iter()
                .map(|t| t.into_inner())
                .collect(),
            transaction_outputs,
            state_view.into_state_cache(),
            block_end_info,
            append_state_checkpoint_to_block,
        )
    }

    pub fn by_transaction_execution_sharded<V: VMBlockExecutor>(
        transactions: PartitionedTransactions,
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
            state_view.into_state_cache(),
            None, // block end info
            append_state_checkpoint_to_block,
        )
    }

    pub fn by_transaction_output(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        state_view: CachedStateView,
    ) -> Result<ExecutionOutput> {
        // collect all accounts touched and dedup
        let write_set = transaction_outputs
            .iter()
            .map(|o| o.write_set())
            .collect::<Vec<_>>();

        // prime the state cache by fetching all touched accounts
        state_view.prime_cache_by_write_set(write_set)?;

        let out = Parser::parse(
            state_view.next_version(),
            transactions,
            transaction_outputs,
            state_view.into_state_cache(),
            None, // block end info
            None, // append state checkpoint to block
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
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<TransactionOutput>> {
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
        state_cache: StateCache,
        block_end_info: Option<BlockEndInfo>,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ExecutionOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output"]);

        let is_block = append_state_checkpoint_to_block.is_some();

        // Collect all statuses.
        let statuses_for_input_txns = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__all_statuses"]);
            transaction_outputs
                .iter()
                .map(|t| t.status())
                .cloned()
                .collect_vec()
        };

        // Isolate retries.
        let (to_retry, has_reconfig) =
            Self::extract_retries(&mut transactions, &mut transaction_outputs);

        // Isolate discards.
        let to_discard = Self::extract_discards(&mut transactions, &mut transaction_outputs);

        // The rest is to be committed, attach block epilogue as needed and optionally get next EpochState.
        let to_commit = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__to_commit"]);
            let to_commit = TransactionsWithOutput::new(transactions, transaction_outputs);
            TransactionsToKeep::index(
                Self::maybe_add_block_epilogue(
                    to_commit,
                    has_reconfig,
                    block_end_info.as_ref(),
                    append_state_checkpoint_to_block,
                ),
                has_reconfig,
            )
        };
        let next_epoch_state = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__next_epoch_state"]);
            has_reconfig
                .then(|| Self::ensure_next_epoch_state(&to_commit))
                .transpose()?
        };

        let out = ExecutionOutput::new(
            is_block,
            first_version,
            statuses_for_input_txns,
            to_commit,
            to_discard,
            to_retry,
            state_cache,
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

    fn extract_retries(
        transactions: &mut Vec<Transaction>,
        transaction_outputs: &mut Vec<TransactionOutput>,
    ) -> (TransactionsWithOutput, bool) {
        let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__retries"]);

        let last_non_retry = transaction_outputs
            .iter()
            .rposition(|t| !t.status().is_retry());
        let is_reconfig = if let Some(idx) = last_non_retry {
            transaction_outputs[idx].has_new_epoch_event()
        } else {
            false
        };

        let first_retry = last_non_retry.map_or(0, |pos| pos + 1);
        let to_retry = TransactionsWithOutput::new(
            transactions.drain(first_retry..).collect(),
            transaction_outputs.drain(first_retry..).collect(),
        );

        (to_retry, is_reconfig)
    }

    fn extract_discards(
        transactions: &mut Vec<Transaction>,
        transaction_outputs: &mut Vec<TransactionOutput>,
    ) -> TransactionsWithOutput {
        let _timer = OTHER_TIMERS.timer_with(&["parse_raw_output__discards"]);

        let to_discard = {
            let mut res = TransactionsWithOutput::new_empty();
            for idx in 0..transactions.len() {
                if transaction_outputs[idx].status().is_discarded() {
                    res.push(transactions[idx].clone(), transaction_outputs[idx].clone());
                } else if !res.is_empty() {
                    transactions[idx - res.len()] = transactions[idx].clone();
                    transaction_outputs[idx - res.len()] = transaction_outputs[idx].clone();
                }
            }
            if !res.is_empty() {
                let remaining = transactions.len() - res.len();
                transactions.truncate(remaining);
                transaction_outputs.truncate(remaining);
            }
            res
        };

        // Sanity check transactions with the Discard status:
        to_discard.iter().for_each(|(t, o)| {
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

        to_discard
    }

    fn maybe_add_block_epilogue(
        mut to_commit: TransactionsWithOutput,
        is_reconfig: bool,
        block_end_info: Option<&BlockEndInfo>,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> TransactionsWithOutput {
        if !is_reconfig {
            // Append the StateCheckpoint transaction to the end
            if let Some(block_id) = append_state_checkpoint_to_block {
                let state_checkpoint_txn = match block_end_info {
                    None => Transaction::StateCheckpoint(block_id),
                    Some(block_end_info) => {
                        Transaction::block_epilogue(block_id, block_end_info.clone())
                    },
                };

                to_commit.push(state_checkpoint_txn, TransactionOutput::new_empty_success());
            }
        }; // else: not adding block epilogue at epoch ending.

        to_commit
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

impl<'a> TStateView for WriteSetStateView<'a> {
    type Key = StateKey;

    fn get_state_value(
        &self,
        state_key: &Self::Key,
    ) -> aptos_types::state_store::StateViewResult<Option<StateValue>> {
        Ok(self
            .write_set
            .get(state_key)
            .and_then(|write_op| write_op.as_state_value()))
    }

    fn get_usage(&self) -> aptos_types::state_store::StateViewResult<StateStorageUsage> {
        unreachable!("Not supposed to be called on WriteSetStateView.")
    }
}
#[cfg(test)]
mod tests {
    use super::Parser;
    use aptos_storage_interface::state_store::state_view::cached_state_view::StateCache;
    use aptos_types::{
        contract_event::ContractEvent,
        transaction::{
            ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionOutput,
            TransactionStatus,
        },
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
        let execution_output =
            Parser::parse(0, txns, txn_outs, StateCache::new_dummy(), None, None).unwrap();
        assert_eq!(
            vec![event_0, event_2],
            *execution_output.subscribable_events
        );
    }
}
