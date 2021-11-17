// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use forge::{forge_main, ForgeConfig, LocalFactory, Options, Result};
use smoke_test::{
    fullnode::LaunchFullnode,
    nft_transaction::NFTTransaction,
    scripts_and_modules::{ExecuteCustomModuleAndScript, MalformedScript},
};

fn main() -> Result<()> {
    let tests = ForgeConfig::default()
        // TODO: add other tests here
        .with_nft_public_usage_tests(&[&NFTTransaction])
        .with_admin_tests(&[&MalformedScript, &ExecuteCustomModuleAndScript])
        .with_network_tests(&[&LaunchFullnode])
        .with_genesis_modules_bytes(
            diem_experimental_framework_releases::current_module_blobs().to_vec(),
        );

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
