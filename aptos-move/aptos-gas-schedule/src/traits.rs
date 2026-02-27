// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::collections::BTreeMap;

/// A trait for converting from a map representation of the on-chain gas schedule.
pub trait FromOnChainGasSchedule: Sized {
    /// Constructs a value of this type from a map representation of the on-chain gas schedule.
    /// `None` should be returned when the gas schedule is missing some required entries.
    /// Unused entries should be safely ignored.
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String>;
}

/// A trait for converting to a list of entries of the on-chain gas schedule.
pub trait ToOnChainGasSchedule {
    /// Converts `self` into a list of entries of the on-chain gas schedule.
    /// Each entry is a key-value pair where the key is a string representing the name of the
    /// parameter, where the value is the gas parameter itself.
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)>;
}

/// Constructs an initial gas parameter from a raw `u64`, applying a multiplier where appropriate.
///
/// Implemented for all gas parameter types used in this crate:
/// - Gas cost types apply [`crate::scaling::GAS_COST_MULTIPLIER`].
/// - Storage fee types apply [`crate::scaling::STORAGE_FEE_MULTIPLIER`].
/// - Limit, quota, and unit-conversion types pass the value through unchanged.
pub trait InitialGasParam: Sized {
    fn from_raw(raw: u64) -> Self;
}

/// A trait for defining an initial value to be used in the genesis.
pub trait InitialGasSchedule: Sized {
    /// Returns the initial value of this type, which is used in the genesis.
    fn initial() -> Self;
}
