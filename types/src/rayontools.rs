// Copyright © Aptos Foundation

use rayon::{
    iter::ParallelExtend,
    prelude::{IntoParallelIterator, ParallelIterator},
};
use std::marker::PhantomData;

/// Adds method `by_ref()` to `ParallelExtend` that returns a wrapper around `&mut A`
/// that "inherits" the implementation of `ParallelExtend` from `A`.
///
/// Useful for overcoming some limitations in rayon's `ParallelExtend` API.
/// See: https://github.com/rayon-rs/rayon/issues/1089
pub trait ParExtendByRefTrait<T: Send>: ParallelExtend<T> {
    fn by_ref(&mut self) -> ExtendRef<'_, T, Self> {
        ExtendRef::new(self)
    }
}

impl<T: Send, A: ParallelExtend<T> + ?Sized> ParExtendByRefTrait<T> for A {}

/// A wrapper around `&mut A` that "inherits" the implementation of `ParallelExtend` from `A`.
///
/// Useful for overcoming some limitations in rayon's `ParallelExtend` API.
/// See: https://github.com/rayon-rs/rayon/issues/1089
///
/// The type parameter `T` is necessary for Rust to infer which implementation of `by_ref()` to use.
pub struct ExtendRef<'a, T, A: ?Sized> {
    inner: &'a mut A,
    _phantom: PhantomData<T>,
}

impl<'a, T, A: ?Sized> ExtendRef<'a, T, A> {
    /// Create a new `ExtendRef` from a mutable reference.
    pub fn new(inner: &'a mut A) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Send, A: ParallelExtend<T> + ?Sized> ParallelExtend<T> for ExtendRef<'a, T, A> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        self.inner.par_extend(par_iter)
    }
}

#[cfg(test)]
mod tests {
    use crate::rayontools::{ExtendRef, ParExtendByRefTrait};
    use rayon::prelude::*;

    #[test]
    fn simple_extend_ref_test() {
        let mut vec1: Vec<f64> = vec![1., 2., 3.];
        let mut vec2: Vec<String> = vec!["1".to_string(), "2".to_string(), "3".to_string()];

        let input = vec![4, 5, 6];

        (vec1.by_ref(), vec2.by_ref())
            .par_extend(input.into_par_iter().map(|i| (i as f64, i.to_string())));

        assert_eq!(&vec1, &vec![1., 2., 3., 4., 5., 6.]);
        assert_eq!(&vec2, &vec!["1", "2", "3", "4", "5", "6"]);
    }
}
