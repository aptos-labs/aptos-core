// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_gas_schedule_updator::GasScheduleGenerator;
use clap::Parser;

fn main() -> Result<()> {
    GasScheduleGenerator::parse().execute()
}
