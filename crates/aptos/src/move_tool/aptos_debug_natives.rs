// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::extended_checks;
use aptos_gas_schedule::{NativeGasParameters, VMGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{Features, TimedFeaturesBuilder};
use aptos_vm::natives;
use move_vm_runtime::native_functions::NativeFunctionTable;

// move_stdlib has the testing feature enabled to include debug native functions
pub fn aptos_debug_natives(
    native_gas_parameters: NativeGasParameters,
    vm_gas_params: VMGasParameters,
) -> NativeFunctionTable {
    // As a side effect, also configure for unit testing
    natives::configure_for_unit_test();
    extended_checks::configure_extended_checks_for_unit_test();
    // Return all natives -- build with the 'testing' feature, therefore containing
    // debug related functions.
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        native_gas_parameters,
        vm_gas_params,
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}
