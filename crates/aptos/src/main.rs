// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

use aptos::Tool;
use aptos_logger::Level;
use clap::Parser;
use std::process::exit;

#[tokio::main]
async fn main() {
    let mut logger = aptos_logger::Logger::new();
    logger
        .channel_size(1000)
        .is_async(false)
        .level(Level::Warn)
        .read_env();
    logger.build();

    // Run the corresponding tools
    let result = Tool::parse().execute().await;

    // At this point, we'll want to print and determine whether to exit for an error code
    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        }
    }
}
