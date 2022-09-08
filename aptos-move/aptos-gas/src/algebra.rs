// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::{GasQuantity, InternalGasUnit, UnitDiv};

pub use gas_algebra_ext::{
    AbstractValueSize, AbstractValueSizePerArg, AbstractValueUnit, InternalGasPerAbstractValueUnit,
};

/// Unit of (external) gas.
pub enum GasUnit {}

/// Unit of gas currency. 1 Octa = 10^-8 Aptos coins.
pub enum Octa {}

pub type Gas = GasQuantity<GasUnit>;

pub type GasScalingFactor = GasQuantity<UnitDiv<InternalGasUnit, GasUnit>>;

pub type Fee = GasQuantity<Octa>;

pub type FeePerGasUnit = GasQuantity<UnitDiv<Octa, GasUnit>>;
