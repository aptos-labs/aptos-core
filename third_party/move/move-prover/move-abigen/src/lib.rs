// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

extern crate core;

mod abigen;
mod structgen;

pub use crate::abigen::*;
pub use crate::structgen::*;
