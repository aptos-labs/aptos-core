// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_gas_schedule_updator::{generate_update_proposal, GenerateNewSchedule};
use clap::Parser;

fn main() -> Result<()> {
    let args = GenerateNewSchedule::parse();

    generate_update_proposal(&args)
}
