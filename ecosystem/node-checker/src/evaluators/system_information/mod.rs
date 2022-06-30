// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod build_version;
mod common;
mod types;

pub use build_version::{BuildVersionEvaluator, BuildVersionEvaluatorArgs};
pub use types::*;

pub const CATEGORY: &str = "system_information";
