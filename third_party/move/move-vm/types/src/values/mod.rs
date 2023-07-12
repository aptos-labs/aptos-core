// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod value_exchange;
pub mod values_impl;

#[cfg(test)]
mod value_exchange_test;
#[cfg(test)]
mod value_tests;

#[cfg(all(test, feature = "fuzzing"))]
mod value_prop_tests;

pub use value_exchange::*;
pub use values_impl::*;
