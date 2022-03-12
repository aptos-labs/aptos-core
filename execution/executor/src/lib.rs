// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;
mod logging;
pub mod metrics;
#[cfg(test)]
mod mock_vm;
#[cfg(test)]
mod tests;

pub mod block_executor;
pub mod chunk_executor;
pub mod components;
pub mod db_bootstrapper;
