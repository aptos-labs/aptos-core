// Copyright © Aptos Foundation

/// A simple trait for batched streams.
pub trait BatchedStream: Sized {
    /// The type of items the stream.
    type StreamItem;

    /// An iterator over the items in a batch.
    type BatchIter<'a>: Iterator<Item = Self::StreamItem>
    where
        Self: 'a;

    /// Applies a function to the next batch of items in the stream.
    fn process_batch<'a, F, R>(&'a mut self, f: F) -> R
    where
        F: FnOnce(Option<Self::BatchIter<'a>>) -> R;

    /// Returns the total number of items in all remaining batches of the stream combined,
    /// if available.
    fn opt_len(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of batches remaining in the stream, if available.
    fn opt_batch_count(&self) -> Option<usize> {
        None
    }

    /// Returns an iterator over the batches in the stream.
    ///
    /// NB: due to Rust borrow checking rules, each batch must be collected into a `Vec`
    /// before it is returned from the `BatchIterator`.
    /// See the comments in the `BatchIterator` implementation for more details on why
    /// this is necessary.
    fn into_batch_iter(self) -> BatchIterator<Self> {
        BatchIterator::new(self)
    }

    /// Returns an iterator over the items in the stream.
    ///
    /// NB: due to Rust borrow checking rules, each batch is collected into a `Vec`
    /// before the items can be returned from the `ItemsIterator`.
    /// See the comments in the `ItemsIterator` implementation for more details on why
    /// this is necessary.
    fn into_items_iter(self) -> ItemsIterator<Self> {
        ItemsIterator::new(self)
    }
}

impl<I> BatchedStream for I
where
    I: Iterator,
    I::Item: ExactSizeIterator,
{
    type StreamItem = <I::Item as Iterator>::Item;

    type BatchIter<'a> = I::Item
    where
        Self: 'a;

    fn process_batch<'a, F, R>(&'a mut self, f: F) -> R
    where
        F: FnOnce(Option<Self::BatchIter<'a>>) -> R,
    {
        f(self.next())
    }
}

/// A trait for batched streams with known exact size.
pub trait ExactSizeBatchedStream: BatchedStream {
    /// Returns the total number of items in all remaining batches of the stream combined.
    fn len(&self) -> usize {
        self.opt_len().unwrap()
    }
}

/// A trait for batched streams with known exact batch count.
pub trait ExactBatchCountBatchedStream: BatchedStream {
    /// Returns the total number of batches remaining in the stream.
    fn batch_count(&self) -> usize {
        self.opt_batch_count().unwrap()
    }
}

/// An iterator over batches of a batched stream.
pub struct BatchIterator<S> {
    inner: S,
}

impl<S> BatchIterator<S> {
    fn new(stream: S) -> Self {
        Self { inner: stream }
    }
}

impl<S: BatchedStream> Iterator for BatchIterator<S> {
    type Item = Vec<S::StreamItem>;

    fn next(&mut self) -> Option<Vec<S::StreamItem>> {
        // NB: as the returned item's lifetime is not ties to `self`, we cannot return
        // anything that may contain a reference to `self` or `inner`, including `BatchIter`.
        // Hence, we have to collect the batch into a `Vec` before returning it.
        self.inner
            .process_batch(|batch| batch.map(Iterator::collect))
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
    batch_iter: BatchIterator<S>,
    current_batch_iter: std::vec::IntoIter<S::StreamItem>,
}

impl<S: BatchedStream> ItemsIterator<S> {
    fn new(stream: S) -> Self {
        Self {
            batch_iter: stream.into_batch_iter(),
            current_batch_iter: vec![].into_iter(),
        }
    }
}

impl<S: BatchedStream> Iterator for ItemsIterator<S> {
    type Item = S::StreamItem;

    fn next(&mut self) -> Option<S::StreamItem> {
        // while skips empty batches.
        while self.current_batch_iter.len() == 0 {
            match self.batch_iter.next().map(|vec| vec.into_iter()) {
                Some(iter) => self.current_batch_iter = iter,
                None => return None,
            }
        }

        self.current_batch_iter.next()
    }
}

impl<S> ExactSizeIterator for ItemsIterator<S>
where
    S: ExactSizeBatchedStream,
{
    fn len(&self) -> usize {
        self.batch_iter.inner.len() + self.current_batch_iter.len()
    }
}

pub struct Batched<I> {
    items: I,
    batch_size: usize,
}

impl<I> BatchedStream for Batched<I>
where
    I: Iterator,
{
    type StreamItem = I::Item;

    type BatchIter<'a> = std::iter::Peekable<std::iter::Take<&'a mut I>>
    where
        Self: 'a;

    fn process_batch<'a, F, R>(&'a mut self, f: F) -> R
    where
        F: FnOnce(Option<Self::BatchIter<'a>>) -> R,
    {
        let mut batch = self.items.by_ref().take(self.batch_size).peekable();
        f(if batch.peek().is_some() { Some(batch) } else { None })
    }

    fn opt_len(&self) -> Option<usize> {
        self.items.size_hint().1
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.opt_len().map(|len| (len + self.batch_size - 1) / self.batch_size)
    }
}
