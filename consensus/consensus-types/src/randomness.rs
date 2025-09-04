// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use velor_types::randomness::FullRandMetadata;

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
