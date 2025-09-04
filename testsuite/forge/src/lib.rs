// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Forge is a framework for writing and running end-to-end tests in Velor

pub use anyhow::Result;

mod interface;
pub use interface::*;

pub mod observer;
mod runner;
pub use runner::*;

mod backend;
pub use velor_transaction_emitter_lib::*;
pub use velor_transaction_generator_lib::*;
pub use velor_transaction_workloads_lib::*;
pub use backend::*;

mod report;
pub use report::*;
pub mod result;

mod github;
pub use github::*;

mod slack;
pub use slack::*;

pub mod success_criteria;

pub mod test_utils;

pub mod config;
pub use config::ForgeConfig;
