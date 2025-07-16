// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use move_asm::assembler::{run, Options};

fn main() {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    let options = Options::parse();
    if let Err(e) = run(options, &mut error_writer) {
        eprintln!("error: {:#}", e);
        std::process::exit(1)
    }
}
