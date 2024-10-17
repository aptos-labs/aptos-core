// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_executor_types::{
    should_forward_to_subscription_service, ChunkCommitNotification, LedgerUpdateOutput,
};
use aptos_storage_interface::{state_delta::StateDelta, ExecutedTrees};
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    transaction::TransactionToCommit,
};

#[derive(Debug)]
pub struct ExecutedChunk {
    pub result_state: StateDelta,
    pub ledger_info: Option<LedgerInfoWithSignatures>,
    /// If set, this is the new epoch info that should be changed to if this is committed.
    pub next_epoch_state: Option<EpochState>,
    pub ledger_update_output: LedgerUpdateOutput,
}

impl ExecutedChunk {
    pub fn transactions_to_commit(&self) -> &Vec<TransactionToCommit> {
        &self.ledger_update_output.to_commit
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    pub fn result_view(&self) -> ExecutedTrees {
        ExecutedTrees::new(
            self.result_state.clone(),
            self.ledger_update_output.transaction_accumulator.clone(),
        )
    }

    pub fn into_chunk_commit_notification(self) -> ChunkCommitNotification {
        let reconfiguration_occurred = self.has_reconfiguration();

        let mut committed_transactions =
            Vec::with_capacity(self.ledger_update_output.to_commit.len());
        let mut subscribable_events =
            Vec::with_capacity(self.ledger_update_output.to_commit.len() * 2);
        for txn_to_commit in &self.ledger_update_output.to_commit {
            let TransactionToCommit {
                transaction,
                events,
                ..
            } = txn_to_commit;
            committed_transactions.push(transaction.clone());
            subscribable_events.extend(
                events
                    .iter()
                    .filter(|evt| should_forward_to_subscription_service(evt))
                    .cloned(),
            );
        }

        ChunkCommitNotification {
            committed_transactions,
            subscribable_events,
            reconfiguration_occurred,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self {
            result_state: Default::default(),
            ledger_info: None,
            next_epoch_state: None,
            ledger_update_output: Default::default(),
        }
    }
}

#[test]
fn into_chunk_commit_notification_should_apply_event_filters() {
    use aptos_types::{account_config::NewEpochEvent, contract_event::ContractEvent};
    let event_1 = ContractEvent::new_v2_with_type_tag_str(
        "0x2345::random_module::RandomEvent",
        b"random_data_x".to_vec(),
    );
    let event_2 =
        ContractEvent::new_v2_with_type_tag_str("0x1::dkg::DKGStartEvent", b"dkg_data_2".to_vec());
    let event_3 = ContractEvent::new_v2_with_type_tag_str(
        "0x6789::random_module::RandomEvent",
        b"random_data_y".to_vec(),
    );
    let event_4 = ContractEvent::from((1, NewEpochEvent::dummy()));

    let ledger_update_output = LedgerUpdateOutput::new_dummy_with_txns_to_commit(vec![
        TransactionToCommit::dummy_with_events(vec![event_1.clone()]),
        TransactionToCommit::dummy_with_events(vec![event_2.clone(), event_3.clone()]),
        TransactionToCommit::dummy_with_events(vec![event_4.clone()]),
    ]);

    let chunk = ExecutedChunk {
        ledger_update_output,
        ..ExecutedChunk::dummy()
    };

    let notification = chunk.into_chunk_commit_notification();

    assert_eq!(vec![event_2, event_4], notification.subscribable_events);
}
