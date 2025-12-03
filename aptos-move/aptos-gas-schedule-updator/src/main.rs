// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use anyhow::Result;
use aptos_gas_schedule_updator::{generate_update_proposal, GenArgs};
use clap::Parser;

fn main() -> Result<()> {
    let args = GenArgs::parse();

    generate_update_proposal(&args)
}
