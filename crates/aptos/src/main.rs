// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain
//!
//! TODO: Examples
//!
#![forbid(unsafe_code)]

use aptos::Tool;
use std::process::exit;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    // Run the corresponding tools
    let result = Tool::from_args().execute().await;

    // At this point, we'll want to print and determine whether to exit for an error code
    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        }
    }
}
