// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::payload_manager::TPayloadManager;
use velor_bitvec::BitVec;
use velor_config::config::BlockTransactionFilterConfig;
use velor_consensus_types::{
    block::Block,
    common::{Author, Payload},
};
use velor_executor_types::*;
use velor_types::transaction::SignedTransaction;
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

    fn prefetch_payload_data(&self, _payload: &Payload, _author: Author, _timestamp: u64) {}

    fn check_denied_inline_transactions(
        &self,
        block: &Block,
        block_txn_filter_config: &BlockTransactionFilterConfig,
    ) -> anyhow::Result<()> {
        // If the filter is disabled, return early
        if !block_txn_filter_config.is_enabled() {
            return Ok(());
        }

        // Get the inline transactions for the block proposal. Note: all
        // transactions in a direct mempool payload are inline transactions.
        let (inline_transactions, _, _) = get_transactions_from_block(block)?;
        if inline_transactions.is_empty() {
            return Ok(());
        }

        // Fetch the block metadata
        let block_id = block.id();
        let block_author = block.author();
        let block_epoch = block.epoch();
        let block_timestamp = block.timestamp_usecs();

        // Identify any denied inline transactions
        let block_transaction_filter = block_txn_filter_config.block_transaction_filter();
        let denied_inline_transactions = block_transaction_filter.get_denied_block_transactions(
            block_id,
            block_author,
            block_epoch,
            block_timestamp,
            inline_transactions,
        );
        if !denied_inline_transactions.is_empty() {
            return Err(anyhow::anyhow!(
                "Inline transactions for DirectMempoolPayload denied by block transaction filter: {:?}",
                denied_inline_transactions
            ));
        }

        Ok(()) // No transactions were denied
    }

    fn check_payload_availability(&self, _block: &Block) -> Result<(), BitVec> {
        Ok(())
    }

    async fn get_transactions(
        &self,
        block: &Block,
        _block_signers: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
        get_transactions_from_block(block)
    }
}

/// Returns the direct mempool transactions from a block's payload.
fn get_transactions_from_block(
    block: &Block,
) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
    let Some(payload) = block.payload() else {
        return Ok((Vec::new(), None, None));
    };

    match payload {
        Payload::DirectMempool(txns) => Ok((txns.clone(), None, None)),
        _ => unreachable!(
            "DirectMempoolPayloadManager: Unacceptable payload type {}. Epoch: {}, Round: {}, Block: {}",
            payload,
            block.block_data().epoch(),
            block.block_data().round(),
            block.id()
        ),
    }
}
