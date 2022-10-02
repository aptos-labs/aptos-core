// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use backup_cli::{
    backup_types::total_supply::verify::{StateManifestOpt, TotalSupplyController},
    storage::StorageOpt,
    utils::GlobalRestoreOpt,
};
use clap::Parser;
use std::convert::TryInto;

#[derive(Parser)]
struct Opt {
    #[clap(flatten)]
    global: GlobalRestoreOpt,
    #[clap(flatten)]
    opt: StateManifestOpt,
    #[clap(subcommand)]
    storage: StorageOpt,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();
    let storage = opt.storage.init_storage().await?;
    let controller = TotalSupplyController::new(opt.opt, opt.global.clone().try_into()?, storage);
    controller.run().await?;
    Ok(())
}
