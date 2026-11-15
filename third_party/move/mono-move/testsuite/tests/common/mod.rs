// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs Move unit tests on V1 and MonoMove side by side.

// Each test binary uses a subset.
#![allow(dead_code)]

use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{
    aptos_test_feature_flags_genesis, Features, TimedFeaturesBuilder,
};
use aptos_vm::natives;
use mono_move_unit_test_backend::MonoMoveBackendFactory;
use move_model::model::GlobalEnv;
use move_package::BuildConfig;
use move_unit_test::{
    backend::Backends,
    package_test::{run_move_unit_tests_with_backend, UnitTestResult},
    test_reporter::UnitTestFactoryWithCostTable,
    test_validation,
    v1_backend::V1BackendFactory,
    UnitTestingConfig,
};
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::path::Path;

/// V1's natives, with the unit-test extensions and the extended checks
/// configured.
fn aptos_test_natives() -> NativeFunctionTable {
    fn validate(env: &GlobalEnv) {
        extended_checks::run_extended_checks(env);
    }
    natives::configure_for_unit_test();
    test_validation::set_validation_hook(Box::new(validate));
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

/// Builds the V1 backend with Aptos test natives.
pub fn v1_backend() -> V1BackendFactory<UnitTestFactoryWithCostTable> {
    V1BackendFactory::new(
        Some(aptos_test_natives()),
        true,
        UnitTestFactoryWithCostTable::new(None, None),
    )
}

/// Runs the package at `path` on V1 and MonoMove. Panics on runner errors,
/// failed tests, or backend mismatches. Unsupported and incomparable tests are
/// reported without failing the run.
pub fn run_package_on_both(path: &Path, build_config: BuildConfig) {
    let v1 = v1_backend();
    let mono = MonoMoveBackendFactory::new(u64::MAX);
    let result = run_move_unit_tests_with_backend(
        path,
        build_config,
        UnitTestingConfig {
            allow_inconclusive: true,
            ..UnitTestingConfig::default()
        },
        Backends::Differential {
            reference: &v1,
            candidate: &mono,
        },
        aptos_test_feature_flags_genesis(),
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    )
    .unwrap_or_else(|err| panic!("failed to run unit tests in {}: {err:#}", path.display()));
    assert_eq!(
        result,
        UnitTestResult::Success,
        "unit tests failed or disagreed in {}",
        path.display()
    );
}
