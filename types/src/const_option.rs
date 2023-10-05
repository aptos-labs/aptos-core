// Copyright © Aptos Foundation

use std::ops::{Deref, DerefMut};

mod private {
    /// Used to create a "sealed" trait, i.e., a trait that cannot be implemented
    /// outside of this module.
    pub trait Seal {}
}

/// Similar to `Option`, but it is determined at compile time whether the value is `Some` or `None`.
/// Can be used to cover some of the use-cases of specialization
/// (see: https://github.com/rust-lang/rust/issues/31844).
pub trait ConstOption<T>: private::Seal {
    fn into_option(self) -> Option<T>;

    fn as_option(&self) -> Option<&T>;

    fn as_option_mut(&mut self) -> Option<&mut T>;
}

#[derive(Clone, Copy, Debug)]
pub struct ConstSome<T>(pub T);

impl<T> private::Seal for ConstSome<T> {}

impl<T> ConstOption<T> for ConstSome<T> {
    fn into_option(self) -> Option<T> {
        Some(self.0)
    }

    fn as_option(&self) -> Option<&T> {
        Some(&self.0)
    }

    fn as_option_mut(&mut self) -> Option<&mut T> {
        Some(&mut self.0)
    }
}

impl<T> AsRef<T> for ConstSome<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for ConstSome<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Deref for ConstSome<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ConstSome<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConstNone();

impl private::Seal for ConstNone {}

impl<T> ConstOption<T> for ConstNone {
    fn into_option(self) -> Option<T> {
        None
    }

    fn as_option(&self) -> Option<&T> {
        None
    }

    fn as_option_mut(&mut self) -> Option<&mut T> {
        None
    }
}
