// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use aptos_types::randomness::RandMetadata;

impl From<&Block> for RandMetadata {
    fn from(block: &Block) -> Self {
        Self::new(
            block.epoch(),
            block.round(),
            block.id(),
            block.timestamp_usecs(),
        )
    }
}
