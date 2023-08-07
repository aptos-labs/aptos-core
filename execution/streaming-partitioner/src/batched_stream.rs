
// Copyright © Aptos Foundation

pub trait BatchedStream {
    type Item;
    type NestedIter: ExactSizeIterator<Item = Self::Item>;

    fn next_batch(&mut self) -> Option<Self::NestedIter>;

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
