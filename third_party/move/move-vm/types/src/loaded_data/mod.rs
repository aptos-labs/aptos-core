// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0
//! Loaded definition of code data used in runtime.
//!
//! This module contains the loaded definition of code data used in runtime.

pub mod runtime_access_specifier;
#[cfg(test)]
mod runtime_access_specifiers_prop_tests;
pub mod runtime_types;
pub mod struct_name_indexing;
pub mod ty_args_fingerprint;
