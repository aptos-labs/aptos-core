// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use move_command_line_common::files::MOVE_ERROR_DESC_EXTENSION;
use move_core_types::errmap::ErrorMapping;
#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// The location (module id) returned with a `MoveAbort` error
    #[clap(long = "location", short = 'l')]
    location: String,
    /// The abort code returned with a `MoveAbort` error
    #[clap(long = "abort-code", short = 'a')]
    abort_code: u64,
    /// Path to the error code mapping file
    #[clap(long = MOVE_ERROR_DESC_EXTENSION, short = 'e')]
    errmap_path: String,
}

fn main() {
    let args = Args::parse();

    let errmap_bytes = std::fs::read(&args.errmap_path).expect("Could not load errmap from file");
    let errmap: ErrorMapping =
        bcs::from_bytes(&errmap_bytes).expect("Failed to deserialize errmap");

    match errmap.get_explanation(&args.location, args.abort_code) {
        None => println!(
            "Unable to find a description for {}::{}",
            args.location, args.abort_code
        ),
        Some(error_desc) => println!(
            "Name: {}\nDescription: {}",
            error_desc.code_name, error_desc.code_description,
        ),
    }
}
