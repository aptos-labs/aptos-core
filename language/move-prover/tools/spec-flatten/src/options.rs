// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use structopt::StructOpt;

use move_lang::shared::{parse_named_address, AddressBytes};

/// Options passed into the specification flattening tool.
#[derive(StructOpt)]
pub struct FlattenOptions {
    /// Sources of the target modules
    pub srcs: Vec<String>,

    /// Dependencies
    #[structopt(short = "d", long = "dependency")]
    pub deps: Vec<String>,

    /// Do not include default named address
    #[structopt(long = "no-default-named-addresses")]
    pub no_default_named_addresses: bool,

    /// Extra mappings for named address
    #[structopt(short = "a", long = "address", parse(try_from_str = parse_named_address))]
    pub named_addresses_extra: Option<Vec<(String, AddressBytes)>>,

    /// Verbose mode
    #[structopt(short, long)]
    pub verbose: bool,
}
