// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Change log:
// - V10
//   - Added generate_unique_address and get_txn_hash native functions
//   - Storage gas charges (excluding "storage fees") stop respecting the storage gas curves
// - V9
//   - Accurate tracking of the cost of loading resource groups
// - V8
//   - Added BLS12-381 operations.
// - V7
//   - Native support for exists<T>
//   - New formulae for storage fees based on fixed APT costs
//   - Lower gas price (other than the newly introduced storage fees) by upping the scaling factor
// - V6
//   - Added a new native function - blake2b_256.
// - V5
//   - u16, u32, u256
//   - free_write_bytes_quota
//   - configurable ChangeSetConfigs
// - V4
//   - Consider memory leaked for event natives
// - V3
//   - Add memory quota
//   - Storage charges:
//     - Distinguish between new and existing resources
//     - One item write comes with 1K free bytes
//     - abort with STORAGE_WRITE_LIMIT_REACHED if WriteOps or Events are too large
// - V2
//   - Table
//     - Fix the gas formula for loading resources so that they are consistent with other
//       global operations.
// - V1
//   - TBA
pub const LATEST_GAS_FEATURE_VERSION: u64 = 10;
