// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![forbid(missing_debug_implementations)]

mod args;
mod atomic_histogram;
mod cluster;
mod emit;
mod instance;
mod wrappers;

// These are the top level things you should need to run the emitter.
pub use args::{ClusterArgs, EmitArgs};
pub use emit::{TxnStats, TxnStatsRate};
pub use wrappers::emit_transactions;

// We export these if you want finer grained control.
pub use cluster::Cluster;
pub use emit::{query_sequence_numbers, EmitJob, EmitJobRequest, EmitThreadParams, TxnEmitter};
pub use wrappers::emit_transactions_with_cluster;
