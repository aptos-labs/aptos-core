// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, NodePosition, ARITY};
use anyhow::{ensure, Result};
use aptos_crypto::{hash::HOT_STATE_PLACE_HOLDER_HASH, HashValue};
use aptos_experimental_layered_map::LayeredMap;
use aptos_metrics_core::TimerHelper;
use std::{
    cell::UnsafeCell,
    sync::{atomic, atomic::Ordering},
};

struct BigVector<T> {
    chunks: Vec<Vec<UnsafeCell<T>>>,
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

            let mut chunk = Vec::with_capacity(chunk_size);
            chunk.resize_with(chunk_size, || UnsafeCell::new(template.clone()));
            chunks.push(chunk);

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

    pub fn expect_raw(&self, index: usize) -> *mut T {
        let chunk = index / Self::CHUNK_SIZE;
        let offset = index % Self::CHUNK_SIZE;
        self.chunks[chunk][offset].get()
    }

    // Another thread is possibly modifying the cell, explicit synchronization is needed through,
    // e.g. atomic::fence
    pub unsafe fn unsafe_expect(&self, index: usize) -> &T {
        unsafe { &*self.expect_raw(index) }
    }

    // The caller must guarantee the cell pointed by the index is not accessed while
    // it's mutated.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn unsafe_expect_mut(&self, index: usize) -> &mut T {
        unsafe { &mut *self.expect_raw(index) }
    }
}

pub struct HexyBase {
    levels_by_height: Vec<BigVector<HashValue>>,
}

/// N.B. Considerations have been taken with regard to reading and writing happening from different
/// threads.
unsafe impl Sync for HexyBase {}

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

    pub fn num_levels(&self) -> usize {
        self.levels_by_height.len()
    }

    pub fn num_levels_u8(&self) -> u8 {
        self.num_levels() as u8
    }

    pub(crate) unsafe fn unsafe_get_hash(&self, position: NodePosition) -> Result<HashValue> {
        ensure!(
            position.level_height < self.num_levels_u8(),
            "level_height out of bound. num_of_leaves: {:?}, requested position: {:?}",
            self.num_leaves(),
            position,
        );

        let level = self.expect_level(position);
        if position.index_in_level < level.len() as u32 {
            Ok(*level.unsafe_expect(position.index_in_level as usize))
        } else {
            let parent_position = position.parent();
            ensure!(
                parent_position.level_height < self.num_levels_u8(),
                "index_in_level out of bound for root level. num_of_leaves: {:?}, requested position: {:?}",
                self.num_leaves(),
                position,
            );
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

    unsafe fn unsafe_get_hash_mut(&self, position: NodePosition) -> Result<&mut HashValue> {
        let num_leaves = self.num_leaves();
        ensure!(
            position.level_height < self.num_levels_u8(),
            "level_height out of bound. num_of_leaves: {:?}, requested position: {:?}",
            num_leaves,
            position,
        );
        let level = self.expect_level(position);
        ensure!(
            position.index_in_level < level.len() as u32,
            "index_in_level out of bound. num_of_leaves: {:?}, requested position: {:?}",
            num_leaves,
            position,
        );

        Ok(level.unsafe_expect_mut(position.index_in_level as usize))
    }

    fn expect_level(&self, position: NodePosition) -> &BigVector<HashValue> {
        &self.levels_by_height[position.level_height as usize]
    }

    pub(crate) unsafe fn unsafe_expect_hash(&self, position: NodePosition) -> HashValue {
        self.unsafe_get_hash(position).expect("Failed to get hash.")
    }

    pub fn root_position(&self) -> NodePosition {
        NodePosition {
            level_height: self.num_levels_u8() - 1,
            index_in_level: 0,
        }
    }

    pub(crate) fn root_hash(&self) -> HashValue {
        atomic::fence(Ordering::Acquire);

        self.levels_by_height
            .last()
            .map_or(*HOT_STATE_PLACE_HOLDER_HASH, |level| unsafe {
                *level.unsafe_expect(0)
            })
    }

    // N.B. Any view that's older than the overlay being committed can return wrong hashes, since
    // the cells touched between `committed` and `to commit` will be mutated on the base.
    pub fn merge(&self, overlay: LayeredMap<NodePosition, HashValue>) -> Result<()> {
        let _timer = TIMER.timer_with(&["merge"]);

        // N.B. Assuming any valid view has all the cells being updated in a LayeredMap, we can
        // update those cells without locking.
        for (position, hash) in overlay.iter() {
            unsafe { *self.unsafe_get_hash_mut(position)? = hash }
        }

        // N.B. when a new view is constructed, an Acquire fence will be in place to make sure
        // updates before the committed overlay are synced over.
        // TODO(aldenhu): it's NOT necessary for a Release fence because we know the root
        //                overlay must be updated under a lock after the merge; but it'll be
        //                better the code is restructured so it's more obvious.
        // atomic::fence(Ordering::Release);

        Ok(())
    }
}
