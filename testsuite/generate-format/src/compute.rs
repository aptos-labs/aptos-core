// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use generate_format::Corpus;
use std::{fs::File, io::Write};

#[derive(Debug, Parser)]
#[clap(
    name = "Velor format generator",
    about = "Trace serde (de)serialization to generate format descriptions for Velor types"
)]
struct Options {
    #[clap(long, value_enum, default_value_t = Corpus::Velor, ignore_case = true)]
    corpus: Corpus,

    #[clap(long)]
    record: bool,
}

fn main() {
    let options = Options::parse();

    let registry = options.corpus.get_registry();
    let output_file = options.corpus.output_file();

    let content = serde_yaml::to_string(&registry).unwrap();
    if options.record {
        match output_file {
            Some(path) => {
                let mut f = File::create("testsuite/generate-format/".to_string() + path).unwrap();
                write!(f, "{}", content).unwrap();
            },
            None => panic!("Corpus {:?} doesn't record formats on disk", options.corpus),
        }
    } else {
        println!("{}", content);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Options::command().debug_assert()
}
