// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! An operational tool for node operators
//!
//! TODO: Examples
//!

use crate::CliResult;
use clap::{ArgEnum, Subcommand};

pub mod key;

/// CLI tool for performing operational tasks
///
#[derive(Debug, ArgEnum, Subcommand)]
pub enum OpTool {
    #[clap(subcommand)]
    Key(key::KeyTool),
}

impl OpTool {
    pub async fn execute(self) -> CliResult {
        match self {
            OpTool::Key(tool) => tool.execute().await,
        }
    }
}
