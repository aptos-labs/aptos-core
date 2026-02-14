// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Chain ID of the current chain
pub const X_APTOS_CHAIN_ID: &str = "X-Aptos-Chain-Id";
/// Current epoch of the chain
pub const X_APTOS_EPOCH: &str = "X-Aptos-Epoch";
/// Current ledger version of the chain
pub const X_APTOS_LEDGER_VERSION: &str = "X-Aptos-Ledger-Version";
/// Oldest non-pruned ledger version of the chain
pub const X_APTOS_LEDGER_OLDEST_VERSION: &str = "X-Aptos-Ledger-Oldest-Version";
/// Current block height of the chain
pub const X_APTOS_BLOCK_HEIGHT: &str = "X-Aptos-Block-Height";
/// Oldest non-pruned block height of the chain
pub const X_APTOS_OLDEST_BLOCK_HEIGHT: &str = "X-Aptos-Oldest-Block-Height";
/// Current timestamp of the chain
pub const X_APTOS_LEDGER_TIMESTAMP: &str = "X-Aptos-Ledger-TimestampUsec";
/// Cursor used for pagination.
pub const X_APTOS_CURSOR: &str = "X-Aptos-Cursor";
/// The cost of the call in terms of gas. Only applicable to calls that result in
/// function execution in the VM, e.g. view functions, txn simulation.
pub const X_APTOS_GAS_USED: &str = "X-Aptos-Gas-Used";
/// Provided by the client to identify what client it is.
pub const X_APTOS_CLIENT: &str = "x-aptos-client";
/// Per-epoch transaction encryption key (hex-encoded)
pub const X_APTOS_TXN_ENCRYPTION_KEY: &str = "X-Aptos-Txn-Encryption-Key";
/// W3C Trace Context traceparent header for distributed tracing
pub const TRACEPARENT: &str = "traceparent";
