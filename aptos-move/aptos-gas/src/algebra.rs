// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::{Arg, GasQuantity, InternalGasUnit, UnitDiv};

/// Unit of (external) gas.
pub enum GasUnit {}

/// Unit of gas currency. 1 Octa = 10^-8 Aptos coins.
pub enum Octa {}

/// Unit of abstract value size -- a conceptual measurement of the memory space a Move value occupies.
pub enum AbstractValueUnit {}

pub type Gas = GasQuantity<GasUnit>;

pub type GasScalingFactor = GasQuantity<UnitDiv<InternalGasUnit, GasUnit>>;

pub type Fee = GasQuantity<Octa>;

pub type FeePerGasUnit = GasQuantity<UnitDiv<Octa, GasUnit>>;

pub type AbstractValueSize = GasQuantity<AbstractValueUnit>;

pub type InternalGasPerAbstractValueUnit = GasQuantity<UnitDiv<InternalGasUnit, AbstractValueUnit>>;

pub type AbstractValueSizePerArg = GasQuantity<UnitDiv<AbstractValueUnit, Arg>>;
