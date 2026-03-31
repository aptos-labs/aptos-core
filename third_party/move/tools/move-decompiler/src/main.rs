// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
