// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//! Loaded definition of code data used in runtime.
//!
//! This module contains the loaded definition of code data used in runtime.

pub mod runtime_access_specifier;
#[cfg(test)]
mod runtime_access_specifiers_prop_tests;
pub mod runtime_types;
pub mod struct_name_indexing;
