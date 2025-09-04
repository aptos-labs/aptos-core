// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::async_concurrent_dropper::AsyncConcurrentDropper;
use once_cell::sync::Lazy;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

pub mod async_concurrent_dropper;
pub mod async_drop_queue;
mod metrics;

thread_local! {
    static IN_ANY_DROP_POOL: Cell<bool> = const { Cell::new(false) };
}

pub static DEFAULT_DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("default", 32, 8));

/// Arc<T: ArcAsyncDrop> will be `Send + 'static`, which is required to be able to drop Arc<T>
/// in another thread
pub trait ArcAsyncDrop: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> ArcAsyncDrop for T {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DropHelper<T: Send + 'static> {
    inner: Option<T>,
}

impl<T: Default + Send + 'static> Default for DropHelper<T> {
    fn default() -> Self {
        Self {
            inner: Some(T::default()),
        }
    }
}

impl<T: Send + 'static> DropHelper<T> {
    pub fn new(inner: T) -> Self {
        Self { inner: Some(inner) }
    }

    pub fn into_inner(mut self) -> T {
        self.inner.take().expect("Initialized to Some.")
    }
}

impl<T: Send + 'static> Drop for DropHelper<T> {
    fn drop(&mut self) {
        DEFAULT_DROPPER.schedule_drop(self.inner.take());
    }
}

impl<T: Send + 'static> Deref for DropHelper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("Initialized to Some.")
    }
}

impl<T: Send + 'static> DerefMut for DropHelper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().expect("Initialized to Some.")
    }
}
