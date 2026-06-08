// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas parameters for the native position subsystem.

use crate::{gas_schedule::NativeGasParameters, ver::gas_feature_versions::RELEASE_V1_48};
use aptos_gas_algebra::{InternalGas, InternalGasPerByte};

crate::gas_schedule::macros::define_gas_parameters!(
    PositionGasParameters,
    "position",
    NativeGasParameters => .position,
    // TODO: placeholder values, not yet benchmark-calibrated.
    [
        [set_position_base: InternalGas, { RELEASE_V1_48.. => "set_position.base" }, 5000],
        [set_position_per_byte: InternalGasPerByte, { RELEASE_V1_48.. => "set_position.per_byte" }, 50],
        [delete_position_base: InternalGas, { RELEASE_V1_48.. => "delete_position.base" }, 3000],
    ]
);
