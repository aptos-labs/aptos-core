// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use forge::{forge_main, ForgeConfig, LocalFactory, Options, Result};
use smoke_test::{fullnode::LaunchFullnode, indexer, rest_api};

fn main() -> Result<()> {
    let tests = ForgeConfig::default()
        .with_aptos_tests(&[
            &rest_api::GetIndex,
            &rest_api::BasicClient,
            &indexer::Indexer,
        ])
        //TODO Re-enable these tests once we fix how the move compiler is invoked
        // .with_admin_tests(&[&MalformedScript, &ExecuteCustomModuleAndScript])
        .with_network_tests(&[&LaunchFullnode]);

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
