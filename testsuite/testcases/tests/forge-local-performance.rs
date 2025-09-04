// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_forge::{
    forge_main,
    success_criteria::{StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig, InitialVersion, LocalFactory, Options, Result,
};
use velor_testcases::performance_test::PerformanceBenchmark;
use std::num::NonZeroUsize;

fn main() -> Result<()> {
    ::velor_logger::Logger::init_for_testing();

    let tests = ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(2).unwrap())
        .with_initial_version(InitialVersion::Newest)
        .add_network_test(PerformanceBenchmark)
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 30 })
                .gas_price(velor_global_constants::GAS_UNIT_PRICE),
        )
        .with_success_criteria(SuccessCriteria::new(20).add_chain_progress(
            StateProgressThreshold {
                max_non_epoch_no_progress_secs: 0.0,
                max_epoch_no_progress_secs: 0.0,
                max_non_epoch_round_gap: 0,
                max_epoch_round_gap: 0,
            },
        ));

    let options = Options::parse();
    forge_main(tests, LocalFactory::from_workspace(None)?, &options)
}
