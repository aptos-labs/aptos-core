// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Integer representing the latest gas feature version.
/// The general rule is that this should bumped exactly once after each release, provided there
/// exists some gas related changes. Such changes include:
///   - New gas parameters being added, removed or renamed
///   - Changing how gas is calculated in any way
///
/// Change log:
/// - V31:
///    - Gas charging for modules used in type tags
///
/// - V22
///    - Gas parameters for enums
///    - Gas parameters for new native function `bcs::serialized_size`
/// - V21
///   - Fix type to type tag conversion in MoveVM
/// - V20
///   - Limits for bounding MoveVM type sizes
/// - V19
///   - Gas for aggregator_v2::is_at_least native function
/// - V18
///   - Separate limits for governance scripts
///   - Function info & dispatchable token gas params
/// - V17
///   - Gas for keyless
/// - V16
///   - IO Gas for the transaction itself and events in the transaction output
/// - V15
///   - Gas & limits for dependencies
/// - V14
///   - Gas for type creation
///   - Storage Fee: Make state bytes refundable and remove the per slot free quota, gated by flag REFUNDABLE_BYTES
/// - V13
///   (skipped due to testnet mis-operation)
/// - V12
///   - Added BN254 operations.
///   - IO gas change: 1. read bytes charged at 4KB intervals; 2. ignore free_write_bytes_quota
///   - aggregator v2 gas charges
/// - V11
///   - Ristretto255 natives (point cloning & double-scalar multiplication) and Bulletproofs natives
///   - Hard limit on the number of write ops per transaction
/// - V10
///   - Added generate_unique_address and get_txn_hash native functions
///   - Storage gas charges (excluding "storage fees") stop respecting the storage gas curves
/// - V9
///   - Accurate tracking of the cost of loading resource groups
/// - V8
///   - Added BLS12-381 operations.
/// - V7
///   - Native support for `exists<T>`
///   - New formulae for storage fees based on fixed APT costs
///   - Lower gas price (other than the newly introduced storage fees) by upping the scaling factor
/// - V6
///   - Added a new native function - blake2b_256.
/// - V5
///   - u16, u32, u256
///   - free_write_bytes_quota
///   - configurable ChangeSetConfigs
/// - V4
///   - Consider memory leaked for event natives
/// - V3
///   - Add memory quota
///   - Storage charges:
///     - Distinguish between new and existing resources
///     - One item write comes with 1K free bytes
///     - abort with STORAGE_WRITE_LIMIT_REACHED if WriteOps or Events are too large
/// - V2
///   - Table
///     - Fix the gas formula for loading resources so that they are consistent with other
///       global operations.
/// - V1
///   - TBA
pub const LATEST_GAS_FEATURE_VERSION: u64 = gas_feature_versions::RELEASE_V1_29;

pub mod gas_feature_versions {
    pub const RELEASE_V1_8: u64 = 11;
    pub const RELEASE_V1_9_SKIPPED: u64 = 12;
    pub const RELEASE_V1_9: u64 = 13;
    pub const RELEASE_V1_10: u64 = 15;
    pub const RELEASE_V1_11: u64 = 16;
    pub const RELEASE_V1_12: u64 = 17;
    pub const RELEASE_V1_13: u64 = 18;
    pub const RELEASE_V1_14: u64 = 19;
    pub const RELEASE_V1_15: u64 = 20;
    pub const RELEASE_V1_16: u64 = 21;
    pub const RELEASE_V1_18: u64 = 22;
    pub const RELEASE_V1_19: u64 = 23;
    pub const RELEASE_V1_20: u64 = 24;
    pub const RELEASE_V1_21: u64 = 25;
    pub const RELEASE_V1_22: u64 = 26;
    pub const RELEASE_V1_23: u64 = 27;
    pub const RELEASE_V1_24: u64 = 28;
    pub const RELEASE_V1_26: u64 = 30;
    pub const RELEASE_V1_27: u64 = 31;
    pub const RELEASE_V1_28: u64 = 32;
    pub const RELEASE_V1_29: u64 = 33;
    pub const RELEASE_V1_30: u64 = 34;
}
