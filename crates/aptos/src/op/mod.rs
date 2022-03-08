// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! An operational tool for node operators
//!
//! TODO: Examples
//!

use crate::CliResult;
use structopt::StructOpt;

pub mod key;

/// CLI tool for performing operational tasks
///
#[derive(Debug, StructOpt)]
pub enum OpTool {
    Key(key::KeyTool),
}

impl OpTool {
    pub async fn execute(self) -> CliResult {
        match self {
            OpTool::Key(key_tool) => key_tool.execute().await,
        }
    }
}
