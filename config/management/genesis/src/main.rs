// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use aptos_genesis_tool::command::Command;
use structopt::StructOpt;

fn main() {
    match Command::from_args().execute() {
        Ok(output) => println!("{}", output),
        Err(err) => {
            println!("Operation unsuccessful: {}", err);
            std::process::exit(1);
        }
    }
}
