// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::block::Block;
use aptos_types::randomness::FullRandMetadata;

impl From<&Block> for FullRandMetadata {
    fn from(block: &Block) -> Self {
        Self::new(
            block.epoch(),
            block.round(),
            block.id(),
            block.timestamp_usecs(),
        )
    }
}
