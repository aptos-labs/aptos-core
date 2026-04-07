// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction lifecycle tracing for latency analysis.
//!
//! Tracks individual transactions through the Aptos pipeline stages
//! (mempool → QS batching → consensus → execution → commit) and records
//! timestamps at each stage. Only transactions from configured sender
//! addresses are traced.

pub mod counters;
pub mod filter;
pub mod store;
pub mod types;

pub use filter::TransactionFilter;
pub use store::TransactionTraceStore;
pub use types::{
    BatchInclusionType, ExecutionStatus, StageMetadata, TransactionStage, TransactionTrace,
};
