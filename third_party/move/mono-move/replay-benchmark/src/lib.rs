// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replays an entry-function transaction on both the legacy Move VM (V1) and MonoMove (V2),
//! reporting an execution-time comparison and a coarse correctness check. See the `README` for
//! usage.

pub mod capture;
pub mod compare;
pub mod data;
pub mod report;
pub mod timing;
pub mod v1;
pub mod v2;

pub use compare::{compare_outcomes, Correctness, ExecOutcome};
pub use data::{BenchmarkInput, ReadSet};
pub use report::TransactionReport;
pub use timing::{Samples, TimingConfig};

/// The result of running a transaction's entry function on one VM: its (single) execution outcome
/// and the collected timing samples for the measured region.
pub struct BenchmarkRun {
    pub outcome: ExecOutcome,
    pub samples: Samples,
}
