// Copyright © Aptos Foundation

/// Provides a set of adaptors to map iterators with explicit data capture.
/// Unlike using normal `map` with implicitly captured data, the resulting types can
/// be explicitly named, which is useful for specifying them in associated types in
/// trait implementations.
///
/// This trait will likely become unnecessary as soon as any
/// of the following features become stabilized:
/// * type_alias_impl_trait: https://github.com/rust-lang/rust/issues/63063
/// * unboxed_closures / fn_traits : https://github.com/rust-lang/rust/issues/29625
pub trait MapWithOp: Iterator + Sized {
    /// Maps an iterator with a closure that takes takes ownership of the captured data.
    fn map_with_move<T, F, R>(self, data: T, f: F) -> MapWith<Self, T, F>
    where
        F: FnMut(&mut T, Self::Item) -> R
    {
        MapWith {
            iter: self,
            data,
            f,
        }
    }

    /// Maps an iterator with a closure that clones the captured data for each call.
    fn map_with_clone<T, F, R>(self, data: T, f: F) -> MapWithClone<Self, T, F>
    where
        T: Clone,
        F: FnMut(T, Self::Item) -> R
    {
        MapWithClone {
            iter: self,
            data,
            f,
        }
    }

    /// Maps an iterator with a closure that takes a reference to the captured data.
    fn map_with_ref<T, F, R>(self, data: &T, f: F) -> MapWithRef<'_, Self, T, F>
    where
        F: FnMut(&T, Self::Item) -> R
    {
        MapWithRef {
            iter: self,
            data,
            f,
        }
    }

    /// Maps an iterator with a closure that takes a mutable reference to the captured data.
    fn map_with_mut<T, F, R>(self, data: &mut T, f: F) -> MapWithMut<'_, Self, T, F>
    where
        F: FnMut(&mut T, Self::Item) -> R
    {
        MapWithMut {
            iter: self,
            data,
            f,
        }
    }
}

impl<I> MapWithOp for I where I: Iterator {}

/// An iterator that maps items with a closure that takes ownership of the captured data.
pub struct MapWith<I, T, F> {
    iter: I,
    data: T,
    f: F,
}

impl<I, T, F, R> Iterator for MapWith<I, T, F>
where
    I: Iterator,
    F: FnMut(&mut T, I::Item) -> R,
{
    type Item = R;

    fn next(&mut self) -> Option<R> {
        self.iter.next().map(|item| (self.f)(&mut self.data, item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// An iterator that maps items with a closure that clones the captured data for each call.
pub struct MapWithClone<I, T, F> {
    iter: I,
    data: T,
    f: F,
}

impl<I, T, F, R> Iterator for MapWithClone<I, T, F>
where
    I: Iterator,
    T: Clone,
    F: FnMut(T, I::Item) -> R,
{
    type Item = R;

    fn next(&mut self) -> Option<R> {
        self.iter.next().map(|item| (self.f)(self.data.clone(), item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// An iterator that maps items with a closure that takes a reference to the captured data.
pub struct MapWithRef<'a, I, T, F> {
    iter: I,
    data: &'a T,
    f: F,
}

impl<'a, I, T, F, R> Iterator for MapWithRef<'a, I, T, F>
where
    I: Iterator,
    F: FnMut(&T, I::Item) -> R,
{
    type Item = R;

    fn next(&mut self) -> Option<R> {
        self.iter.next().map(|item| (self.f)(self.data, item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// An iterator that maps items with a closure that takes a mutable reference to the captured data.
pub struct MapWithMut<'a, I, T, F> {
    iter: I,
    data: &'a mut T,
    f: F,
}

impl<'a, I, T, F, R> Iterator for MapWithMut<'a, I, T, F>
where
    I: Iterator,
    F: FnMut(&mut T, I::Item) -> R,
{
    type Item = R;

    fn next(&mut self) -> Option<R> {
        self.iter.next().map(|item| (self.f)(self.data, item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
