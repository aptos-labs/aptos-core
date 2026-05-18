// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{Features, TimedFeaturesBuilder};
use aptos_vm::natives;
use move_model::model::GlobalEnv;
use move_vm_runtime::native_functions::NativeFunctionTable;

/// Configures the unit test validation hook to run the extended checker.
fn configure_extended_checks_for_unit_test() {
    fn validate(env: &GlobalEnv) {
        extended_checks::run_extended_checks(env);
    }
    move_unit_test::test_validation::set_validation_hook(Box::new(validate));
}

// move_stdlib has the testing feature enabled to include debug native functions
pub fn aptos_debug_natives(
    native_gas_parameters: NativeGasParameters,
    misc_gas_params: MiscGasParameters,
) -> NativeFunctionTable {
    // As a side effect, also configure for unit testing
    #[cfg(feature = "testing")]
    natives::configure_for_unit_test();
    configure_extended_checks_for_unit_test();
    // Return all natives -- build with the 'testing' feature, therefore containing
    // debug related functions.
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        native_gas_parameters,
        misc_gas_params,
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}
