// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::network::observer_message::{OrderedBlock, OrderedBlockWithWindow};

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
