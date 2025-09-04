// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// Chain ID of the current chain
pub const X_VELOR_CHAIN_ID: &str = "X-Velor-Chain-Id";
/// Current epoch of the chain
pub const X_VELOR_EPOCH: &str = "X-Velor-Epoch";
/// Current ledger version of the chain
pub const X_VELOR_LEDGER_VERSION: &str = "X-Velor-Ledger-Version";
/// Oldest non-pruned ledger version of the chain
pub const X_VELOR_LEDGER_OLDEST_VERSION: &str = "X-Velor-Ledger-Oldest-Version";
/// Current block height of the chain
pub const X_VELOR_BLOCK_HEIGHT: &str = "X-Velor-Block-Height";
/// Oldest non-pruned block height of the chain
pub const X_VELOR_OLDEST_BLOCK_HEIGHT: &str = "X-Velor-Oldest-Block-Height";
/// Current timestamp of the chain
pub const X_VELOR_LEDGER_TIMESTAMP: &str = "X-Velor-Ledger-TimestampUsec";
/// Cursor used for pagination.
pub const X_VELOR_CURSOR: &str = "X-Velor-Cursor";
/// The cost of the call in terms of gas. Only applicable to calls that result in
/// function execution in the VM, e.g. view functions, txn simulation.
pub const X_VELOR_GAS_USED: &str = "X-Velor-Gas-Used";
/// Provided by the client to identify what client it is.
pub const X_VELOR_CLIENT: &str = "x-velor-client";
