// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Lightweight Move package builder")]
struct Args {
    input_path: std::path::PathBuf,
    #[structopt(
        long,
        short = "o",
        about = "Optional output path, defaults to input_path/out"
    )]
    output_path: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::from_args();

    let build_config = move_package::BuildConfig {
        dev_mode: false,
        generate_abis: false,
        generate_docs: true,
        install_dir: args.output_path,
        ..Default::default()
    };

    build_config
        .compile_package(&args.input_path, &mut std::io::stdout())
        .unwrap();
}
