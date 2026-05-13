// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
