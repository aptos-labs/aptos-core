// Copyright © Aptos Foundation

use rayon::iter::ParallelExtend;
use rayon::prelude::IntoParallelIterator;

/// A wrapper around `&mut A` that "inherits" the implementation of `ParallelExtend` from `A`.
///
/// Useful for overcoming some limitations in rayon's `ParallelExtend` API.
/// See: https://github.com/rayon-rs/rayon/issues/1089
pub struct ExtendRef<'a, A> {
    inner: &'a mut A,
}

impl<'a, A> ExtendRef<'a, A> {
    /// Create a new `ExtendRef` from a mutable reference.
    pub fn new(inner: &'a mut A) -> Self {
        Self { inner }
    }
}

impl<'a, T: Send, A: ParallelExtend<T>> ParallelExtend<T> for ExtendRef<'a, A> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        self.inner.par_extend(par_iter)
    }
}
