// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_variables)]

use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct GenerateOpenapi {}

// TODO: To implement this, I first want to make fake implementations of the
// major traits that we need to feed into Api.
pub async fn generate_openapi(args: GenerateOpenapi) -> Result<()> {
    Ok(())
}
