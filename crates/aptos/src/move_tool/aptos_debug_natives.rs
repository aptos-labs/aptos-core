// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use aptos_vm::natives;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::sync::Arc;

// move_stdlib has the testing feature enabled to include debug native functions
pub fn aptos_debug_natives(
    gas_parameters: NativeGasParameters,
    abs_val_size_gas_params: AbstractValueSizeGasParameters,
) -> NativeFunctionTable {
    // As a side effect, also configure for unit testing
    natives::configure_for_unit_test();
    // Return all natives -- build with the 'testing' feature, therefore containing
    // debug related functions.
    natives::aptos_natives(
        gas_parameters,
        abs_val_size_gas_params,
        LATEST_GAS_FEATURE_VERSION,
        TimedFeatures::enable_all(),
        Arc::new(Features::default()),
    )
}
