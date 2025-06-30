// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    checks::node::global_storage_includes::GlobalStorageIncludes,
    types::storage::{MovementAptosStorage, MovementStorage},
};
use clap::Parser;
use std::path::PathBuf;

mod global_storage_includes;

#[derive(Parser)]
#[clap(
    name = "migration-node-validation",
    about = "Validates data conformity after movement migration."
)]
pub struct Command {
    #[clap(long = "movement", help = "The path to the movement database.")]
    pub movement_db: PathBuf,
    #[clap(
        long = "movement-aptos",
        help = "The path to the movement Aptos database."
    )]
    pub movement_aptos_db: PathBuf,
}

impl Command {
    pub async fn run(self) -> anyhow::Result<()> {
        let movement_storage = MovementStorage::open(&self.movement_db)?;
        let movement_aptos_storage = MovementAptosStorage::open(&self.movement_aptos_db)?;

        GlobalStorageIncludes::satisfies(&movement_storage, &movement_aptos_storage)?;

        Ok(())
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Command::command().debug_assert()
}
