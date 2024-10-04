// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::metrics::OTHER_TIMERS;
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{
    chunk_output::ChunkOutput, parsed_transaction_output::TransactionsWithParsedOutput,
    should_forward_to_subscription_service, LedgerUpdateOutput, ParsedTransactionOutput,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    contract_event::ContractEvent,
    proof::accumulator::{InMemoryEventAccumulator, InMemoryTransactionAccumulator},
    transaction::TransactionInfo,
};
use itertools::{izip, Itertools};
use rayon::prelude::*;
use std::sync::Arc;

pub struct MakeLedgerUpdate;

impl MakeLedgerUpdate {
    pub fn make(
        chunk_output: &ChunkOutput,
        state_checkpoint_hashes: &[Option<HashValue>],
        base_txn_accumulator: &InMemoryTransactionAccumulator,
    ) -> Result<LedgerUpdateOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["assemble_ledger_diff_for_block"]);

        // Update counters.
        chunk_output.update_counters_for_processed_chunk();

        // Calculate hashes
        let to_commit = &chunk_output.to_commit;
        let txn_outs = to_commit.parsed_outputs();

        let (event_hashes, writeset_hashes) = Self::calculate_events_and_writeset_hashes(txn_outs);

        // Assemble `TransactionInfo`s
        let (transaction_infos, subscribable_events) = Self::assemble_transaction_infos(
            &to_commit,
            &state_checkpoint_hashes,
            &event_hashes,
            &writeset_hashes,
        );

        // Calculate root hash
        let transaction_info_hashes = transaction_infos.iter().map(CryptoHash::hash).collect_vec();
        let transaction_accumulator =
            Arc::new(base_txn_accumulator.append(&transaction_info_hashes));

        Ok(LedgerUpdateOutput {
            transaction_infos,
            transaction_info_hashes,
            transaction_accumulator,
            subscribable_events,
        })
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
        state_checkpoint_hashes: &[Option<HashValue>],
        event_hashes: &[HashValue],
        writeset_hashes: &[HashValue],
    ) -> (Vec<TransactionInfo>, Vec<ContractEvent>) {
        let _timer = OTHER_TIMERS.timer_with(&["process_events_and_writeset_hashes"]);

        izip!(
            to_commit.iter(),
            state_checkpoint_hashes,
            event_hashes,
            writeset_hashes
        )
        .map(
            |((txn, txn_out), state_checkpoint_hash, event_root_hash, write_set_hash)| {
                let subscribable_events: Vec<ContractEvent> = txn_out
                    .events()
                    .iter()
                    .filter(should_forward_to_subscription_service)
                    .cloned()
                    .collect();
                let txn_info = TransactionInfo::new(
                    txn.hash(),
                    write_set_hash,
                    event_root_hash,
                    state_checkpoint_hash.cloned(),
                    txn_out.gas_used(),
                    txn_out.status().as_kept_status().expect("Already sorted."),
                );
                (txn_info, subscribable_events)
            },
        )
        .unzip()
    }
}

mod tests {
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
        let state_updates_vec = vec![
            ShardedStateUpdates::default(),
            ShardedStateUpdates::default(),
        ];
        let state_checkpoint_hashes = vec![Some(HashValue::zero()), Some(HashValue::zero())];
        let (_, _, subscribable_events) = ApplyChunkOutput::calculate_transaction_infos(
            txns_n_outputs,
            state_updates_vec,
            state_checkpoint_hashes,
        );
        assert_eq!(vec![event_0, event_2], subscribable_events);
    }
}
