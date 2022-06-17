// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Create {}

pub async fn create(args: Create) -> Result<()> {
    Ok(())
}
