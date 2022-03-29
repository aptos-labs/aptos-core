// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

#[derive(Debug, Parser)]
#[clap(about = "Lightweight Move package builder")]
struct Args {
    input_path: std::path::PathBuf,
    #[clap(long, short = 'o')]
    output_path: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::parse();

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
