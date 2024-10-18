// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    components::do_get_execution_output::update_counters_for_processed_chunk, metrics::OTHER_TIMERS,
};
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{
    parsed_transaction_output::TransactionsWithParsedOutput,
    should_forward_to_subscription_service, state_checkpoint_output::StateCheckpointOutput,
    LedgerUpdateOutput, ParsedTransactionOutput,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    contract_event::ContractEvent,
    proof::accumulator::{InMemoryEventAccumulator, InMemoryTransactionAccumulator},
    transaction::{Transaction, TransactionInfo},
};
use itertools::{izip, Itertools};
use rayon::prelude::*;
use std::sync::Arc;

pub struct DoLedgerUpdate;

impl DoLedgerUpdate {
    pub fn run(
        state_checkpoint_output: StateCheckpointOutput,
        base_txn_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Result<(LedgerUpdateOutput, Vec<Transaction>, Vec<Transaction>)> {
        let (
            txns,
            state_updates_vec,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
            block_end_info,
        ) = state_checkpoint_output.into_inner();

        let (statuses_for_input_txns, to_commit, to_discard, to_retry) = txns.into_inner();
        for group in [&to_commit, &to_discard, &to_retry] {
            update_counters_for_processed_chunk(group.txns(), group.parsed_outputs(), "execution");
        }

        // these are guaranteed by caller side logic
        assert_eq!(to_commit.len(), state_updates_vec.len());
        assert_eq!(to_commit.len(), state_checkpoint_hashes.len());

        let (event_hashes, write_set_hashes) =
            Self::calculate_events_and_writeset_hashes(to_commit.parsed_outputs());

        let (transaction_infos, subscribible_events) = Self::assemble_transaction_infos(
            &to_commit,
            state_checkpoint_hashes,
            event_hashes,
            write_set_hashes,
        );
        let transaction_info_hashes = transaction_infos.iter().map(CryptoHash::hash).collect_vec();
        let transaction_accumulator =
            Arc::new(base_txn_accumulator.append(&transaction_info_hashes));

        let (transactions, transaction_outputs) = to_commit.into_inner();

        let ledger_update_output = LedgerUpdateOutput::new(
            statuses_for_input_txns,
            transactions,
            transaction_outputs,
            transaction_infos,
            state_updates_vec,
            subscribible_events,
            transaction_info_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
            transaction_accumulator,
            base_txn_accumulator,
            block_end_info,
        );

        Ok((
            ledger_update_output,
            to_discard.into_txns(),
            to_retry.into_txns(),
        ))
    }

    fn calculate_events_and_writeset_hashes(
        to_commit: &[ParsedTransactionOutput],
    ) -> (Vec<HashValue>, Vec<HashValue>) {
        let _timer = OTHER_TIMERS.timer_with(&["calculate_events_and_writeset_hashes"]);

        let num_txns = to_commit.len();
        to_commit
            .par_iter()
            .with_min_len(optimal_min_len(num_txns, 64))
            .map(|txn_output| {
                let event_hashes = txn_output
                    .events()
                    .iter()
                    .map(CryptoHash::hash)
                    .collect::<Vec<_>>();

                (
                    InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash(),
                    CryptoHash::hash(txn_output.write_set()),
                )
            })
            .unzip()
    }

    fn assemble_transaction_infos(
        to_commit: &TransactionsWithParsedOutput,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        event_hashes: Vec<HashValue>,
        writeset_hashes: Vec<HashValue>,
    ) -> (Vec<TransactionInfo>, Vec<ContractEvent>) {
        let _timer = OTHER_TIMERS.timer_with(&["process_events_and_writeset_hashes"]);

        let mut txn_infos = Vec::with_capacity(to_commit.len());
        let mut subscribable_events = Vec::new();
        izip!(
            to_commit.iter(),
            state_checkpoint_hashes,
            event_hashes,
            writeset_hashes
        )
        .for_each(
            |((txn, txn_out), state_checkpoint_hash, event_root_hash, write_set_hash)| {
                subscribable_events.extend(
                    txn_out
                        .events()
                        .iter()
                        .filter(|evt| should_forward_to_subscription_service(evt))
                        .cloned(),
                );
                txn_infos.push(TransactionInfo::new(
                    txn.hash(),
                    write_set_hash,
                    event_root_hash,
                    state_checkpoint_hash,
                    txn_out.gas_used(),
                    txn_out.status().as_kept_status().expect("Already sorted."),
                ));
            },
        );

        (txn_infos, subscribable_events)
    }
}

#[cfg(test)]
mod tests {
    use super::DoLedgerUpdate;
    use aptos_crypto::hash::HashValue;
    use aptos_executor_types::parsed_transaction_output::{
        ParsedTransactionOutput, TransactionsWithParsedOutput,
    };
    use aptos_types::{
        contract_event::ContractEvent,
        transaction::{
            ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionOutput,
            TransactionStatus,
        },
        write_set::WriteSet,
    };

    #[test]
    fn assemble_ledger_diff_should_filter_subscribable_events() {
        let event_0 =
            ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_1".to_vec());
        let event_1 = ContractEvent::new_v2_with_type_tag_str(
            "0x2345::random_module::RandomEvent",
            b"random_x".to_vec(),
        );
        let event_2 =
            ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_2".to_vec());
        let txns_n_outputs = TransactionsWithParsedOutput::new(
            vec![Transaction::dummy(), Transaction::dummy()],
            vec![
                ParsedTransactionOutput::from(TransactionOutput::new(
                    WriteSet::default(),
                    vec![event_0.clone()],
                    0,
                    TransactionStatus::Keep(ExecutionStatus::Success),
                    TransactionAuxiliaryData::default(),
                )),
                ParsedTransactionOutput::from(TransactionOutput::new(
                    WriteSet::default(),
                    vec![event_1.clone(), event_2.clone()],
                    0,
                    TransactionStatus::Keep(ExecutionStatus::Success),
                    TransactionAuxiliaryData::default(),
                )),
            ],
        );
        let state_checkpoint_hashes = vec![Some(HashValue::zero()); 2];
        let event_hashes = vec![HashValue::zero(); 2];
        let write_set_hashes = vec![HashValue::zero(); 2];
        let (_transaction_infos, subscribable_events) = DoLedgerUpdate::assemble_transaction_infos(
            &txns_n_outputs,
            state_checkpoint_hashes,
            event_hashes,
            write_set_hashes,
        );
        assert_eq!(vec![event_0, event_2], subscribable_events);
    }
}
