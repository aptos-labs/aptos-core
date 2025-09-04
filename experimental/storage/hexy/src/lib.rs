// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod hashing;
pub mod in_mem;
mod metrics;
#[cfg(test)]
pub(crate) mod tests;
pub mod utils;

pub const ARITY: usize = 16;

pub type LeafIdx = u32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodePosition {
    level_height: u8,
    index_in_level: u32,
}

impl NodePosition {
    const LEAF_LEVEL_HEIGHT: u8 = 0;

    pub fn leaf(index: LeafIdx) -> Self {
        Self {
            index_in_level: index,
            level_height: Self::LEAF_LEVEL_HEIGHT,
        }
    }

    pub fn height_and_index(level_height: usize, index_in_level: usize) -> Self {
        Self {
            index_in_level: index_in_level as u32,
            level_height: level_height as u8,
        }
    }

    pub fn parent_index_in_level(&self) -> u32 {
        self.index_in_level / ARITY as u32
    }

    pub fn index_in_siblings(&self) -> usize {
        self.index_in_level as usize % ARITY
    }

    pub fn index_in_level(&self) -> usize {
        self.index_in_level as usize
    }

    pub fn parent(&self) -> Self {
        Self {
            index_in_level: self.parent_index_in_level(),
            level_height: self.level_height + 1,
        }
    }

    pub fn child(&self, idx_in_siblings: usize) -> Self {
        Self {
            index_in_level: self.index_in_level * ARITY as u32 + idx_in_siblings as u32,
            level_height: self.level_height - 1,
        }
    }
}
