// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use forge::{forge_main, ForgeConfig, LocalFactory, Options, Result};
use smoke_test::{
    event_fetcher::EventFetcher,
    replay_tooling::ReplayTooling,
    scripts_and_modules::{ExecuteCustomModuleAndScript, MalformedScript},
    transaction::ExternalTransactionSigner,
    verifying_client::{VerifyingClientEquivalence, VerifyingGetLatestMetadata, VerifyingSubmit},
};

fn main() -> Result<()> {
    let tests = ForgeConfig::default()
        .with_public_usage_tests(&[
            &EventFetcher,
            &ExternalTransactionSigner,
            &ReplayTooling,
            &VerifyingSubmit,
            &VerifyingClientEquivalence,
            &VerifyingGetLatestMetadata,
        ])
        .with_admin_tests(&[&MalformedScript, &ExecuteCustomModuleAndScript]);

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
