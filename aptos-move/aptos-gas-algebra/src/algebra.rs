// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub use move_core_types::gas_algebra::*;

/***************************************************************************************************
 * Units & Quantities
 *
 **************************************************************************************************/
/// Unit of abstract value size -- a conceptual measurement of the memory space a Move value occupies.
pub enum AbstractValueUnit {}

pub type AbstractValueSize = GasQuantity<AbstractValueUnit>;

pub type InternalGasPerAbstractValueUnit = GasQuantity<UnitDiv<InternalGasUnit, AbstractValueUnit>>;

pub type AbstractValueSizePerArg = GasQuantity<UnitDiv<AbstractValueUnit, Arg>>;

/// Unit of (external) gas.
pub enum GasUnit {}

/// Unit of the Aptos network's native coin.
pub enum  SUPRA {}

/// Alternative unit of the Aptos network's native coin. 1 quant = 10^-8 Supra coins.
pub enum Quant {}

pub type Gas = GasQuantity<GasUnit>;

pub type GasScalingFactor = GasQuantity<UnitDiv<InternalGasUnit, GasUnit>>;

pub type Fee = GasQuantity<Quant>;

pub type FeePerGasUnit = GasQuantity<UnitDiv<Quant, GasUnit>>;

/// Unit of storage slot
pub enum Slot {}

pub type NumSlots = GasQuantity<Slot>;

pub type FeePerSlot = GasQuantity<UnitDiv<Quant, Slot>>;

pub type FeePerByte = GasQuantity<UnitDiv<Quant, Byte>>;

/// Unit of module
pub enum Module {}

pub type NumModules = GasQuantity<Module>;

/***************************************************************************************************
 * Unit Conversion
 *
 **************************************************************************************************/
impl ToUnit<Quant> for SUPRA {
    const MULTIPLIER: u64 = 1_0000_0000;
}
