// Copyright © Aptos Foundation

#![forbid(unsafe_code)]

use aptos_cargo_cli::AptosCargoCli;
use clap::Parser;
use std::process::exit;

fn main() {
    let result = AptosCargoCli::parse().execute();

    // At this point, we'll want to print and determine whether to exit for an error code
    match result {
        Ok(_) => println!("Done"),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}
