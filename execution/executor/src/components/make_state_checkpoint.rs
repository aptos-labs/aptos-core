// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use itertools::{Itertools};
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::HashValue;
use aptos_executor_types::chunk_output::ChunkOutput;
use aptos_executor_types::state_checkpoint_output::StateCheckpointOutput;
use aptos_storage_interface::state_delta::StateDelta;
use crate::components::in_memory_state_calculator_v2::InMemoryStateCalculatorV2;

pub struct MakeStateCheckpoint;

impl MakeStateCheckpoint {
    pub fn calculate_state_checkpoint(
        chunk_output: &ChunkOutput,
        parent_state: &StateDelta,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        is_block: bool,
    ) -> Result<StateCheckpointOutput> {
        // Apply the write set, get the latest state.
        let mut res = InMemoryStateCalculatorV2::calculate_for_transactions(
            parent_state,
            chunk_output,
            is_block,
        )?;

        // On state sync/replay, we generate state checkpoints only periodically, for the
        // last state checkpoint of each chunk.
        // A mismatch in the SMT will be detected at that occasion too. Here we just copy
        // in the state root from the TxnInfo in the proof.
        if let Some(state_checkpoint_hashes) = known_state_checkpoints {
            res.check_and_update_state_checkpoint_hashes(state_checkpoint_hashes)?;
        }

        Ok(res)
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
        let txns_n_outputs =
            TransactionsWithParsedOutput::new(vec![Transaction::dummy(), Transaction::dummy()], vec![
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
            ]);
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
