// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{
    vec_deque::{IntoIter, Iter},
    VecDeque,
};

#[derive(Clone)]
pub struct BoundedVecDeque<T> {
    inner: VecDeque<T>,
    capacity: usize,
}

impl<T> BoundedVecDeque<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            inner: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn is_full(&self) -> bool {
        self.inner.len() == self.capacity
    }

    pub fn push_back(&mut self, item: T) -> Option<T> {
        let oldest = if self.is_full() {
            self.inner.pop_front()
        } else {
            None
        };

        self.inner.push_back(item);
        assert!(self.inner.len() <= self.capacity);
        oldest
    }

    pub fn push_front(&mut self, item: T) -> Option<T> {
        let oldest = if self.is_full() {
            self.inner.pop_back()
        } else {
            None
        };

        self.inner.push_front(item);
        assert!(self.inner.len() <= self.capacity);
        oldest
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> IntoIterator for BoundedVecDeque<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::BoundedVecDeque;

    #[test]
    fn test_bounded_vec_deque_capacity() {
        let capacity = 10;
        let mut queue = BoundedVecDeque::new(capacity);
        for i in 0..capacity {
            queue.push_back(i);
        }

        assert!(queue.is_full());

        assert_eq!(queue.push_back(capacity), Some(0));

        assert_eq!(queue.push_front(0), Some(capacity));
    }

    #[test]
    fn test_bounded_vec_deque_iter() {
        let capacity = 10;
        let mut queue = BoundedVecDeque::new(capacity);
        for i in 0..capacity {
            queue.push_back(i);
        }

        for (i, item) in queue.iter().enumerate() {
            assert_eq!(i, *item);
        }

        for (i, item) in queue.into_iter().enumerate() {
            assert_eq!(i, item);
        }
    }
}
