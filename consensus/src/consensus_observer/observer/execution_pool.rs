// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::error::Error,
        network::observer_message::{OrderedBlock, OrderedBlockWithWindow},
        observer::pending_blocks::PendingBlockStore,
    },
    util,
};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_infallible::Mutex;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
#[cfg(test)]
use rand::{rngs::OsRng, Rng};
use std::{ops::Deref, sync::Arc};

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
            let ordered_block_with_window = OrderedBlockWithWindow::new(ordered_block.clone());
            ObservedOrderedBlock::new_with_window(ordered_block_with_window)
        }
    }

    /// Consumes the observed ordered block and returns the inner ordered block
    pub fn consume_ordered_block(self) -> OrderedBlock {
        match self {
            Self::Ordered(ordered_block) => ordered_block,
            Self::OrderedWithWindow(ordered_block_with_window) => {
                ordered_block_with_window.consume_ordered_block()
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

/// Calculates the epoch and round indices to remove for the given commit.
/// This is expected to be used with the split_off() method on a BTreeMap
/// indexed by (epoch, round) to remove old blocks and payloads.
///
/// The (epoch, round) indices returned by this function are calculated as follows:
/// - If the execution pool window size is None, all indices up to
/// (and including) the epoch and round of the commit will be removed.
/// - Otherwise, a buffer of indices preceding the commit will be retained
/// (to ensure we have enough entries to satisfy the execution window).
pub fn calculate_epoch_round_split_for_commit(
    commit_ledger_info: &LedgerInfoWithSignatures,
    execution_pool_window_size: Option<u64>,
    window_buffer_multiplier: u64,
) -> (u64, u64) {
    // Determine the epoch to split off (execution pool doesn't buffer across epochs)
    let split_off_epoch = commit_ledger_info.ledger_info().epoch();

    // Determine the round to split off
    let commit_round = commit_ledger_info.ledger_info().round();
    let split_off_round = if let Some(window_size) = execution_pool_window_size {
        let window_buffer_size = window_size * window_buffer_multiplier;
        if commit_round < window_buffer_size {
            0 // Clear everything from previous epochs
        } else {
            // Retain all payloads in the window buffer
            commit_round
                .saturating_sub(window_buffer_size)
                .saturating_add(1)
        }
    } else {
        // Execution pool is disabled. Remove everything up to (and including) the commit round.
        commit_round.saturating_add(1)
    };

    (split_off_epoch, split_off_round)
}

/// Returns all pipelined blocks for the given block with window. This
/// requires traversing backward via the parent links to identify and fetch
/// the blocks from the pending block store.
///
/// The blocks are returned in chronological (ascending) order, and the given
/// ordered block is included in the returned list. If any block is missing,
/// this function will return an error.
pub fn get_all_pipelined_blocks_for_window(
    pending_block_store: Arc<Mutex<PendingBlockStore>>,
    ordered_block: &OrderedBlock,
    window_size: u64,
) -> Result<Vec<Arc<PipelinedBlock>>, Error> {
    // If the window size is 0, something is wrong!
    if window_size == 0 {
        return Err(Error::UnexpectedError(format!(
            "Execution pool window size is 0 for ordered block with window: {:?}",
            ordered_block.proof_block_info()
        )));
    }

    // Ensure the ordered block only has one inner pipelined block (multiple
    // inner blocks are not supported and will have already been dropped!).
    let num_inner_blocks = ordered_block.blocks().len();
    if num_inner_blocks != 1 {
        return Err(Error::UnexpectedError(format!(
            "Found ordered block with multiple ({}) inner blocks! First block epoch: {}, round: {}",
            num_inner_blocks,
            ordered_block.first_block().epoch(),
            ordered_block.first_block().round()
        )));
    }

    // Get the inner pipelined block from the ordered block
    let pipelined_block = match ordered_block.blocks().first() {
        Some(pipelined_block) => pipelined_block.clone(),
        None => {
            return Err(Error::UnexpectedError(format!(
                "Failed to get first block for ordered block with window: {:?}",
                ordered_block.proof_block_info()
            )));
        },
    };

    // If the window size is 1, return the current block only
    if window_size == 1 {
        return Ok(vec![pipelined_block]);
    }

    // Otherwise, fetch the ordered block window (excluding the current block)
    let pending_block_store = pending_block_store.lock();
    let mut block_window = util::get_block_window_from_storage(
        Arc::new(pending_block_store.deref()),
        pipelined_block.block(),
        window_size,
    )
    .map_err(|error| {
        Error::MissingBlockError(format!(
            "Failed to get block window for ordered block with window: {:?}. Error: {:?}",
            ordered_block.proof_block_info(),
            error
        ))
    })?;

    // Append the current block to the end of the block window
    block_window.push(pipelined_block);

    Ok(block_window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus_observer::observer::pending_blocks::PendingBlockWithMetadata;
    use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        common::{Author, Payload},
        pipelined_block::{OrderedBlockWindow, PipelinedBlock},
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
    fn test_calculate_epoch_round_split_for_commit() {
        // Verify the epoch and round splits for a window size of None
        for (epoch, round, expected_epoch_split, expected_round_split) in [
            (0, 0, 0, 1),
            (1, 1, 1, 2),
            (10, 100, 10, 101),
            (100, 1000, 100, 1001),
        ] {
            verify_epoch_round_split(
                epoch,
                round,
                None, // Window size is None
                1,
                expected_epoch_split,
                expected_round_split,
            );
        }

        // Verify the epoch and round splits for a buffer size of 10 (window = 10, multiplier = 1)
        for (epoch, round, expected_epoch_split, expected_round_split) in [
            (0, 0, 0, 0),
            (10, 9, 10, 0),
            (10, 10, 10, 1),
            (10, 11, 10, 2),
            (20, 100, 20, 91),
            (100, 1000, 100, 991),
        ] {
            verify_epoch_round_split(
                epoch,
                round,
                Some(10), // Window size is 10
                1,        // Buffer multiplier is 1
                expected_epoch_split,
                expected_round_split,
            );
        }

        // Verify the epoch and round splits for a buffer size of 30 (window = 6, multiplier = 5)
        for (epoch, round, expected_epoch_split, expected_round_split) in [
            (0, 0, 0, 0),
            (1, 29, 1, 0),
            (1, 30, 1, 1),
            (1, 31, 1, 2),
            (35, 100, 35, 71),
            (10, 1000, 10, 971),
        ] {
            verify_epoch_round_split(
                epoch,
                round,
                Some(6), // Window size is 6
                5,       // Buffer multiplier is 5
                expected_epoch_split,
                expected_round_split,
            );
        }
    }

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
        let error = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_zero,
            0,
        )
        .unwrap_err();
        assert_matches!(error, Error::UnexpectedError(_));

        // Fetch all blocks for a window size of 1, and ensure it returns round 0
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_zero,
            1,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![ordered_block_round_zero.clone()]);

        // Fetch all blocks for a window size of 10, and ensure it returns only round 0
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_zero,
            10,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![ordered_block_round_zero]);
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
        let error = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            0,
        )
        .unwrap_err();
        assert_matches!(error, Error::UnexpectedError(_));

        // Fetch all blocks for a window size of 1, and ensure it returns round 3
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            1,
        );
        verify_pipeline_block_window(
            all_pipelined_blocks,
            vec![ordered_block_round_three.clone()],
        );

        // Fetch all blocks for a window size of 2, and ensure it returns rounds 2 to 3
        let ordered_block_round_two = pending_blocks[2].clone();
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            2,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![
            ordered_block_round_two.clone(),
            ordered_block_round_three.clone(),
        ]);

        // Fetch all blocks for a window size of 3, and ensure it returns rounds 1 to 3
        let ordered_block_round_one = pending_blocks[1].clone();
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            3,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![
            ordered_block_round_one.clone(),
            ordered_block_round_two.clone(),
            ordered_block_round_three.clone(),
        ]);

        // Fetch all blocks for a window size of 4, and ensure it returns rounds 0 to 3
        let ordered_block_round_zero = pending_blocks[0].clone();
        let first_four_ordered_blocks = vec![
            ordered_block_round_zero.clone(),
            ordered_block_round_one.clone(),
            ordered_block_round_two.clone(),
            ordered_block_round_three.clone(),
        ];
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            4,
        );
        verify_pipeline_block_window(all_pipelined_blocks, first_four_ordered_blocks.clone());

        // Fetch all blocks for a window size of 5, and ensure it returns rounds 0 to 3
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            5,
        );
        verify_pipeline_block_window(all_pipelined_blocks, first_four_ordered_blocks.clone());

        // Fetch all blocks for a window size of 10, and ensure it returns rounds 0 to 3
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_three,
            10,
        );
        verify_pipeline_block_window(all_pipelined_blocks, first_four_ordered_blocks.clone());
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

        // Insert blocks into the store from rounds 0 to 9
        let current_epoch = 10;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            10,
            current_epoch,
            starting_round,
        );

        // Remove the block at round 3
        let ordered_block_round_three = pending_blocks[3].clone();
        pending_block_store
            .lock()
            .remove_pending_block(&ordered_block_round_three);

        // Identify the ordered block at round 5
        let ordered_block_round_five = pending_blocks[5].clone();

        // Fetch all blocks for a window size of 1, and ensure it returns round 5
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            1,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![ordered_block_round_five.clone()]);

        // Fetch all blocks for a window size of 2, and ensure it returns round 4 and 5
        let ordered_block_round_four = pending_blocks[4].clone();
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            2,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![
            ordered_block_round_four.clone(),
            ordered_block_round_five.clone(),
        ]);

        // Fetch all blocks for a window size of 3, and ensure it returns an error (round 3 is missing)
        let error = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            3,
        )
        .unwrap_err();
        assert_matches!(error, Error::MissingBlockError(_));

        // Fetch all blocks for a window size of 3, and ensure it returns an error (round 3 is missing)
        let error = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            4,
        )
        .unwrap_err();
        assert_matches!(error, Error::MissingBlockError(_));
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
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            1,
        );
        verify_pipeline_block_window(all_pipelined_blocks, vec![ordered_block_round_five.clone()]);

        // Fetch all blocks for a window size of 5, and ensure it returns round 1 to 5
        let first_five_ordered_blocks: Vec<_> = pending_blocks[0..5].to_vec();
        let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            5,
        );
        verify_pipeline_block_window(all_pipelined_blocks, first_five_ordered_blocks);

        // Fetch all blocks for a window size of 6, and ensure it returns an error (round 0 is missing)
        let error = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            6,
        )
        .unwrap_err();
        assert_matches!(error, Error::MissingBlockError(_));

        // Fetch all blocks for a window size of 10, and ensure it returns an error (round 0 is missing)
        let error = get_all_pipelined_blocks_for_window(
            pending_block_store.clone(),
            &ordered_block_round_five,
            10,
        )
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
            let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            verify_pipeline_block_window(all_pipelined_blocks, vec![
                ordered_block_round_nine.clone()
            ]);
        }

        // Fetch all blocks at round 9, with window sizes of 5 to 6, and ensure they return rounds 5 and 9
        for window_size in 5..=6 {
            let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            verify_pipeline_block_window(all_pipelined_blocks, vec![
                ordered_block_round_five.clone(),
                ordered_block_round_nine.clone(),
            ]);
        }

        // Fetch all blocks at round 9, with window sizes of 7 to 9, and ensure they return rounds 3, 5 and 9
        for window_size in 7..=9 {
            let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            verify_pipeline_block_window(all_pipelined_blocks, vec![
                ordered_block_round_three.clone(),
                ordered_block_round_five.clone(),
                ordered_block_round_nine.clone(),
            ]);
        }

        // Fetch all blocks at round 9, with window sizes of 10 to 100, and ensure they return rounds 0, 3, 5 and 9
        for window_size in 10..=100 {
            let all_pipelined_blocks = get_all_pipelined_blocks_for_window(
                pending_block_store.clone(),
                &ordered_block_round_nine,
                window_size,
            );
            verify_pipeline_block_window(all_pipelined_blocks, vec![
                ordered_block_round_zero.clone(),
                ordered_block_round_three.clone(),
                ordered_block_round_five.clone(),
                ordered_block_round_nine.clone(),
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
        let ordered_block_with_window = OrderedBlockWithWindow::new(ordered_block.clone());
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

    /// Creates and returns a ledger info for the specified epoch and round
    fn create_ledger_info_for_epoch_round(epoch: u64, round: u64) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        )
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
        let block_type = if round == 0 {
            BlockType::Genesis
        } else {
            BlockType::Proposal {
                payload: Payload::DirectMempool(vec![]),
                author: Author::random(),
                failed_authors: vec![],
            }
        };
        let block_data = BlockData::new_for_testing(
            block_info.epoch(),
            block_info.round(),
            block_info.timestamp_usecs(),
            quorum_cert,
            block_type,
        );
        let block = Block::new_for_testing(block_info.id(), block_data, None);
        let pipelined_block = Arc::new(PipelinedBlock::new_ordered(
            block,
            OrderedBlockWindow::empty(),
        ));

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

    /// Verifies the given pipeline block window against
    /// the expected ordered block window.
    fn verify_pipeline_block_window(
        pipeline_blocks: Result<Vec<Arc<PipelinedBlock>>, Error>,
        expected_blocks: Vec<OrderedBlock>,
    ) {
        // Verify that both windows have the same number of entries
        let pipeline_blocks = pipeline_blocks.unwrap();
        assert_eq!(pipeline_blocks.len(), expected_blocks.len());

        // Verify the pipeline block window against the ordered block window
        for (pipelined_block, expected_block) in pipeline_blocks.iter().zip(expected_blocks.iter())
        {
            assert_eq!(
                pipelined_block.block().id(),
                expected_block.first_block().id()
            );
        }
    }

    /// Verifies the expected epoch and round splits for the given commit epoch and round
    fn verify_epoch_round_split(
        commit_epoch: u64,
        commit_round: u64,
        execution_pool_window_size: Option<u64>,
        window_buffer_multiplier: u64,
        expected_epoch_split: u64,
        expected_round_split: u64,
    ) {
        // Create a ledger info for the commit epoch and round
        let commit_ledger_info = create_ledger_info_for_epoch_round(commit_epoch, commit_round);

        // Calculate the epoch and round split
        let (split_off_epoch, split_off_round) = calculate_epoch_round_split_for_commit(
            &commit_ledger_info,
            execution_pool_window_size,
            window_buffer_multiplier,
        );

        // Verify that all split off indices match the expected values
        assert_eq!(split_off_epoch, expected_epoch_split);
        assert_eq!(split_off_round, expected_round_split);
    }
}
