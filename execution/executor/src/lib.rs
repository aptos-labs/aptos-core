// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
// FIXME(aldenhu)
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;
mod logging;
pub mod metrics;
#[cfg(test)]
mod tests;

pub mod block_executor;
pub mod chunk_executor;
pub mod db_bootstrapper;
pub mod types;
pub mod workflow;
