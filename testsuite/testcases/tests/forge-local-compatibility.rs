// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_forge::{forge_main, ForgeConfig, InitialVersion, LocalFactory, Options, Result};
use aptos_testcases::compatibility_test::SimpleValidatorUpgrade;
use std::num::NonZeroUsize;

fn main() -> Result<()> {
    ::aptos_logger::Logger::init_for_testing();

    let tests = ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_version(InitialVersion::Oldest)
        .add_network_test(SimpleValidatorUpgrade);

    let options = Options::parse();
    forge_main(
        tests,
        LocalFactory::with_upstream_merge_base_and_workspace()?,
        &options,
    )
}
