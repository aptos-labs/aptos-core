// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod buffered_x;
mod futures_ordered_x;
mod futures_unordered_x;
mod try_buffered_x;

use crate::utils::stream::{buffered_x::BufferedX, try_buffered_x::TryBufferedX};
use futures::{Future, Stream, TryFuture, TryStream};

pub(crate) trait StreamX: Stream {
    fn buffered_x(self, n: usize, max_in_progress: usize) -> BufferedX<Self>
    where
        Self::Item: Future,
        Self: Sized,
    {
        BufferedX::new(self, n, max_in_progress)
    }
}

impl<T: ?Sized> StreamX for T where T: Stream {}

pub(crate) trait TryStreamX: TryStream {
    fn try_buffered_x(self, n: usize, max_in_progress: usize) -> TryBufferedX<Self>
    where
        Self::Ok: TryFuture<Error = Self::Error>,
        Self: Sized,
    {
        TryBufferedX::new(self, n, max_in_progress)
    }
}

impl<T: ?Sized> TryStreamX for T where T: TryStream {}
