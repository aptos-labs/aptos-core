// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use move_asm::run;

fn main() {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    if let Err(e) = run(&mut error_writer) {
        eprintln!("error: {:#}", e);
        std::process::exit(1)
    }
}
