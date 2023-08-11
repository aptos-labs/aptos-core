// Copyright © Aptos Foundation

/// A simple trait for batched streams.
pub trait BatchedStream: Sized {
    type Item;
    type NestedIter: ExactSizeIterator<Item = Self::Item>;

    fn next_batch(&mut self) -> Option<Self::NestedIter>;

    /// Returns the total number of items in all remaining batches of the stream combined,
    /// if available.
    fn opt_len(&self) -> Option<usize>;

    fn into_iter(self) -> BatchIterator<Self> {
        BatchIterator { inner: self }
    }
}

pub struct BatchIterator<S> {
    inner: S,
}

impl<S: BatchedStream> Iterator for BatchIterator<S> {
    type Item = S::NestedIter;

    fn next(&mut self) -> Option<S::NestedIter> {
        self.inner.next_batch()
    }
}
