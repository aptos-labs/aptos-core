// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crate defines all the gas parameters utilized by the Velor VM & native functions.
//! It also provides traits for conversion between different representations.
//!
//! To define a new gas parameter, simply add a new entry to the appropriate file in the
//! `src/gas_schedule` sub-directory. You will need to provide:
//!
//!   1. A name for the gas parameter. By convention, the name should be prefixed with the
//!      operation (instruction, native function etc.) it is associated with, and be
//!      descriptive of what it is about.
//!
//!   2. The type of the gas parameter, which should be a quantity with a specific unit.
//!      The strong typing helps prevent common programming mistakes.
//!
//!   3. The on-chain name(s) of the gas parameter. You can express multi-versioned views
//!      using the pattern match syntax, allowing the removal or renaming of gas parameters
//!      in the on-chain gas schedule.
//!
//!   4. The current value of the gas parameter. This is used for testing & generating
//!      gas schedule update proposals. Keep in mind that the value will only take effect
//!      on chain after an update proposal including it has been applied.
//!
//!
//! The generation macro will automatically generate a struct type representing the gas parameter,
//! which can be then used in gas expressions once imported.

mod gas_schedule;
mod traits;
mod ver;

pub use gas_schedule::*;
pub use traits::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule};
pub use ver::{gas_feature_versions, LATEST_GAS_FEATURE_VERSION};
