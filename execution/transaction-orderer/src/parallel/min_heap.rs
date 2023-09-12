// Copyright © Aptos Foundation
#![allow(dead_code)]

use rayon::prelude::*;
use std::{cmp::Reverse, collections::BinaryHeap};

pub struct MinHeap<T> {
    inner: BinaryHeap<Reverse<T>>,
}

impl<T> ParallelExtend<T> for MinHeap<T>
where
    T: Ord + Send,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        self.inner.par_extend(par_iter.into_par_iter().map(Reverse));
    }
}

impl<T: Ord> Default for MinHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> MinHeap<T> {
    pub fn new() -> Self {
        Self {
            inner: BinaryHeap::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.inner.push(Reverse(item));
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop().map(|Reverse(x)| x)
    }

    pub fn peek(&self) -> Option<&T> {
        self.inner.peek().map(|Reverse(x)| x)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T> IntoIterator for MinHeap<T> {
    type IntoIter =
        std::iter::Map<std::collections::binary_heap::IntoIter<Reverse<T>>, fn(Reverse<T>) -> T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter().map(|Reverse(x)| x)
    }
}

pub struct MinHeapWithRemove<T> {
    inner: MinHeap<T>,
    delayed_removals: MinHeap<T>,
}

impl<T: Ord> Default for MinHeapWithRemove<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> MinHeapWithRemove<T> {
    pub fn new() -> Self {
        Self {
            inner: MinHeap::new(),
            delayed_removals: MinHeap::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.inner.push(item);
    }

    pub fn peek(&self) -> Option<&T> {
        self.inner.peek()
    }

    pub fn pop(&mut self) -> Option<T> {
        let res = self.inner.pop();
        self.apply_delayed_removals();
        res
    }

    /// Removes an item from the heap.
    ///
    /// The caller is responsible to ensure that the item has previously been
    /// inserted into the heap and has not yet been removed or `pop`ed from the heap
    /// and that each item is inserted and removed at most once.
    /// Otherwise, this method may panic.
    pub fn remove(&mut self, item: T) {
        self.delayed_removals.push(item);
        self.apply_delayed_removals();
    }

    fn apply_delayed_removals(&mut self) {
        while let Some(next_removal) = self.delayed_removals.peek() {
            let Some(next_item) = self.inner.peek() else { break };

            assert!(next_removal <= next_item);
            if next_removal > next_item {
                break;
            }

            self.delayed_removals.pop();
            self.inner.pop();
        }
    }
}
