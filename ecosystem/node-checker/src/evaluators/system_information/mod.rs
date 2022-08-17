// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod build_version;
mod common;
mod hardware;
mod types;

pub use build_version::{BuildVersionEvaluator, BuildVersionEvaluatorArgs};
pub use hardware::{HardwareEvaluator, HardwareEvaluatorArgs};
pub use types::*;

pub const CATEGORY: &str = "system_information";
