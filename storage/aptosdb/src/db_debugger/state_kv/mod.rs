// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod get_value;

use aptos_storage_interface::Result;

#[derive(clap::Subcommand)]
pub enum Cmd {
    GetValue(get_value::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::GetValue(cmd) => cmd.run(),
        }
    }
}
