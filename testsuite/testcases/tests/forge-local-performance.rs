// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use forge::{
    forge_main,
    success_criteria::{StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig, InitialVersion, LocalFactory, Options, Result,
};
use std::num::NonZeroUsize;
use testcases::{gas_price_test::NonZeroGasPrice, performance_test::PerformanceBenchmark};

fn main() -> Result<()> {
    ::aptos_logger::Logger::init_for_testing();

    let tests = ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(2).unwrap())
        .with_initial_version(InitialVersion::Newest)
        .with_network_tests(vec![&PerformanceBenchmark, &NonZeroGasPrice])
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 30 }))
        .with_success_criteria(SuccessCriteria::new(
            20,
            60000,
            false,
            None,
            None,
            Some(StateProgressThreshold {
                max_no_progress_secs: 0.0,
                max_round_gap: 0,
            }),
        ));

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
