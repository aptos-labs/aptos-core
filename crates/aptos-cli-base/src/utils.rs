// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::CliResult;
use std::process::exit;

pub fn print_cli_result(result: CliResult) {
    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        }
    }
}
