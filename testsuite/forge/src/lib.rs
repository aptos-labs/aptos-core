// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Forge is a framework for writing and running end-to-end tests in Diem

pub use anyhow::Result;

mod interface;
pub use interface::*;

mod runner;
pub use runner::*;

mod backend;
pub use backend::*;

pub use transaction_emitter::*;

mod report;
pub use report::*;

mod slack;
pub use slack::*;
