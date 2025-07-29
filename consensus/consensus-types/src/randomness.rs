// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use aptos_types::{decryption::{DecMetadata, Round}, randomness::FullRandMetadata};

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

// impl From<&Block> for DecMetadata {
//     fn from(block: &Block) -> Self {
//         DecMetadata::new(block.epoch(), block.round() as Round, block.timestamp_usecs(), block.id())
//     }
// }
