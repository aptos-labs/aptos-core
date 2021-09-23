// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "1000000")]
    num_accounts: usize,

    #[structopt(long, default_value = "1000000")]
    init_account_balance: u64,

    #[structopt(long, default_value = "500")]
    block_size: usize,

    #[structopt(long, parse(from_os_str))]
    db_dir: PathBuf,

    #[structopt(long)]
    prune_window: Option<u64>,
}

fn main() {
    let opt = Opt::from_args();

    diemdb_benchmark::run_benchmark(
        opt.num_accounts,
        opt.init_account_balance,
        opt.block_size,
        opt.db_dir,
        opt.prune_window,
    );
}
