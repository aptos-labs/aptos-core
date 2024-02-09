use std::collections::{vec_deque, VecDeque};

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

    pub fn iter(&self) -> vec_deque::Iter<'_, T> {
        self.inner.iter()
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
    }
}
