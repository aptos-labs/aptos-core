// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A stand-alone tool to support query commands on move bytecode
//! **Usage**: ` move-querier [OPTIONS] --bytecode-path <BYTECODE_PATH>`
//!
//! - Available `OPTIONS` include (at least one of them must be provided):
//!
//!    - `--dump-call-graph`: Dump the call graph(s) from bytecode.
//!
//!    - `--check-bytecode-type`: Check the type of the bytecode (`script`, `module`, or `unknown`).

use clap::Parser;
use move_querier::querier::{Querier, QuerierOptions};
use std::{fs, process::exit};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Arguments {
    #[clap(long, required = true)]
    pub bytecode_path: String,

    #[clap(flatten)]
    cmd: Option<QuerierOptions>,
}

fn main() {
    let args = Arguments::parse();
    let query_option = if let Some(options) = args.cmd {
        options
    } else {
        println!("Error: no query command provided. For more information, run with --help");
        exit(1);
    };

    if !query_option.has_any_true() {
        println!("Error: No desired command (e.g., --dump-call-graph | --check-bytecode-type) is provided");
        exit(1);
    }

    let bytecode_bytes = match fs::read(&args.bytecode_path) {
        Ok(bytecode) => bytecode,
        Err(e) => {
            println!("Error reading bytecode file: {}", e);
            exit(1);
        },
    };

    let querier = Querier::new(query_option, bytecode_bytes);
    let res = querier.query();
    match res {
        Ok(result) => {
            println!("{}", result);
        },
        Err(e) => {
            println!("Error querying bytecode: {}", e);
            exit(1);
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Arguments::command().debug_assert()
}
