// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod function_values_impl;
pub mod values_impl;

#[cfg(test)]
mod value_tests;

#[cfg(test)]
mod serialization_tests;
#[cfg(test)]
mod value_depth_tests;
#[cfg(all(test, feature = "fuzzing"))]
mod value_prop_tests;

pub use function_values_impl::*;
pub use values_impl::*;
