// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::traits::InitialGasParam;
use aptos_gas_algebra::{
    AbstractValueSize, AbstractValueSizePerArg, Fee, FeePerByte, FeePerGasUnit, FeePerSlot, Gas,
    GasScalingFactor, InternalGasPerAbstractValueUnit, InternalGasPerTypeNode, NumModules,
    NumSlots, NumTypeNodes,
};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, InternalGasPerByte, NumBytes};

/// Multiplier applied to the initial values of gas cost parameters:
/// - `InternalGas`
/// - `InternalGasPerAbstractValueUnit`
/// - `InternalGasPerArg`
/// - `InternalGasPerByte`
/// - `InternalGasPerTypeNode`
pub(crate) const GAS_COST_MULTIPLIER: u64 = 1;

/// Multiplier applied to the initial values of storage fee parameters:
/// - `Fee`
/// - `FeePerByte`
/// - `FeePerSlot`
pub(crate) const STORAGE_FEE_MULTIPLIER: u64 = 1;

macro_rules! impl_initial_gas_param {
    // Applies a multiplier to the raw value before converting to the gas parameter type.
    ($multiplier:expr => $($ty:ty),+ $(,)?) => {$(
        impl InitialGasParam for $ty {
            fn from_raw(raw: u64) -> Self {
                (raw * $multiplier).into()
            }
        }
    )+};
    // Directly converts the raw value to the gas parameter type without applying a multiplier.
    ($($ty:ty),+ $(,)?) => {$(
        impl InitialGasParam for $ty {
            fn from_raw(raw: u64) -> Self {
                raw.into()
            }
        }
    )+};
}

impl_initial_gas_param!(GAS_COST_MULTIPLIER =>
    InternalGas,
    InternalGasPerAbstractValueUnit,
    InternalGasPerArg,
    InternalGasPerByte,
    InternalGasPerTypeNode,
);

impl_initial_gas_param!(STORAGE_FEE_MULTIPLIER =>
    Fee,
    FeePerByte,
    FeePerSlot,
);

impl_initial_gas_param!(
    AbstractValueSize,
    AbstractValueSizePerArg,
    Gas,
    GasScalingFactor,
    FeePerGasUnit,
    NumBytes,
    NumModules,
    NumSlots,
    NumTypeNodes,
);
