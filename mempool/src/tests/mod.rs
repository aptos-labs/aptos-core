// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Command to run all the mempool tests:
//      cargo test --package velor-mempool --lib -- tests --show-output

#[cfg(test)]
mod common;
#[cfg(test)]
mod core_mempool_test;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod multi_node_test;
#[cfg(test)]
mod node;
#[cfg(test)]
mod shared_mempool_test;

pub mod fuzzing;
#[cfg(any(feature = "fuzzing", test))]
pub mod mocks;
#[cfg(test)]
mod test_framework;
