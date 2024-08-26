// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, NodePosition, ARITY};
use anyhow::{ensure, Result};
use aptos_crypto::{hash::HOT_STATE_PLACE_HOLDER_HASH, HashValue};
use aptos_experimental_layered_map::LayeredMap;
use aptos_metrics_core::TimerHelper;

struct BigVector<T> {
    chunks: Vec<Vec<T>>,
}

impl<T> BigVector<T>
where
    T: Clone,
{
    const CHUNK_SIZE: usize = 1024 * 1024;

    pub fn allocate(template: &T, size: usize) -> Self {
        let mut chunks = vec![];
        let mut remainder = size;
        while remainder > 0 {
            let chunk_size = Self::CHUNK_SIZE.min(remainder);
            chunks.push(vec![template.clone(); chunk_size]);
            remainder -= chunk_size;
        }
        Self { chunks }
    }

    pub fn len(&self) -> usize {
        if let Some(last_chunk) = self.chunks.last() {
            (self.chunks.len() - 1) * Self::CHUNK_SIZE + last_chunk.len()
        } else {
            0
        }
    }

    pub fn expect(&self, index: usize) -> &T {
        let chunk = index / Self::CHUNK_SIZE;
        let offset = index % Self::CHUNK_SIZE;
        &self.chunks[chunk][offset]
    }

    pub fn expect_mut(&mut self, index: usize) -> &mut T {
        let chunk = index / Self::CHUNK_SIZE;
        let offset = index % Self::CHUNK_SIZE;
        &mut self.chunks[chunk][offset]
    }
}

pub struct HexyBase {
    levels_by_height: Vec<BigVector<HashValue>>,
}

impl HexyBase {
    pub fn allocate(num_leaves: u32) -> Self {
        assert!(num_leaves > 0);

        let mut levels_by_height = vec![];
        let mut level_size = num_leaves as usize;
        loop {
            let level = BigVector::allocate(&*HOT_STATE_PLACE_HOLDER_HASH, level_size);
            levels_by_height.push(level);

            if level_size == 1 {
                break;
            } else {
                level_size = level_size / ARITY + (level_size % ARITY != 0) as usize;
            }
        }

        Self { levels_by_height }
    }

    pub fn num_leaves(&self) -> usize {
        self.levels_by_height[0].len()
    }

    pub fn height(&self) -> usize {
        self.levels_by_height.len()
    }

    pub fn height_u8(&self) -> u8 {
        self.height() as u8
    }

    pub fn get_hash(&self, position: NodePosition) -> Result<HashValue> {
        ensure!(
            position.level_height < self.height_u8(),
            "level_height out of bound. num_of_leaves: {:?}, requested position: {:?}",
            self.num_leaves(),
            position,
        );

        let level = self.expect_level(position);
        if position.index_in_level < level.len() as u32 {
            Ok(*level.expect(position.index_in_level as usize))
        } else {
            ensure!(
                position.level_height < self.height_u8(),
                "index_in_level out of bound. num_of_leaves: {:?}, requested position: {:?}",
                self.num_leaves(),
                position,
            );

            let parent_position = position.parent();
            let parent_level = self.expect_level(parent_position);

            ensure!(
                parent_position.index_in_level < parent_level.len() as u32,
                "index_in_level out of bound. num_of_leaves: {:?}, requested position: {:?}",
                self.num_leaves(),
                position,
            );

            Ok(*HOT_STATE_PLACE_HOLDER_HASH)
        }
    }

    pub fn get_hash_mut(&mut self, position: NodePosition) -> Result<&mut HashValue> {
        let num_leaves = self.num_leaves();
        ensure!(
            position.level_height < self.height_u8(),
            "level_height out of bound. num_of_leaves: {:?}, requested position: {:?}",
            num_leaves,
            position,
        );
        let level = self.expect_level_mut(position);
        ensure!(
            position.index_in_level < level.len() as u32,
            "index_in_level out of bound. num_of_leaves: {:?}, requested position: {:?}",
            num_leaves,
            position,
        );

        Ok(level.expect_mut(position.index_in_level as usize))
    }

    fn expect_level(&self, position: NodePosition) -> &BigVector<HashValue> {
        &self.levels_by_height[position.level_height as usize]
    }

    fn expect_level_mut(&mut self, position: NodePosition) -> &mut BigVector<HashValue> {
        &mut self.levels_by_height[position.level_height as usize]
    }

    pub fn expect_hash(&self, position: NodePosition) -> HashValue {
        self.get_hash(position).expect("Failed to get hash.")
    }

    pub fn root_position(&self) -> NodePosition {
        NodePosition {
            level_height: self.height_u8() - 1,
            index_in_level: 0,
        }
    }

    pub fn root_hash(&self) -> HashValue {
        self.levels_by_height
            .last()
            .map_or(*HOT_STATE_PLACE_HOLDER_HASH, |level| *level.expect(0))
    }

    pub fn merge(&self, overlay: LayeredMap<NodePosition, HashValue>) -> Result<()> {
        let _timer = TIMER.timer_with(&["merge"]);

        for (position, hash) in overlay.iter() {
            unsafe {
                let raw_self = self as *const Self as *mut Self;
                let mut_self = raw_self.as_mut().expect("self is null.");
                *(mut_self.get_hash_mut(position)?) = hash;
            }
        }
        Ok(())
    }
}
