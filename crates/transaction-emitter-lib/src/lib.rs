// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod args;
mod cluster;
pub mod emitter;
mod instance;
mod wrappers;

// These are the top level things you should need to run the emitter.
pub use args::{ClusterArgs, CoinSourceArgs, CreateAccountsArgs, EmitArgs};
// We export these if you want finer grained control.
pub use cluster::Cluster;
pub use emitter::{
    query_sequence_number, query_sequence_numbers,
    stats::{TxnStats, TxnStatsRate},
    EmitJob, EmitJobMode, EmitJobRequest, EmitModeParams, TxnEmitter,
};
pub use wrappers::{create_accounts_command, emit_transactions, emit_transactions_with_cluster};
