// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::components::partial_state_compute_result::PartialStateComputeResult;
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::TransactionToCommit};

#[derive(Debug)]
pub struct ExecutedChunk {
    pub output: PartialStateComputeResult,
    pub ledger_info_opt: Option<LedgerInfoWithSignatures>,
}

impl ExecutedChunk {
    pub fn transactions_to_commit(&self) -> &[TransactionToCommit] {
        &self.output.expect_ledger_update_output().to_commit
    }
}

#[test]
fn into_chunk_commit_notification_should_apply_event_filters() {
    /* FIXME(aldenhu): repair
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
     */
}
