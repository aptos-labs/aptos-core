// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

/// Unit of type tag pseudo gas -- an internal budget bounding how large a type tag may get, not
/// part of the transaction's gas. See `ty_tag_pseudo_gas_cost` in the Move VM for the formula.
pub enum TypeTagPseudoGasUnit {}

/// Pseudo gas cost of a type tag, i.e. a measure of its size.
pub type NumTypeTagPseudoGasUnits = GasQuantity<TypeTagPseudoGasUnit>;

/// Rate converting a type tag's pseudo gas cost into abstract value units.
pub type AbstractValueSizePerTypeTagPseudoGasUnit =
    GasQuantity<UnitDiv<AbstractValueUnit, TypeTagPseudoGasUnit>>;

/// Unit of (external) gas.
pub enum GasUnit {}

/// Unit of the Aptos network's native coin.
pub enum APT {}

/// Alternative unit of the Aptos network's native coin. 1 Octa = 10^-8 Aptos coins.
pub enum Octa {}

pub type Gas = GasQuantity<GasUnit>;

pub type GasScalingFactor = GasQuantity<UnitDiv<InternalGasUnit, GasUnit>>;

pub type Fee = GasQuantity<Octa>;

pub type FeePerGasUnit = GasQuantity<UnitDiv<Octa, GasUnit>>;

/// Unit of storage slot
pub enum Slot {}

pub type NumSlots = GasQuantity<Slot>;

pub type FeePerSlot = GasQuantity<UnitDiv<Octa, Slot>>;

pub type FeePerByte = GasQuantity<UnitDiv<Octa, Byte>>;

/// Unit of module
pub enum Module {}

pub type NumModules = GasQuantity<Module>;

/***************************************************************************************************
 * Unit Conversion
 *
 **************************************************************************************************/
impl ToUnit<Octa> for APT {
    const MULTIPLIER: u64 = 1_0000_0000;
}
