// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_gas_schedule_updator::{generate_update_proposal, GenArgs};
use clap::Parser;

fn main() -> Result<()> {
    let args = GenArgs::parse();

    generate_update_proposal(&args)
}
