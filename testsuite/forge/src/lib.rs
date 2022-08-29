// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Forge is a framework for writing and running end-to-end tests in Aptos

pub use anyhow::Result;

mod interface;
pub use interface::*;

mod runner;
pub use runner::*;

mod backend;
pub use backend::*;

pub use transaction_emitter_lib::*;

mod report;
pub use report::*;

mod github;
pub use github::*;

mod slack;
pub use slack::*;

pub mod success_criteria;

pub mod test_utils;
