// Copyright © Aptos Foundation

use crate::no_error;
use crate::no_error::NoError;

/// A trait to represent fallible batched computation.
///
/// Any type implementing `Iterator<Item = Result<impl IntoIterator, _>`
/// or `Iterator<Item = impl IntoIterator>` can be converted to a batched stream via
/// `into_batched_stream()` or `into_no_error_batched_stream()` methods respectively
/// (from traits `IntoBatchedStream` and `IntoNoErrorBatchedStream`).
/// Conversely, any batched stream can be converted to
/// `Iterator<Item = Result<Self::Batch, Self::Error>>` via method `into_batch_iter`.
/// A batched stream with special `NoError` error type can be converted to
/// `Iterator<Item = Self::Batch>` via method `into_no_error_batch_iter`.
///
/// Additionally, this trait provides utility methods for optional stream length information
/// (`opt_items_count` and `opt_batch_count`) and for materializing batches (`materialize`)
/// to obtain a stream with `Vec<T>` batch type.
///
/// NOTE: the lifetime of a `Batch` cannot depend on the lifetime of `self`.
/// Changing it would require using Generic Associated Types (GATs).
/// However, Rust's support for GATs is quite limited at the moment.
pub trait BatchedStream: Sized {
    /// The type of items the stream.
    type StreamItem;

    /// An iterator over the items in a batch.
    type Batch: IntoIterator<Item = Self::StreamItem>;

    /// The error type of the stream.
    ///
    /// Use `no_error::NoError` if the stream cannot fail.
    type Error;

    // Required method:

    /// Advances the stream and returns the next batch.
    ///
    /// Returns [`None`] when stream is finished.
    /// If an error occurs, returns [`Some(Err(error))`].
    /// Repeated calls to `next_batch` after the first error may return errors,
    /// [`None`], or new batches, depending on the implementation.
    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>>;

    // Optional methods:

    /// Returns the total number of items in all remaining batches of the stream combined,
    /// if available.
    /// However, the stream may end prematurely if an error occurs.
    fn opt_items_count(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of batches remaining in the stream, if available.
    /// However, the stream may end prematurely if an error occurs.
    fn opt_batch_count(&self) -> Option<usize> {
        None
    }

    // Provided methods:

    /// Returns a batched stream with batches collected into `Vec`.
    /// This method is usually zero-cost if the stream already has `Vec` batch type.
    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    /// Returns an iterator over `Result<Self::Batch, Self::Error>`.
    fn into_batch_iter(self) -> BatchIterator<Self> {
        BatchIterator::new(self)
    }

    /// Returns an iterator over `Self::Batch` when `Self::Error = NoError`.
    fn into_no_error_batch_iter(self) -> NoErrorBatchIterator<Self>
    where
        Self: BatchedStream<Error = NoError>,
    {
        NoErrorBatchIterator::new(self)
    }

    /// Returns an iterator over the items in the stream.
    fn into_items_iter(self) -> ItemsIterator<Self> {
        ItemsIterator::new(self)
    }

    /// Applies [`Result::and_then`] to the result of computation of each batch of the stream.
    fn and_then<B, F>(self, op: F) -> AndThen<Self, F>
    where
        B: IntoIterator,
        F: FnMut(Self::Batch) -> Result<B, Self::Error>,
    {
        AndThen::new(self, op)
    }

    /// Takes a closure and creates a batched stream that calls that closure
    /// on the result of computation of each batch.
    fn map_results<B, E, F>(self, f: F) -> MapResults<Self, F>
    where
        B: IntoIterator,
        F: FnMut(Result<Self::Batch, Self::Error>) -> Result<B, E>
    {
        MapResults::new(self, f)
    }

    /// Takes a closure and creates a batched stream that calls that closure on each batch.
    fn map_batches<R, F>(self, f: F) -> MapBatches<Self, F>
    where
        R: IntoIterator,
        F: FnMut(Self::Batch) -> R,
    {
        MapBatches::new(self, f)
    }

    /// Takes a closure and creates a batched stream that calls that closure on each item.
    ///
    /// The closure cannot be mutable and must implement `Clone` as batches cannot borrow `self`.
    /// The latter can be easily achieved by passing an immutable reference to the closure.
    /// Lifting these requirements would require using the `LendingIterator` pattern,
    /// which is not yet fully supported by stable Rust and would significantly
    /// complicate the API.
    ///
    /// Consider using `map_batches` if you need to pass a mutable closure.
    fn map_items<R, F>(self, f: F) -> MapItems<Self, F>
    where
        F: Clone + Fn(Self::StreamItem) -> R,
    {
        MapItems::new(self, f)
    }

    /// Takes a closure and creates a batched stream that calls that closure
    /// on the returned errors, if any.
    fn map_errors<E, F>(self, f: F) -> MapErrors<Self, F>
    where
        F: Fn(Self::Error) -> E,
    {
        MapErrors::new(self, f)
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

/// A trait for batched streams with special `NoError` error type.
pub trait NoErrorBatchedStream: BatchedStream<Error = NoError> {}

impl<S> NoErrorBatchedStream for S where S: BatchedStream<Error = NoError> {}

/// A trait for batched streams with `Vec<T>` batch type.
pub trait MaterializedBatchedStream<T>: BatchedStream<StreamItem = T, Batch = Vec<T>> {}

impl<T, S> MaterializedBatchedStream<T> for S where S: BatchedStream<StreamItem = T, Batch = Vec<T>> {}

// A mutable reference to a `BatchedStream` is a `BatchedStream` itself.
impl<'a, S> BatchedStream for &'a mut S
where
    S: BatchedStream,
{
    type StreamItem = S::StreamItem;
    type Batch = S::Batch;
    type Error = S::Error;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        (**self).next_batch()
    }

    fn opt_items_count(&self) -> Option<usize> {
        (**self).opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        (**self).opt_batch_count()
    }
}

impl<'a, S> ExactItemsCountBatchedStream for &'a mut S
where
    S: ExactItemsCountBatchedStream,
{
    fn items_count(&self) -> usize {
        (**self).items_count()
    }
}

impl<'a, S> ExactBatchCountBatchedStream for &'a mut S
where
    S: ExactBatchCountBatchedStream,
{
    fn batch_count(&self) -> usize {
        (**self).batch_count()
    }
}

/// Adds method `as_batched_stream` for all types
/// implementing `Iterator<Item = Result<impl IntoIterator, _>`.
pub trait IntoBatchedStream: Sized {
    fn into_batched_stream(self) -> IterIntoBatchedStream<Self> {
        IterIntoBatchedStream { iter: self }
    }
}

impl<I, II, E> IntoBatchedStream for I
where
    I: Iterator<Item = Result<II, E>>,
    II: IntoIterator,
{
}

/// Adds method `as_no_error_batched_stream` for all types
/// implementing `Iterator<Item = impl IntoIterator>`.
pub trait IntoNoErrorBatchedStream: Sized {
    fn into_no_error_batched_stream(self) -> IterIntoNoErrorBatchedStream<Self> {
        IterIntoNoErrorBatchedStream { iter: self }
    }
}

impl<I> IntoNoErrorBatchedStream for I
where
    I: Iterator,
    I::Item: IntoIterator,
{
}

/// Adds a method `batched` to all iterators.
///
/// `batched` creates a batch stream by grouping the elements into batches
/// of size `batch_size`. Each batch is collected into a `Vec` before being returned.
pub trait Batched: Iterator + Sized {
    fn batched(self, batch_size: usize) -> BatchedIter<Self> {
        BatchedIter::new(self, batch_size)
    }
}

impl<I: Iterator> Batched for I {}

/// Returns a batched stream that returns a single batch or a single error.
pub fn once<Batch, Error>(result: Result<Batch, Error>) -> Once<Batch, Error>
where
    Batch: IntoIterator,
{
    Once::new(result)
}

// Wrapper types:

/// A batched stream that wraps an iterator over `Result`s of batches.
pub struct IterIntoBatchedStream<I> {
    iter: I,
}

impl<I, II, E> BatchedStream for IterIntoBatchedStream<I>
where
    I: Iterator<Item = Result<II, E>>,
    II: IntoIterator,
{
    type StreamItem = II::Item;
    type Batch = II;
    type Error = E;

    fn next_batch(&mut self) -> Option<Result<II, E>> {
        self.iter.next()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        let (min, max) = self.iter.size_hint();
        if min == max? {
            Some(min)
        } else {
            None
        }
    }
}

/// A batched stream that wraps an iterator over batches of items, with no errors.
pub struct IterIntoNoErrorBatchedStream<I> {
    iter: I,
}

impl<I> BatchedStream for IterIntoNoErrorBatchedStream<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    type StreamItem = <I::Item as IntoIterator>::Item;
    type Batch = I::Item;
    type Error = NoError;

    fn next_batch(&mut self) -> Option<no_error::Result<Self::Batch>> {
        self.iter.next().map(|batch| Ok(batch))
    }

    fn opt_batch_count(&self) -> Option<usize> {
        let (min, max) = self.iter.size_hint();
        if min == max? {
            Some(min)
        } else {
            None
        }
    }
}

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
    type Error = S::Error;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        // NB: due to the use of specialization in the standard library,
        // `vec.into_iter().collect::<Vec<_>>()` is usually a no-op and does
        // not allocate a new `Vec`.
        // At the moment of writing this comment, the only exception is when the original
        // `Vec` has capacity more than twice its size.
        // (See: https://doc.rust-lang.org/src/alloc/vec/spec_from_iter.rs.html)
        self.inner
            .next_batch()
            .map(|batch_or_err| batch_or_err.map(|batch| batch.into_iter().collect()))
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

impl<S: BatchedStream> Iterator for BatchIterator<S> {
    type Item = Result<S::Batch, S::Error>;

    fn next(&mut self) -> Option<Self::Item> {
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

/// An iterator over batches of a `NoErrorBatchedStream`.
pub struct NoErrorBatchIterator<S> {
    inner: S,
}

impl<S> NoErrorBatchIterator<S> {
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }
}

impl<S> Iterator for NoErrorBatchIterator<S>
where
    S: BatchedStream<Error = NoError>,
{
    type Item = S::Batch;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next_batch() {
            Some(Ok(batch)) => Some(batch),
            Some(Err(err)) => {
                let _: NoError = err; // type assertion, to make the code fool-proof.
                unreachable!("NoError cannot be instantiated")
            },
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(count) = self.inner.opt_batch_count() {
            (count, Some(count))
        } else {
            (0, None)
        }
    }
}

impl<S> ExactSizeIterator for NoErrorBatchIterator<S>
where
    S: ExactBatchCountBatchedStream<Error = NoError>,
{
    fn len(&self) -> usize {
        self.inner.batch_count()
    }
}

/// An iterator over items of a batched stream.
pub struct ItemsIterator<S: BatchedStream> {
    stream: S,
    current_batch_iter: Option<Result<<S::Batch as IntoIterator>::IntoIter, S::Error>>,
}

impl<S: BatchedStream> ItemsIterator<S> {
    fn new(mut stream: S) -> Self {
        let current_batch_iter = stream
            .next_batch()
            .map(|batch_or_err| batch_or_err.map(|batch| batch.into_iter()));

        Self {
            stream,
            current_batch_iter,
        }
    }
}

impl<S: BatchedStream> Iterator for ItemsIterator<S> {
    type Item = Result<S::StreamItem, S::Error>;

    fn next(&mut self) -> Option<Result<S::StreamItem, S::Error>> {
        // This loop will skip over empty batches.
        loop {
            match self.current_batch_iter.take() {
                Some(Ok(mut iter)) => {
                    if let Some(item) = iter.next() {
                        // Put the iterator k into `self.current_batch_iter`.
                        self.current_batch_iter = Some(Ok(iter));
                        return Some(Ok(item));
                    } else {
                        self.current_batch_iter = self
                            .stream
                            .next_batch()
                            .map(|batch_or_err| batch_or_err.map(|batch| batch.into_iter()));
                        // the loop continues.
                    }
                },
                Some(Err(err)) => {
                    return Some(Err(err));
                },
                None => {
                    return None;
                },
            }
        }
    }
}

impl<S> ExactSizeIterator for ItemsIterator<S>
where
    S: ExactItemsCountBatchedStream<Error = NoError>,
    <S::Batch as IntoIterator>::IntoIter: ExactSizeIterator,
{
    fn len(&self) -> usize {
        match &self.current_batch_iter {
            Some(Ok(iter)) => iter.len() + self.stream.items_count(),
            Some(Err(err)) => {
                let _: &NoError = err; // type assertion, to make the code fool-proof.
                unreachable!("NoError cannot be instantiated")
            },
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
    type Error = NoError;

    fn next_batch(&mut self) -> Option<no_error::Result<Self::Batch>> {
        let mut batch = self.iter.by_ref().take(self.batch_size).peekable();
        // Unfortunately, due to Rust borrow checking rules, the lifetime of the
        // returned batch cannot depend on the lifetime of `self`. Hence, we need
        // to collect the batch into a `Vec` before returning it.
        if batch.peek().is_some() {
            Some(Ok(batch.collect()))
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

/// A batched stream that returns a single batch or a single error.
pub struct Once<B, E> {
    result: Option<Result<B, E>>,
}

impl<B, E> Once<B, E> {
    pub fn new(result: Result<B, E>) -> Self {
        Self {
            result: Some(result),
        }
    }
}

impl<B, E> BatchedStream for Once<B, E>
where
    B: IntoIterator,
{
    type StreamItem = B::Item;
    type Batch = B;
    type Error = E;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        self.result.take()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        Some(if self.result.is_some() { 1 } else { 0 })
    }
}

/// A batched stream adapter that applies [`Result::and_then`] to the result of computation
/// of each batch of the stream.
pub struct AndThen<S, F> {
    stream: S,
    op: F,
}

impl<S, F> AndThen<S, F> {
    pub fn new(stream: S, op: F) -> Self {
        Self { stream, op }
    }
}

impl<S, F, B> BatchedStream for AndThen<S, F>
where
    S: BatchedStream,
    F: FnMut(S::Batch) -> Result<B, S::Error>,
    B: IntoIterator,
{
    type StreamItem = B::Item;
    type Batch = B;
    type Error = S::Error;

    fn next_batch(&mut self) -> Option<Result<B, S::Error>> {
        self.stream
            .next_batch()
            .map(|batch_or_err| batch_or_err.and_then(|batch| (self.op)(batch)))
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.stream.opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.stream.opt_batch_count()
    }
}

/// A batched stream adapter that maps results with the given function.
pub struct MapResults<S, F> {
    stream: S,
    f: F,
}

impl<S, F> MapResults<S, F> {
    pub fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, B, E> BatchedStream for MapResults<S, F>
    where
        S: BatchedStream,
        F: FnMut(Result<S::Batch, S::Error>) -> Result<B, E>,
        B: IntoIterator,
{
    type StreamItem = B::Item;
    type Batch = B;
    type Error = E;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        self.stream
            .next_batch()
            .map(|batch_or_err| (self.f)(batch_or_err))
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.stream.opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.stream.opt_batch_count()
    }
}

/// A batched stream that maps batches with the given function.
pub struct MapBatches<S, F> {
    stream: S,
    f: F,
}

impl<S, F> MapBatches<S, F> {
    pub fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, R> BatchedStream for MapBatches<S, F>
where
    S: BatchedStream,
    F: FnMut(S::Batch) -> R,
    R: IntoIterator,
{
    type StreamItem = R::Item;
    type Batch = R;
    type Error = S::Error;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        self.stream
            .next_batch()
            .map(|batch_or_err| batch_or_err.map(|batch| (self.f)(batch)))
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.stream.opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.stream.opt_batch_count()
    }
}

/// A batched stream adapter that maps items with the given function.
pub struct MapItems<S, F> {
    stream: S,
    f: F,
}

impl<S, F> MapItems<S, F> {
    pub fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, R> BatchedStream for MapItems<S, F>
where
    S: BatchedStream,
    F: Clone + Fn(S::StreamItem) -> R,
{
    type StreamItem = R;
    type Batch = std::iter::Map<<S::Batch as IntoIterator>::IntoIter, F>;
    type Error = S::Error;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        self.stream
            .next_batch()
            .map(|batch_or_err| batch_or_err.map(|batch| batch.into_iter().map(self.f.clone())))
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.stream.opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.stream.opt_batch_count()
    }
}

/// A batched stream adapter that maps errors with the given function.
pub struct MapErrors<S, F> {
    stream: S,
    f: F,
}

impl<S, F> MapErrors<S, F> {
    pub fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, E> BatchedStream for MapErrors<S, F>
where
    S: BatchedStream,
    F: Fn(S::Error) -> E,
{
    type StreamItem = S::StreamItem;
    type Batch = S::Batch;
    type Error = E;

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        self.stream
            .next_batch()
            .map(|batch_or_err| batch_or_err.map_err(|err| (self.f)(err)))
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.stream.opt_items_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.stream.opt_batch_count()
    }
}
