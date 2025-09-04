// Copyright (c) Velor Foundation
// Parts of the project are originally copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use move_decompiler::{Decompiler, Options};

fn main() {
    let options = Options::parse();
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    if let Err(e) = Decompiler::new(options).run(&mut error_writer) {
        eprintln!("error: {:#}", e);
        std::process::exit(1)
    }
}
