// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use clap::Parser;
use std::collections::BTreeMap;

#[derive(Parser)]
#[clap(author, version, about = "Lightweight Move package builder", long_about = None)]
struct Args {
    input_path: std::path::PathBuf,
    output_path: Option<std::path::PathBuf>,
    #[clap(short, long, value_name = "ADDRESS_NAME")]
    address_name: Option<String>,
    #[clap(short, long, value_name = "ADDRESS_HEX_STR")]
    hex_address: Option<String>,
}

fn main() {
    let args = Args::parse();
    let named_address = if let Some(value) = args.address_name {
        BTreeMap::from([(
            value,
            AccountAddress::from_hex_literal(args.hex_address.unwrap().as_str()).unwrap(),
        )])
    } else {
        BTreeMap::new()
    };

    let build_config = move_deps::move_package::BuildConfig {
        dev_mode: false,
        generate_abis: false,
        generate_docs: true,
        install_dir: args.output_path,
        additional_named_addresses: named_address,
        ..Default::default()
    };

    build_config
        .compile_package(&args.input_path, &mut std::io::stdout())
        .unwrap();
}
