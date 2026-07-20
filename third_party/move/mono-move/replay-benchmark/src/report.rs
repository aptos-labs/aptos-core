// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction and summary reporting of the timing comparison + correctness verdict.

use crate::{
    compare::{compare_outcomes, Correctness, ExecOutcome},
    BenchmarkRun,
};
use aptos_types::transaction::Version;
use std::time::Duration;

/// Everything computed for a single transaction across both VMs.
pub struct TransactionReport {
    pub version: Version,
    pub function: String,
    pub v1: Option<Result<BenchmarkRun, String>>,
    pub v2: Option<Result<BenchmarkRun, String>>,
    pub correctness: Option<Correctness>,
}

impl TransactionReport {
    pub fn new(
        version: Version,
        function: String,
        v1: Option<Result<BenchmarkRun, String>>,
        v2: Option<Result<BenchmarkRun, String>>,
    ) -> Self {
        // Correctness is only meaningful when both VMs ran. With a single VM selected (profiling
        // mode) there is nothing to compare against.
        let correctness = match (&v1, &v2) {
            (Some(v1), Some(v2)) => Some(match v1 {
                // V1 is the reference. If it could not run, there is nothing to compare against.
                Ok(v1run) => {
                    let v2_outcome = match v2 {
                        Ok(v2run) => Ok(&v2run.outcome),
                        Err(reason) => Err(reason.as_str()),
                    };
                    compare_outcomes(&v1run.outcome, v2_outcome)
                },
                Err(reason) => Correctness::Mismatch {
                    detail: format!(
                        "V1 (reference) could not execute the transaction: {}",
                        reason
                    ),
                },
            }),
            _ => None,
        };
        Self {
            version,
            function,
            v1,
            v2,
            correctness,
        }
    }

    /// The speedup of V2 over V1 (V1_median / V2_median) for matching outcomes. `>1` means V2 is faster.
    pub fn speedup(&self) -> Option<f64> {
        if !matches!(self.correctness, Some(Correctness::Match)) {
            return None;
        }
        let v1 = self.v1.as_ref()?.as_ref().ok()?;
        let v2 = self.v2.as_ref()?.as_ref().ok()?;
        let v2_nanos = v2.samples.median().as_nanos();
        if v2_nanos == 0 {
            return None;
        }
        Some(v1.samples.median().as_nanos() as f64 / v2_nanos as f64)
    }

    pub fn print(&self) {
        println!("Transaction {} — {}", self.version, self.function);
        print_vm("  V1 (legacy MoveVM)", &self.v1);
        print_vm("  V2 (MonoMove)     ", &self.v2);
        // Speedup and correctness only apply when both VMs ran.
        if self.v1.is_some() && self.v2.is_some() {
            match self.speedup() {
                Some(s) => println!("  speedup (V1/V2)    : {:.2}x  ({})", s, speedup_phrase(s)),
                None => {
                    let both_ran = matches!(self.v1, Some(Ok(_))) && matches!(self.v2, Some(Ok(_)));
                    let reason = if !both_ran {
                        "one VM did not run to completion"
                    } else {
                        "outcomes differ — execution times are not comparable"
                    };
                    println!("  speedup (V1/V2)    : n/a ({})", reason);
                },
            }
        }
        match &self.correctness {
            Some(Correctness::Match) => println!("  correctness        : MATCH"),
            Some(Correctness::Mismatch { detail }) => {
                println!("  correctness        : MISMATCH — {}", detail)
            },
            None => {},
        }
        println!();
    }
}

fn print_vm(label: &str, run: &Option<Result<BenchmarkRun, String>>) {
    match run {
        Some(Ok(run)) => {
            let t = &run.samples;
            println!(
                "{}: {} (median over {} samples; spread ±{}, min {}, max {})  →  outcome: {}",
                label,
                fmt_duration(t.median()),
                t.len(),
                fmt_duration(t.stddev()),
                fmt_duration(t.min()),
                fmt_duration(t.max()),
                describe_outcome(&run.outcome),
            );
        },
        Some(Err(reason)) => println!("{}: did not run — {}", label, reason),
        None => println!("{}: skipped", label),
    }
}

fn describe_outcome(outcome: &ExecOutcome) -> String {
    match outcome {
        ExecOutcome::Success { events, writes } => {
            format!(
                "success ({} event(s), {} write(s))",
                events.len(),
                writes.len()
            )
        },
        ExecOutcome::Aborted { code, message } => match message {
            Some(m) => format!("Move abort (code {}: {})", code, m),
            None => format!("Move abort (code {})", code),
        },
        ExecOutcome::Failure { kind, detail } => format!("failure [{}] {}", kind, detail),
    }
}

fn speedup_phrase(s: f64) -> &'static str {
    if s >= 1.0 {
        "V2 faster"
    } else {
        "V1 faster"
    }
}

fn fmt_duration(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns < 1_000 {
        format!("{} ns", ns)
    } else if ns < 1_000_000 {
        format!("{:.2} µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2} ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.3} s", ns as f64 / 1_000_000_000.0)
    }
}
