// Copyright © Aptos Foundation

/// A simple trait for batched streams.
pub trait BatchedStream: Sized {
    /// The type of items the stream.
    type StreamItem;

    /// An iterator over the items in a batch.
    type Batch: IntoIterator<Item = Self::StreamItem>;

    /// Returns the next batch of items in the stream.
    fn next_batch(&mut self) -> Option<Self::Batch>;

    /// Returns the total number of items in all remaining batches of the stream combined,
    /// if available.
    fn opt_len(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of batches remaining in the stream, if available.
    fn opt_batch_count(&self) -> Option<usize> {
        None
    }

    fn into_batch_iter(self) -> BatchIterator<Self> {
        BatchIterator { inner: self }
    }

    fn into_items_iter(self) -> ItemsIterator<Self> {
        ItemsIterator {
            inner: self,
            current_batch_iter: None,
        }
    }
}

impl<I> BatchedStream for I
where
    I: Iterator,
    I::Item: ExactSizeIterator,
{
    type StreamItem = <I::Item as Iterator>::Item;
    type Batch = I::Item;

    fn next_batch(&mut self) -> Option<Self::Batch> {
        self.next()
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

impl<S: BatchedStream> Iterator for BatchIterator<S> {
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
    inner: S,
    current_batch_iter: Option<<S::Batch as IntoIterator>::IntoIter>,
}

impl<S: BatchedStream> Iterator for ItemsIterator<S> {
    type Item = S::StreamItem;

    fn next(&mut self) -> Option<S::StreamItem> {
        if let Some(batch) = self.current_batch_iter.as_mut() {
            if let Some(item) = batch.next() {
                return Some(item);
            }
        }

        self.current_batch_iter = self.inner.next_batch().map(IntoIterator::into_iter);
        self.current_batch_iter.as_mut().and_then(Iterator::next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(count) = self.inner.opt_len() {
            (count, Some(count))
        } else {
            (0, None)
        }
    }
}

impl<S> ExactSizeIterator for ItemsIterator<S>
where
    S: ExactSizeBatchedStream,
    <S::Batch as IntoIterator>::IntoIter: ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.inner.len() + self.current_batch_iter.as_ref().map(|b| b.len()).unwrap_or(0)
    }
}
