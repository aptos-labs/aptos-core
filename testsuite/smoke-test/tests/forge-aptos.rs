// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use forge::{forge_main, ForgeConfig, LocalFactory, Options, Result};
use smoke_test::{
    aptos::{AccountCreation, ErrorReport, GasCheck, MintTransfer, ModulePublish},
    transaction::ExternalTransactionSigner,
};

fn main() -> Result<()> {
    let tests = ForgeConfig::default()
        .with_aptos_tests(&[
            &AccountCreation,
            &ExternalTransactionSigner,
            &ErrorReport,
            &GasCheck,
            &MintTransfer,
            &ModulePublish,
            &smoke_test::nft_transaction::NFTTransaction,
            // re-enable after delegation is enabled
            // &Staking,
        ])
        .with_genesis_modules_bytes(cached_framework_packages::module_blobs().to_vec());

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
