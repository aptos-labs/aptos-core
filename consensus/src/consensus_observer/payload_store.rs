// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::logging::{LogEntry, LogSchema};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::error;
use aptos_types::{block_info::BlockInfo, transaction::SignedTransaction};
use std::{
    collections::{hash_map::Entry, HashMap},
    mem,
    sync::Arc,
};
use tokio::sync::oneshot;

/// The transaction payload of each block
#[derive(Debug, Clone)]
pub struct BlockTransactionPayload {
    pub transactions: Vec<SignedTransaction>,
    pub limit: Option<usize>,
}

impl BlockTransactionPayload {
    pub fn new(transactions: Vec<SignedTransaction>, limit: Option<usize>) -> Self {
        Self {
            transactions,
            limit,
        }
    }
}

/// The status of the block payload (requested or available)
pub enum BlockPayloadStatus {
    Requested(oneshot::Sender<BlockTransactionPayload>),
    Available(BlockTransactionPayload),
}

/// A simple struct to store the block payloads of ordered and committed blocks
#[derive(Clone)]
pub struct BlockPayloadStore {
    // Block transaction payloads map the block ID to the transaction payloads
    // (the same payloads that the payload manager returns).
    block_transaction_payloads: Arc<Mutex<HashMap<HashValue, BlockPayloadStatus>>>,
}

impl BlockPayloadStore {
    pub fn new() -> Self {
        Self {
            block_transaction_payloads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns true iff all the payloads for the given blocks are available
    pub fn all_payloads_exist(&self, blocks: &[Arc<PipelinedBlock>]) -> bool {
        let block_transaction_payloads = self.block_transaction_payloads.lock();
        blocks.iter().all(|block| {
            matches!(
                block_transaction_payloads.get(&block.id()),
                Some(BlockPayloadStatus::Available(_))
            )
        })
    }

    /// Returns a reference to the block transaction payloads
    pub fn get_block_payloads(&self) -> Arc<Mutex<HashMap<HashValue, BlockPayloadStatus>>> {
        self.block_transaction_payloads.clone()
    }

    /// Inserts the given block payload data into the payload store
    pub fn insert_block_payload(
        &mut self,
        block: BlockInfo,
        transactions: Vec<SignedTransaction>,
        limit: Option<usize>,
    ) {
        let mut block_transaction_payloads = self.block_transaction_payloads.lock();
        let block_transaction_payload = BlockTransactionPayload::new(transactions, limit);

        match block_transaction_payloads.entry(block.id()) {
            Entry::Occupied(mut entry) => {
                // Replace the data status with the new block payload
                let mut status = BlockPayloadStatus::Available(block_transaction_payload.clone());
                mem::swap(entry.get_mut(), &mut status);

                // If the status was originally requested, send the payload to the listener
                if let BlockPayloadStatus::Requested(payload_sender) = status {
                    if payload_sender.send(block_transaction_payload).is_err() {
                        error!(LogSchema::new(LogEntry::ConsensusObserver)
                            .message("Failed to send block payload to listener!",));
                    }
                }
            },
            Entry::Vacant(entry) => {
                // Insert the block payload directly into the payload store
                entry.insert(BlockPayloadStatus::Available(block_transaction_payload));
            },
        }
    }

    /// Removes the given pipelined blocks from the payload store
    pub fn remove_blocks(&self, blocks: &[Arc<PipelinedBlock>]) {
        let mut block_transaction_payloads = self.block_transaction_payloads.lock();
        for block in blocks.iter() {
            block_transaction_payloads.remove(&block.id());
        }
    }
}

impl Default for BlockPayloadStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        quorum_cert::QuorumCert,
    };
    use aptos_types::{block_info::Round, transaction::Version};

    #[test]
    fn test_all_payloads_exist() {
        // Create a new block payload store
        let block_payload_store = BlockPayloadStore::new();

        // Add some blocks to the payload store
        let num_blocks_in_store = 100;
        let pipelined_blocks =
            create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_in_store);

        // Check that all the payloads exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that a subset of the payloads exist in the block payload store
        let subset_pipelined_blocks = &pipelined_blocks[0..50];
        assert!(block_payload_store.all_payloads_exist(subset_pipelined_blocks));

        // Remove some of the payloads from the block payload store
        block_payload_store.remove_blocks(subset_pipelined_blocks);

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_pipelined_blocks));

        // Check that the remaining payloads still exist in the block payload store
        let subset_pipelined_blocks = &pipelined_blocks[50..100];
        assert!(block_payload_store.all_payloads_exist(subset_pipelined_blocks));

        // Remove the remaining payloads from the block payload store
        block_payload_store.remove_blocks(subset_pipelined_blocks);

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_pipelined_blocks));
    }

    #[test]
    fn test_all_payloads_exist_requested() {
        // Create a new block payload store
        let block_payload_store = BlockPayloadStore::new();

        // Add several blocks to the payload store
        let num_blocks_in_store = 10;
        let pipelined_blocks =
            create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_in_store);

        // Check that the payloads exists in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Mark the payload of the first block as requested
        mark_payload_as_requested(block_payload_store.clone(), pipelined_blocks[0].id());

        // Check that the payload no longer exists in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that the remaining payloads still exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks[1..10]));
    }

    #[test]
    fn test_insert_block_payload() {
        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new();

        // Add some blocks to the payload store
        let num_blocks_in_store = 10;
        let pipelined_blocks =
            create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_in_store);

        // Check that the block payload store contains the new block payloads
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Mark the payload of the first block as requested
        let payload_receiver =
            mark_payload_as_requested(block_payload_store.clone(), pipelined_blocks[0].id());

        // Check that the payload no longer exists in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Insert the same block payload into the block payload store
        block_payload_store.insert_block_payload(pipelined_blocks[0].block_info(), vec![], Some(0));

        // Check that the block payload store now contains the requested block payload
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that the payload receiver receives the requested block payload message
        let block_transaction_payload = payload_receiver.blocking_recv().unwrap();
        assert!(block_transaction_payload.transactions.is_empty());
        assert_eq!(block_transaction_payload.limit, Some(0));
    }

    #[test]
    fn test_remove_blocks() {
        // Create a new block payload store
        let block_payload_store = BlockPayloadStore::new();

        // Add some blocks to the payload store
        let num_blocks_in_store = 10;
        let pipelined_blocks =
            create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_in_store);

        // Remove the first block from the block payload store
        block_payload_store.remove_blocks(&pipelined_blocks[0..1]);

        // Check that the block payload store no longer contains the removed block
        let block_transaction_payloads = block_payload_store.get_block_payloads();
        assert!(!block_transaction_payloads
            .lock()
            .contains_key(&pipelined_blocks[0].id()));

        // Remove the last 5 blocks from the block payload store
        block_payload_store.remove_blocks(&pipelined_blocks[5..10]);

        // Check that the block payload store no longer contains the removed blocks
        let block_transaction_payloads = block_payload_store.get_block_payloads();
        for pipelined_block in pipelined_blocks.iter().take(10).skip(5) {
            assert!(!block_transaction_payloads
                .lock()
                .contains_key(&pipelined_block.id()));
        }

        // Remove all the blocks from the block payload store (including some that don't exist)
        block_payload_store.remove_blocks(&pipelined_blocks[0..10]);

        // Check that the block payload store no longer contains any blocks
        let block_transaction_payloads = block_payload_store.get_block_payloads();
        assert!(block_transaction_payloads.lock().is_empty());
    }

    /// Creates and adds the given number of blocks to the block payload store
    fn create_and_add_blocks_to_store(
        mut block_payload_store: BlockPayloadStore,
        num_blocks: usize,
    ) -> Vec<Arc<PipelinedBlock>> {
        let mut pipelined_blocks = vec![];
        for i in 0..num_blocks {
            // Create the block info
            let block_info = BlockInfo::new(
                i as u64,
                i as Round,
                HashValue::random(),
                HashValue::random(),
                i as Version,
                i as u64,
                None,
            );

            // Insert the block payload into the store
            block_payload_store.insert_block_payload(block_info.clone(), vec![], Some(i));

            // Create the equivalent pipelined block
            let block_data = BlockData::new_for_testing(
                block_info.epoch(),
                block_info.round(),
                block_info.timestamp_usecs(),
                QuorumCert::dummy(),
                BlockType::Genesis,
            );
            let block = Block::new_for_testing(block_info.id(), block_data, None);
            let pipelined_block = Arc::new(PipelinedBlock::new_ordered(block));

            // Add the pipelined block to the list
            pipelined_blocks.push(pipelined_block.clone());
        }

        pipelined_blocks
    }

    /// Marks the payload of the given block ID as requested and returns the receiver
    fn mark_payload_as_requested(
        block_payload_store: BlockPayloadStore,
        block_id: HashValue,
    ) -> oneshot::Receiver<BlockTransactionPayload> {
        // Get the block payload entry for the given block ID
        let block_payloads = block_payload_store.get_block_payloads();
        let mut block_payloads = block_payloads.lock();
        let block_payload = block_payloads.get_mut(&block_id).unwrap();

        // Mark the block payload as requested
        let (payload_sender, payload_receiver) = oneshot::channel();
        *block_payload = BlockPayloadStatus::Requested(payload_sender);

        // Return the payload receiver
        payload_receiver
    }
}
