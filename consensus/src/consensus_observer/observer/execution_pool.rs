// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::consensus_observer::network::observer_message::ExecutionPoolWindow;
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
