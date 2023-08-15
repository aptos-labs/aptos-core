// Copyright © Aptos Foundation

use namable_closures::StableFnMut;

/// Adds a set of adaptors that work with the [`namable_closures`] crate.
pub trait ClosureTools: Iterator + Sized {
    /// [`Iterator::map`] that is compatible with the [`namable_closures`] crate.
    fn map_closure<F>(self, f: F) -> MapClosure<Self, F>
    where
        F: StableFnMut<(Self::Item,)>
    {
        MapClosure {
            iter: self,
            f,
        }
    }

    /// [`Iterator::filter`] that is compatible with the [`namable_closures`] crate.
    fn filter_closure<P>(self, predicate: P) -> FilterClosure<Self, P>
    where
        P: for <'a> StableFnMut<(&'a Self::Item,), Output = bool>
    {
        FilterClosure {
            iter: self,
            predicate,
        }
    }
}

impl<I> ClosureTools for I where I: Iterator {}

pub struct MapClosure<I, F> {
    iter: I,
    f: F,
}

impl<I, F> Iterator for MapClosure<I, F>
where
    I: Iterator,
    F: StableFnMut<(I::Item,)>,
{
    type Item = F::Output;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|x| self.f.stable_call_mut((x,)))
    }
}

pub struct FilterClosure<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P> Iterator for FilterClosure<I, P>
where
    I: Iterator,
    P: for <'a> StableFnMut<(&'a I::Item,), Output = bool>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().filter(|x| self.predicate.stable_call_mut((x,)))
    }
}
