// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{should_forward_to_subscription_service, ChunkCommitNotification, LedgerUpdateOutput};
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_storage_interface::{state_delta::StateDelta, ExecutedTrees};
#[cfg(test)]
use aptos_types::account_config::NewEpochEvent;
#[cfg(test)]
use aptos_types::contract_event::ContractEvent;
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    state_store::combine_or_add_sharded_state_updates, transaction::TransactionToCommit,
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
    pub fn reconfig_suffix(&self) -> Self {
        assert!(self.next_epoch_state.is_some());
        Self {
            result_state: self.result_state.clone(),
            ledger_info: None,
            next_epoch_state: self.next_epoch_state.clone(),
            ledger_update_output: self.ledger_update_output.reconfig_suffix(),
        }
    }

    pub fn transactions_to_commit(&self) -> &Vec<TransactionToCommit> {
        &self.ledger_update_output.to_commit
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    pub fn combine(&mut self, rhs: Self) {
        assert_eq!(
            self.ledger_update_output.next_version(),
            rhs.ledger_update_output.first_version(),
            "Chunks to be combined are not consecutive.",
        );
        let Self {
            result_state,
            ledger_info,
            next_epoch_state,
            ledger_update_output,
        } = rhs;

        let old_result_state = self.result_state.replace_with(result_state);
        // TODO(aldenhu): This is very unfortunate. Will revisit soon by remodeling the state diff.
        if self.result_state.base_version > old_result_state.base_version
            && old_result_state.base_version != old_result_state.current_version
        {
            combine_or_add_sharded_state_updates(
                &mut self
                    .ledger_update_output
                    .state_updates_until_last_checkpoint,
                old_result_state.updates_since_base,
            )
        }

        self.ledger_info = ledger_info;
        self.next_epoch_state = next_epoch_state;
        self.ledger_update_output.combine(ledger_update_output)
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
        let mut to_drop = Vec::with_capacity(self.ledger_update_output.to_commit.len());
        for txn_to_commit in self.ledger_update_output.to_commit {
            let TransactionToCommit {
                transaction,
                events,
                state_updates,
                write_set,
                ..
            } = txn_to_commit;
            committed_transactions.push(transaction);
            subscribable_events.extend(
                events
                    .into_iter()
                    .filter(should_forward_to_subscription_service),
            );
            to_drop.push((state_updates, write_set));
        }
        DEFAULT_DROPPER.schedule_drop(to_drop);

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

    let ledger_update_output = LedgerUpdateOutput {
        to_commit: vec![
            TransactionToCommit::dummy_with_events(vec![event_1.clone()]),
            TransactionToCommit::dummy_with_events(vec![event_2.clone(), event_3.clone()]),
            TransactionToCommit::dummy_with_events(vec![event_4.clone()]),
        ],
        ..Default::default()
    };

    let chunk = ExecutedChunk {
        ledger_update_output,
        ..ExecutedChunk::dummy()
    };

    let notification = chunk.into_chunk_commit_notification();

    assert_eq!(vec![event_2, event_4], notification.subscribable_events);
}
