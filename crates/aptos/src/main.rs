// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

use aptos::Tool;
use aptos_cli_base::utils::print_cli_result;
use clap::Parser;

#[tokio::main]
async fn main() {
    // Run the corresponding tools
    let result = Tool::parse().execute().await;

    // At this point, we'll want to print and determine whether to exit for an error code
    print_cli_result(result);
}
