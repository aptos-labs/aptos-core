// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptosdb::db_debugger::Cmd;
use clap::Parser;

fn main() -> Result<()> {
    Cmd::parse().run()
}
