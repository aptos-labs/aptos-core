// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
