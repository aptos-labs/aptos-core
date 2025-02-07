// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::consensus_observer::network::observer_message::ExecutionPoolWindow;
use crate::consensus_observer::{
    common::logging::{LogEntry, LogSchema},
    network::observer_message::{OrderedBlock, OrderedBlockWithWindow},
    observer::pending_blocks::PendingBlockStore,
};
use aptos_infallible::Mutex;
use aptos_logger::{error, warn};
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

/// Returns all ordered blocks for the given ordered block with window.
/// This requires traversing backward via the parent links to identify
/// and fetch the blocks from the pending block store. The blocks are
/// returned in chronological order, and if any block is missing, this
/// will return None.
// TODO: this doesn't support ordered blocks with multiple inner blocks
pub fn get_all_blocks_for_window(
    pending_block_store: Arc<Mutex<PendingBlockStore>>,
    ordered_block: &OrderedBlock,
    window_size: usize,
) -> Option<Vec<OrderedBlock>> {
    // If the window size is 0, something is wrong. Log the error and return.
    if window_size == 0 {
        error!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Execution pool window size is 0 for ordered block with window: {:?}",
                ordered_block.proof_block_info()
            ))
        );
        return None;
    }

    // Identify the window boundary (i.e., the highest round that falls outside the window)
    let ordered_block_round = ordered_block.first_block().round();
    let window_boundary_round = ordered_block_round.saturating_sub(window_size as u64);

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
                // Log the missing block and return
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Missing parent block (ID: {:?}) for ordered block with window: {:?}",
                        parent_block_id,
                        current_block.proof_block_info()
                    ))
                );
                return None; // The parent block is missing!
            },
        };

        // If the parent block is outside the window boundary, break
        let parent_ordered_block = parent_block.ordered_block();
        if parent_ordered_block.first_block().round() <= window_boundary_round {
            break; // We collected as many blocks as possible
        }

        // Append the parent block to the list of ordered blocks
        all_ordered_blocks.push(parent_ordered_block.clone());

        // Update the current block and window size
        current_block = parent_ordered_block.clone();
        remaining_window_size = window_size.saturating_sub(1);
    }

    // Reverse the list of ordered blocks to return them in chronological order
    all_ordered_blocks.reverse();

    // Return the list of ordered blocks
    Some(all_ordered_blocks)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_zero_window_size() {
        panic!("Not implemented");
    }
}
