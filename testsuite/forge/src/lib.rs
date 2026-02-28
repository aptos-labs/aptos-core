// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge is a framework for writing and running end-to-end tests in Aptos

pub use anyhow::Result;

mod interface;
pub use interface::*;

pub mod observer;
mod runner;
pub use runner::*;

mod backend;
pub use aptos_transaction_emitter_lib::*;
pub use aptos_transaction_generator_lib::*;
pub use aptos_transaction_workloads_lib::*;
pub use backend::*;

mod report;
pub use report::*;
pub mod result;

mod github;
pub use github::*;

mod slack;
pub use slack::*;

pub mod success_criteria;

pub mod metrics;

pub mod test_utils;

pub mod config;
pub use config::ForgeConfig;
