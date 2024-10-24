// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::async_concurrent_dropper::AsyncConcurrentDropper;
use derive_more::{Deref, DerefMut};
use once_cell::sync::Lazy;
use std::mem::ManuallyDrop;

pub mod async_concurrent_dropper;
pub mod async_drop_queue;
mod metrics;

pub static DEFAULT_DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("default", 32, 8));

/// Arc<T: ArcAsyncDrop> will be `Send + 'static`, which is required to be able to drop Arc<T>
/// in another thread
pub trait ArcAsyncDrop: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> ArcAsyncDrop for T {}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, DerefMut)]
#[repr(transparent)]
pub struct DropHelper<T: Send + 'static> {
    #[deref]
    #[deref_mut]
    inner: ManuallyDrop<T>,
}

impl<T: Send + 'static> DropHelper<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: ManuallyDrop::new(inner),
        }
    }
}

impl<T: Send + 'static> Drop for DropHelper<T> {
    fn drop(&mut self) {
        let inner = unsafe { ManuallyDrop::take(&mut self.inner) };
        DEFAULT_DROPPER.schedule_drop(inner);
    }
}
