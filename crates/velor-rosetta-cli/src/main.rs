// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Velor Rosetta CLI
//!
//! Why have an Velor version of the Rosetta CLI?
//!
//! The Rosetta CLI doesn't build on my Mac easily and I just wanted something simple to test out
//! the POST requests
//!
//! Why have a separate CLI?
//!
//! We want users to use the Velor CLI over the Rosetta CLI because of the added complexity of a
//! proxy server.  So, we split it out so general users aren't confused.
//!
//! TODO: Make Velor CLI framework common among multiple CLIs

#![forbid(unsafe_code)]

mod account;
mod block;
mod common;
mod construction;
mod network;

use crate::common::{ErrorWrapper, RosettaCliArgs};
use velor_logger::Level;
use clap::Parser;
use std::process::exit;

#[tokio::main]
async fn main() {
    let mut logger = velor_logger::Logger::new();
    logger.channel_size(1000).is_async(false).level(Level::Warn);
    logger.build();

    let args: RosettaCliArgs = RosettaCliArgs::parse();

    let result = args.execute().await;

    match result {
        Ok(value) => println!("{}", value),
        Err(error) => {
            let error = ErrorWrapper {
                error: error.to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&error).unwrap());
            exit(-1)
        },
    }
}
