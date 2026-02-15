// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Agent-driven strategy: Rust code orchestrates a multi-phase loop with specific prompts.

pub mod loop_driver;
pub mod prompt;

pub use loop_driver::run_agent_loop;
