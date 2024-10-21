// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_storage_interface::cached_state_view::StateCache;
use aptos_types::transaction::{block_epilogue::BlockEndInfo, Transaction, TransactionOutput};

pub struct ExecutionOutput {
    /// Input transactions.
    pub transactions: Vec<Transaction>,
    /// Raw VM output.
    pub transaction_outputs: Vec<TransactionOutput>,
    /// Carries the frozen base state view, so all in-mem nodes involved won't drop before the
    /// execution result is processed; as well as all the accounts touched during execution, together
    /// with their proofs.
    pub state_cache: StateCache,
    /// Optional StateCheckpoint payload
    pub block_end_info: Option<BlockEndInfo>,
}
