// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replays a full transaction (entry-function user transactions,
//! block-metadata transactions, and block-epilogue transactions) on
//! both the legacy AptosVM (V1) and the MonoMove-backed Aptos transaction
//! executor (V2), reporting an execution-time comparison and a strict output
//! comparison. Both replays run gas-free so the outputs carry no fee effects.
//! See the `README` for usage.

pub mod capture;
pub mod compare;
pub mod data;
pub mod gas;
pub mod report;
pub mod timing;
pub mod v1;
pub mod v2;

use aptos_types::transaction::TransactionOutput;
pub use data::BenchmarkInput;
pub use timing::Samples;
use timing::{collect_samples, TimingConfig};

/// The result of replaying a transaction on one VM: its (single) materialized
/// output and the collected timing samples for the measured region.
pub struct BenchmarkRun {
    pub outcome: TransactionOutput,
    pub samples: Samples,
}

/// Runs the transaction once untimed — determining the reported output and
/// warming the caches — then times `execute_once` for each sample, keeping
/// the output's deallocation outside the measured region. A failure in any
/// run fails the benchmark rather than biasing the samples.
pub(crate) fn measure(
    timing: &TimingConfig,
    execute_once: impl Fn() -> anyhow::Result<TransactionOutput>,
) -> anyhow::Result<BenchmarkRun> {
    let outcome = execute_once()?;
    let mut first_error = None;
    let samples = collect_samples(timing, || {
        let start = std::time::Instant::now();
        let result = execute_once();
        let elapsed = start.elapsed();
        if let Err(error) = result {
            first_error.get_or_insert(error);
        }
        elapsed
    });
    match first_error {
        Some(error) => Err(error.context("a timed run failed")),
        None => Ok(BenchmarkRun { outcome, samples }),
    }
}
