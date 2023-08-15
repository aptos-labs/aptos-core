// Copyright © Aptos Foundation

/// A simple trait for batched streams of data.
///
/// Any type implementing `impl Iterator<Item = impl IntoIterator>` is a batched stream.
/// Vice versa, any batched stream can be converted to `impl Iterator<Item = impl IntoIterator>`
/// via method `into_batch_iter`.
///
/// Additionally, this trait provides utility methods for optional stream length information
/// (`opt_items_count` and `opt_batch_count`) and for materializing batches (`materialize`)
/// to obtain a stream with `Vec<T>` batch type.
pub trait BatchedStream: Sized {
    /// The type of items the stream.
    type StreamItem;

    /// An iterator over the items in a batch.
    type Batch: IntoIterator<Item = Self::StreamItem>;

    /// Returns the next batch in the stream, if available.
    fn next_batch(&mut self) -> Option<Self::Batch>;

    /// Returns a batched stream with batches collected into `Vec`.
    /// This method is usually zero-cost if the stream already has `Vec` batch type.
    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    /// Returns the total number of items in all remaining batches of the stream combined,
    /// if available.
    fn opt_items_count(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of batches remaining in the stream, if available.
    fn opt_batch_count(&self) -> Option<usize> {
        None
    }

    /// Returns an iterator over the batches in the stream.
    fn into_batch_iter(self) -> BatchIterator<Self> {
        BatchIterator::new(self)
    }

    /// Returns an iterator over the items in the stream.
    fn into_items_iter(self) -> ItemsIterator<Self> {
        ItemsIterator::new(self)
    }
}

/// A trait for batched streams with known exact size.
pub trait ExactItemsCountBatchedStream: BatchedStream {
    /// Returns the total number of items in all remaining batches of the stream combined.
    fn items_count(&self) -> usize {
        self.opt_items_count().unwrap()
    }
}

/// A trait for batched streams with known exact batch count.
pub trait ExactBatchCountBatchedStream: BatchedStream {
    /// Returns the total number of batches remaining in the stream.
    fn batch_count(&self) -> usize {
        self.opt_batch_count().unwrap()
    }
}

/// A trait for batched streams with `Vec<T>` batch type.
pub trait MaterializedBatchedStream<T>: BatchedStream<StreamItem = T, Batch = Vec<T>> {}

impl<T, S> MaterializedBatchedStream<T> for S where S: BatchedStream<StreamItem = T, Batch = Vec<T>> {}

/// Implementation of `BatchedStream` for any iterator over iterators.
impl<I> BatchedStream for I
where
    I: Iterator,
    I::Item: IntoIterator,
{
    type StreamItem = <I::Item as IntoIterator>::Item;
    type Batch = I::Item;

    fn next_batch(&mut self) -> Option<Self::Batch> {
        self.next()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        let (min, max) = self.size_hint();
        if min == max? {
            Some(min)
        } else {
            None
        }
    }
}

/// Adds a method `batched` to all iterators that creates a `MaterializedBatchedStream` by
/// grouping the elements into batches of size `batch_size`. Each batch is collected into a
/// `Vec` before being returned.
pub trait Batched: Iterator + Sized {
    fn batched(self, batch_size: usize) -> BatchedIter<Self> {
        BatchedIter::new(self, batch_size)
    }
}

impl<I: Iterator> Batched for I {}

/// An adapter for a batched stream that materializes batches before returning them.
/// Usually zero-cost if the underlying stream already has `Vec` batch type.
pub struct Materialize<S> {
    inner: S,
}

impl<S> Materialize<S> {
    /// Creates a new `MaterializedStream` from a batched stream.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S: BatchedStream> BatchedStream for Materialize<S> {
    type StreamItem = S::StreamItem;
    type Batch = Vec<S::StreamItem>;

    fn next_batch(&mut self) -> Option<Self::Batch> {
        // NB: due to the use of specialization in the standard library,
        // `vec.into_iter().collect::<Vec<_>>()` is usually a no-op and does
        // not allocate a new `Vec`.
        // At the moment of writing this comment, the only exception is when the original
        // `Vec` has capacity more than twice its size.
        // (See: https://doc.rust-lang.org/src/alloc/vec/spec_from_iter.rs.html)
        self.inner
            .next_batch()
            .map(|batch| batch.into_iter().collect())
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.inner.opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.inner.opt_batch_count()
    }
}

/// An iterator over batches of a batched stream.
pub struct BatchIterator<S> {
    inner: S,
}

impl<S> BatchIterator<S> {
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }
}

impl<'s, S: BatchedStream> Iterator for BatchIterator<S> {
    type Item = S::Batch;

    fn next(&mut self) -> Option<S::Batch> {
        self.inner.next_batch()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(count) = self.inner.opt_batch_count() {
            (count, Some(count))
        } else {
            (0, None)
        }
    }
}

impl<S: ExactBatchCountBatchedStream> ExactSizeIterator for BatchIterator<S> {
    fn len(&self) -> usize {
        self.inner.batch_count()
    }
}

/// An iterator over items of a batched stream.
pub struct ItemsIterator<S: BatchedStream> {
    stream: S,
    /// Iterator over the current batch.
    /// None indicates that the stream is empty.
    current_batch_iter: Option<<S::Batch as IntoIterator>::IntoIter>,
}

impl<S: BatchedStream> ItemsIterator<S> {
    fn new(mut stream: S) -> Self {
        let current_batch_iter = stream.next_batch().map(|batch| batch.into_iter());
        Self {
            stream,
            current_batch_iter,
        }
    }
}

impl<S: BatchedStream> Iterator for ItemsIterator<S> {
    type Item = S::StreamItem;

    fn next(&mut self) -> Option<S::StreamItem> {
        // This loop will skip over empty batches.
        loop {
            // If `current_batch_iter` is `None`, then the stream is empty.
            let current_batch_iter = self.current_batch_iter.as_mut()?;

            if let Some(item) = current_batch_iter.next() {
                return Some(item);
            } else {
                self.current_batch_iter = self.stream.next_batch().map(|batch| batch.into_iter());
            }
        }
    }
}

impl<S> ExactSizeIterator for ItemsIterator<S>
where
    S: ExactItemsCountBatchedStream,
    <S::Batch as IntoIterator>::IntoIter: ExactSizeIterator,
{
    fn len(&self) -> usize {
        match &self.current_batch_iter {
            Some(iter) => iter.len() + self.stream.items_count(),
            None => 0,
        }
    }
}

/// A wrapper around an iterator that groups its elements into batches of size `batch_size`.
pub struct BatchedIter<I> {
    iter: I,
    batch_size: usize,
}

impl<I> BatchedIter<I> {
    pub fn new(iter: I, batch_size: usize) -> Self {
        Self { iter, batch_size }
    }
}

impl<'a, I> BatchedStream for &'a mut BatchedIter<I>
where
    I: Iterator,
{
    type StreamItem = I::Item;
    type Batch = Vec<I::Item>;

    fn next_batch(&mut self) -> Option<Self::Batch> {
        let mut batch = self.iter.by_ref().take(self.batch_size).peekable();
        // Unfortunately, due to Rust borrow checking rules, the lifetime of the
        // returned batch cannot depend on the lifetime of `self`. Hence, we need
        // to collect the batch into a `Vec` before returning it.
        if batch.peek().is_some() {
            Some(batch.collect())
        } else {
            None
        }
    }

    fn opt_items_count(&self) -> Option<usize> {
        let (min, max) = self.iter.size_hint();
        if min == max? {
            Some(min)
        } else {
            None
        }
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.opt_items_count()
            .map(|len| (len + self.batch_size - 1) / self.batch_size)
    }
}
