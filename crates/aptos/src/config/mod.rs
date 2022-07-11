// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliResult},
    utils::init_logger,
};
use clap::Parser;

/// Tool for configuration of the CLI tool
///
#[derive(Parser)]
pub enum ConfigTool {
    Init(crate::common::init::InitTool),
}

impl ConfigTool {
    pub async fn execute(self) -> CliResult {
        init_logger();
        match self {
            ConfigTool::Init(tool) => tool.execute_serialized_success().await,
        }
    }
}
