// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::consensus_observer::network::observer_message::ExecutionPoolWindow;
use crate::consensus_observer::{
    common::error::Error,
    network::observer_message::{OrderedBlock, OrderedBlockWithWindow},
    observer::pending_blocks::PendingBlockStore,
};
use aptos_infallible::Mutex;
#[cfg(test)]
use rand::{rngs::OsRng, Rng};
use std::sync::Arc;

/// A simple enum wrapper that holds an observed ordered block, allowing
/// self-contained ordered blocks and ordered blocks with execution pool windows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedOrderedBlock {
    Ordered(OrderedBlock),
    OrderedWithWindow(OrderedBlockWithWindow),
}

impl ObservedOrderedBlock {
    /// Creates a new observed ordered block
    pub fn new(ordered_block: OrderedBlock) -> Self {
        Self::Ordered(ordered_block)
    }

    /// Creates a new observed ordered block with window
    pub fn new_with_window(ordered_block_with_window: OrderedBlockWithWindow) -> Self {
        Self::OrderedWithWindow(ordered_block_with_window)
    }

    #[cfg(test)]
    /// Creates a new observed ordered block for testing.
    /// Note: the observed type is determined randomly.
    pub fn new_for_testing(ordered_block: OrderedBlock) -> Self {
        if OsRng.gen::<u8>() % 2 == 0 {
            ObservedOrderedBlock::new(ordered_block.clone())
        } else {
            let ordered_block_with_window = OrderedBlockWithWindow::new(
                ordered_block.clone(),
                ExecutionPoolWindow::new(vec![]),
            );
            ObservedOrderedBlock::new_with_window(ordered_block_with_window)
        }
    }

    /// Consumes the observed ordered block and returns the inner ordered block
    pub fn consume_ordered_block(self) -> OrderedBlock {
        match self {
            Self::Ordered(ordered_block) => ordered_block,
            Self::OrderedWithWindow(ordered_block_with_window) => {
                let (ordered_block, _) = ordered_block_with_window.into_parts();
                ordered_block
            },
        }
    }

    /// Returns a reference to the inner ordered block
    pub fn ordered_block(&self) -> &OrderedBlock {
        match self {
            Self::Ordered(ordered_block) => ordered_block,
            Self::OrderedWithWindow(ordered_block_with_window) => {
                ordered_block_with_window.ordered_block()
            },
        }
    }
}

/// Returns all ordered blocks for the given ordered block with window. This
/// requires traversing backward via the parent links to identify and fetch
/// the blocks from the pending block store. The blocks are returned in
/// chronological order, and if any block is missing, this will return an error.
pub fn get_all_blocks_for_window(
    pending_block_store: Arc<Mutex<PendingBlockStore>>,
    ordered_block: &OrderedBlock,
    window_size: u64,
) -> Result<Vec<OrderedBlock>, Error> {
    // If the window size is 0, something is wrong!
    if window_size == 0 {
        return Err(Error::UnexpectedError(format!(
            "Execution pool window size is 0 for ordered block with window: {:?}",
            ordered_block.proof_block_info()
        )));
    }

    // Ensure the ordered block only has one inner block (multiple inner
    // blocks are not supported and will have already been dropped!).
    let num_inner_blocks = ordered_block.blocks().len();
    if num_inner_blocks != 1 {
        return Err(Error::UnexpectedError(format!(
            "Found ordered block with multiple ({}) inner blocks! First block epoch: {}, round: {}",
            num_inner_blocks,
            ordered_block.first_block().epoch(),
            ordered_block.first_block().round()
        )));
    }

    // Identify the window boundary (i.e., the lowest round that falls inside the window)
    let ordered_block_round = ordered_block.first_block().round();
    let window_boundary_round = ordered_block_round
        .saturating_add(1)
        .saturating_sub(window_size);

    // Add the current block to the window of all ordered blocks
    let mut all_ordered_blocks = vec![ordered_block.clone()];

    // Collect all ordered blocks for the window
    let mut current_block = ordered_block.clone();
    let mut remaining_window_size = window_size.saturating_sub(1);
    while remaining_window_size > 0 {
        // If the current block round is 0, break (we can't go further back)
        let first_block = current_block.first_block();
        if first_block.round() == 0 {
            break; // We collected as many blocks as possible
        }

        // Get the parent block for the current block
        let parent_block_id = first_block.parent_id();
        let parent_block = match pending_block_store
            .lock()
            .get_pending_block_by_hash(parent_block_id)
        {
            Some(pending_block) => pending_block,
            None => {
                // The parent block is missing
                return Err(Error::MissingBlockError(format!(
                    "Missing parent block (ID: {:?}) for ordered block with window: {:?}",
                    parent_block_id,
                    current_block.proof_block_info()
                )));
            },
        };

        // If the parent block is outside the window boundary, break
        let parent_ordered_block = parent_block.ordered_block();
        if parent_ordered_block.first_block().round() < window_boundary_round {
            break; // We collected as many blocks as possible
        }

        // Append the parent block to the list of ordered blocks
        all_ordered_blocks.push(parent_ordered_block.clone());

        // Update the current block and window size
        current_block = parent_ordered_block.clone();
        remaining_window_size = remaining_window_size.saturating_sub(1);
    }

    // Reverse the list of ordered blocks to return them in chronological order
    all_ordered_blocks.reverse();

    // Return the list of ordered blocks
    Ok(all_ordered_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus_observer::observer::pending_blocks::PendingBlockWithMetadata;
    use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        pipelined_block::PipelinedBlock,
        quorum_cert::QuorumCert,
        vote_data::VoteData,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::{BlockInfo, Round},
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    };
    use claims::assert_matches;
    use std::time::Instant;

    #[test]
    fn test_get_all_blocks_for_window_round_zero() {
        // Create a new pending block store
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store (starting from round 0)
        let current_epoch = 10;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
        );

        // Identify the ordered block at round 0
        let ordered_block_round_zero = pending_blocks[0].clone();

        // Fetch all blocks for a window size of 0, and ensure it returns an error
        let error =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_zero, 0)
                .unwrap_err();
        assert_matches!(error, Error::UnexpectedError(_));

        // Fetch all blocks for a window size of 1, and ensure it returns round 0
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_zero, 1);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_zero.clone()
        ]);

        // Fetch all blocks for a window size of 10, and ensure it returns only round 0
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_zero, 10);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_zero.clone()
        ]);
    }

    #[test]
    fn test_get_all_blocks_for_window_higher_rounds() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store (starting from round 0)
        let current_epoch = 10;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
        );

        // Identify the ordered block at round 3
        let ordered_block_round_three = pending_blocks[3].clone();

        // Fetch all blocks for a window size of 0, and ensure it returns an error
        let error =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 0)
                .unwrap_err();
        assert_matches!(error, Error::UnexpectedError(_));

        // Fetch all blocks for a window size of 1, and ensure it returns round 3
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 1);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_three.clone()
        ]);

        // Fetch all blocks for a window size of 2, and ensure it returns rounds 2 to 3
        let ordered_block_round_two = pending_blocks[2].clone();
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 2);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_two.clone(),
            ordered_block_round_three.clone()
        ]);

        // Fetch all blocks for a window size of 3, and ensure it returns rounds 1 to 3
        let ordered_block_round_one = pending_blocks[1].clone();
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 3);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_one.clone(),
            ordered_block_round_two.clone(),
            ordered_block_round_three.clone()
        ]);

        // Fetch all blocks for a window size of 4, and ensure it returns rounds 0 to 3
        let ordered_block_round_zero = pending_blocks[0].clone();
        let first_four_ordered_blocks = vec![
            ordered_block_round_zero.clone(),
            ordered_block_round_one.clone(),
            ordered_block_round_two.clone(),
            ordered_block_round_three.clone(),
        ];
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 4);
        assert_eq!(all_ordered_blocks.unwrap(), first_four_ordered_blocks);

        // Fetch all blocks for a window size of 5, and ensure it returns rounds 0 to 3
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 5);
        assert_eq!(all_ordered_blocks.unwrap(), first_four_ordered_blocks);

        // Fetch all blocks for a window size of 10, and ensure it returns rounds 0 to 3
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_three, 10);
        assert_eq!(all_ordered_blocks.unwrap(), first_four_ordered_blocks);
    }

    #[test]
    fn test_get_all_blocks_for_window_missing_blocks() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert blocks into the store from rounds 0 to 2
        let current_epoch = 10;
        let starting_round = 0;
        let mut pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            3,
            current_epoch,
            starting_round,
        );

        // Insert blocks into the store from rounds 4 to 9 (missing round 3)
        let additional_pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            6,
            current_epoch,
            starting_round + 4,
        );
        pending_blocks.extend_from_slice(&additional_pending_blocks);

        // Identify the ordered block at round 5
        let ordered_block_round_five = pending_blocks[4].clone();

        // Fetch all blocks for a window size of 1, and ensure it returns round 5
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 1);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_five.clone()
        ]);

        // Fetch all blocks for a window size of 2, and ensure it returns round 4 and 5
        let ordered_block_round_four = pending_blocks[3].clone();
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 2);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_four.clone(),
            ordered_block_round_five.clone()
        ]);

        // Fetch all blocks for a window size of 3, and ensure it returns an error (round 3 is missing)
        let errorr =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 3)
                .unwrap_err();
        assert_matches!(errorr, Error::MissingBlockError(_));
    }

    #[test]
    fn test_get_all_blocks_for_window_missing_zero() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert blocks into the store from rounds 1 to 10
        let current_epoch = 10;
        let starting_round = 1;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            10,
            current_epoch,
            starting_round,
        );

        // Identify the ordered block at round 5
        let ordered_block_round_five = pending_blocks[4].clone();

        // Fetch all blocks for a window size of 1, and ensure it returns round 5
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 1);
        assert_eq!(all_ordered_blocks.unwrap(), vec![
            ordered_block_round_five.clone()
        ]);

        // Fetch all blocks for a window size of 5, and ensure it returns round 1 to 5
        let first_five_ordered_blocks = pending_blocks[0..5].to_vec();
        let all_ordered_blocks =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 5);
        assert_eq!(all_ordered_blocks.unwrap(), first_five_ordered_blocks);

        // Fetch all blocks for a window size of 6, and ensure it returns an error (round 0 is missing)
        let error =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 6)
                .unwrap_err();
        assert_matches!(error, Error::MissingBlockError(_));

        // Fetch all blocks for a window size of 10, and ensure it returns an error (round 0 is missing)
        let error =
            get_all_blocks_for_window(pending_block_store.clone(), &ordered_block_round_five, 10)
                .unwrap_err();
        assert_matches!(error, Error::MissingBlockError(_));
    }

    #[test]
    fn test_get_all_blocks_for_window_missing_rounds() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Create an ordered block at round 0
        let current_epoch = 100;
        let starting_round = 0;
        let ordered_block_round_zero =
            create_ordered_block(current_epoch, starting_round, 0, BlockInfo::empty());

        // Create an ordered block at round 3 (linked to round 0)
        let ordered_block_round_three = create_ordered_block(
            current_epoch,
            starting_round,
            3,
            ordered_block_round_zero.first_block().block_info(),
        );

        // Create an ordered block at round 5 (linked to round 3)
        let ordered_block_round_five = create_ordered_block(
            current_epoch,
            starting_round,
            5,
            ordered_block_round_three.first_block().block_info(),
        );

        // Create an ordered block at round 9 (linked to round 5)
        let ordered_block_round_nine = create_ordered_block(
            current_epoch,
            starting_round,
            9,
            ordered_block_round_five.first_block().block_info(),
        );

        // Insert the ordered blocks into the pending block store
        add_ordered_block_to_store(pending_block_store.clone(), &ordered_block_round_zero);
        add_ordered_block_to_store(pending_block_store.clone(), &ordered_block_round_three);
        add_ordered_block_to_store(pending_block_store.clone(), &ordered_block_round_five);
        add_ordered_block_to_store(pending_block_store.clone(), &ordered_block_round_nine);

        // Fetch all blocks at round 9, with window sizes of 1 to 4, and ensure they return round 9
        for window_size in 1..=4 {
            let all_ordered_blocks = get_all_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            assert_eq!(all_ordered_blocks.unwrap(), vec![
                ordered_block_round_nine.clone()
            ]);
        }

        // Fetch all blocks at round 9, with window sizes of 5 to 6, and ensure they return rounds 5 and 9
        for window_size in 5..=6 {
            let all_ordered_blocks = get_all_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            assert_eq!(all_ordered_blocks.unwrap(), vec![
                ordered_block_round_five.clone(),
                ordered_block_round_nine.clone()
            ]);
        }

        // Fetch all blocks at round 9, with window sizes of 7 to 9, and ensure they return rounds 3, 5 and 9
        for window_size in 7..=9 {
            let all_ordered_blocks = get_all_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            assert_eq!(all_ordered_blocks.unwrap(), vec![
                ordered_block_round_three.clone(),
                ordered_block_round_five.clone(),
                ordered_block_round_nine.clone()
            ]);
        }

        // Fetch all blocks at round 9, with window sizes of 10 to 100, and ensure they return rounds 0, 3, 5 and 9
        for window_size in 10..=100 {
            let all_ordered_blocks = get_all_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            assert_eq!(all_ordered_blocks.unwrap(), vec![
                ordered_block_round_zero.clone(),
                ordered_block_round_three.clone(),
                ordered_block_round_five.clone(),
                ordered_block_round_nine.clone()
            ]);
        }
    }

    /// Creates and adds the specified number of blocks to the pending block store
    fn create_and_add_pending_blocks(
        pending_block_store: Arc<Mutex<PendingBlockStore>>,
        num_pending_blocks: usize,
        epoch: u64,
        starting_round: Round,
    ) -> Vec<OrderedBlock> {
        let mut pending_blocks = vec![];
        for i in 0..num_pending_blocks {
            // Identify the parent block info for the ordered block
            let parent_block_info = if i == 0 {
                BlockInfo::empty() // No block parent for the first block
            } else {
                let parent_ordered_block: &OrderedBlock = pending_blocks.iter().last().unwrap();
                parent_ordered_block.first_block().block_info()
            };

            // Create an ordered block
            let ordered_block = create_ordered_block(epoch, starting_round, i, parent_block_info);

            // Add the ordered block to the pending block store
            add_ordered_block_to_store(pending_block_store.clone(), &ordered_block);

            // Add the ordered block to the pending blocks
            pending_blocks.push(ordered_block);
        }

        pending_blocks
    }

    /// Adds the given ordered block to the pending block store
    fn add_ordered_block_to_store(
        pending_block_store: Arc<Mutex<PendingBlockStore>>,
        ordered_block: &OrderedBlock,
    ) {
        // Create an observed ordered block
        let ordered_block_with_window =
            OrderedBlockWithWindow::new(ordered_block.clone(), ExecutionPoolWindow::new(vec![]));
        let observed_ordered_block =
            ObservedOrderedBlock::new_with_window(ordered_block_with_window);

        // Create a pending block with metadata
        let pending_block_with_metadata = PendingBlockWithMetadata::new_with_arc(
            PeerNetworkId::random(),
            Instant::now(),
            observed_ordered_block,
        );

        // Insert the ordered block into the pending block store
        pending_block_store
            .lock()
            .insert_pending_block(pending_block_with_metadata.clone());
    }

    /// Creates and returns an ordered block with the specified maximum number of pipelined blocks
    fn create_ordered_block(
        epoch: u64,
        starting_round: Round,
        block_index: usize,
        parent_block_info: BlockInfo,
    ) -> OrderedBlock {
        // Calculate the block round
        let round = starting_round + (block_index as Round);

        // Create a new block info
        let block_info = BlockInfo::new(
            epoch,
            round,
            HashValue::random(),
            HashValue::random(),
            round,
            block_index as u64,
            None,
        );

        // Create the quorum certificate for the block
        let vote_data = VoteData::new(parent_block_info.clone(), parent_block_info.clone());
        let ledger_info = LedgerInfo::new(block_info.clone(), HashValue::random());
        let quorum_cert = QuorumCert::new(
            vote_data,
            LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty()),
        );

        // Create a single pipelined block
        let block_data = BlockData::new_for_testing(
            block_info.epoch(),
            block_info.round(),
            block_info.timestamp_usecs(),
            quorum_cert,
            BlockType::Genesis,
        );
        let block = Block::new_for_testing(block_info.id(), block_data, None);
        let pipelined_block = Arc::new(PipelinedBlock::new_ordered(block));

        // Create and return an ordered block
        let ordered_proof = LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, starting_round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        );
        OrderedBlock::new(vec![pipelined_block], ordered_proof.clone())
    }
}
