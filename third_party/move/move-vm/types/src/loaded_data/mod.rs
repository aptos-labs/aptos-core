// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0
//! Loaded definition of code data used in runtime.
//!
//! This module contains the loaded definition of code data used in runtime.

mod intern_table;
pub mod runtime_types;
mod tuple_helper;

pub use intern_table::IndexMap;
