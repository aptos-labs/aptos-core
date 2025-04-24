// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::payload_manager::TPayloadManager;
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload},
};
use aptos_executor_types::*;
use aptos_types::transaction::SignedTransaction;
use async_trait::async_trait;

/// A payload manager that directly returns the transactions in a block's payload.
pub struct DirectMempoolPayloadManager {}

impl DirectMempoolPayloadManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TPayloadManager for DirectMempoolPayloadManager {
    fn notify_commit(&self, _block_timestamp: u64, _payloads: Vec<Payload>) {}

    fn prefetch_payload_data(
        &self,
        _payload: &Payload,
        _author: Author,
        _timestamp: u64,
        _voters: Option<BitVec>,
    ) {
    }

    async fn get_transactions(
        &self,
        block: &Block,
        _block_signers: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>)> {
        let Some(payload) = block.payload() else {
            return Ok((Vec::new(), None));
        };

        match payload {
            Payload::DirectMempool(txns) => Ok((txns.clone(), None)),
            _ => unreachable!(
                "DirectMempoolPayloadManager: Unacceptable payload type {}. Epoch: {}, Round: {}, Block: {}",
                payload,
                block.block_data().epoch(),
                block.block_data().round(),
                block.id()
            ),
        }
    }
}
