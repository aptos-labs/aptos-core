// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use forge::{forge_main, ForgeConfig, InitialVersion, LocalFactory, Options, Result};
use std::num::NonZeroUsize;
use testcases::{gas_price_test::NonZeroGasPrice, performance_test::PerformanceBenchmark};

fn main() -> Result<()> {
    ::diem_logger::Logger::init_for_testing();

    let tests = ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_version(InitialVersion::Newest)
        .with_network_tests(&[&PerformanceBenchmark, &NonZeroGasPrice]);

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
