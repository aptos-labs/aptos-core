// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

pub mod checks;
mod types;

#[derive(Parser)]
#[clap(
    name = "Movement post-migration validation tool",
    author,
    disable_version_flag = true
)]
pub enum ValidationTool {
    Api(checks::api::Command),
    Node(checks::node::Command),
}

impl ValidationTool {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            ValidationTool::Api(cmd) => cmd.run().await,
            ValidationTool::Node(cmd) => cmd.run().await,
        }
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    ValidationTool::command().debug_assert()
}
