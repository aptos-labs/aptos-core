// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{Profiler, ProfilerFunction, ProfilerInstruction};
use std::time::Instant;

#[usdt::provider]
mod vm_profiler {
    fn function_entry(function_name: String) {}
    fn function_exit(nanos: u64) {}

    fn instruction_entry(instruction_name: String) {}
    fn instruction_exit(nanos: u64) {}
}

/// A profiler that emits USDT (Userland Statically Defined Tracing) probes.
/// See [usdt](https://crates.io/crates/usdt) for more details.
///
/// It emits the following probes for function and instruction entry/exit:
/// - `function_entry(function_name: String)`
/// - `function_exit(nanos: u64)`
/// - `instruction_entry(instruction_name: String)`
/// - `instruction_exit(nanos: u64)`
/// Note that the exit probes include the elapsed time in nanoseconds.
pub struct ProbeProfiler;

impl Default for ProbeProfiler {
    fn default() -> Self {
        usdt::register_probes().expect("Failed to register probes");
        Self
    }
}

impl Profiler for ProbeProfiler {
    type FnGuard = ProbeFnGuard;
    type InstrGuard = ProbeInstrGuard;

    #[inline]
    fn function_start<F>(&self, function: &F) -> Self::FnGuard
    where
        F: ProfilerFunction,
    {
        ProbeFnGuard::new(function)
    }

    #[inline]
    fn instruction_start<I>(&self, instruction: &I) -> Self::InstrGuard
    where
        I: ProfilerInstruction,
    {
        ProbeInstrGuard::new(instruction)
    }
}

pub struct ProbeFnGuard {
    start: Instant,
}

impl ProbeFnGuard {
    #[must_use]
    fn new<F>(function: &F) -> Self
    where
        F: ProfilerFunction,
    {
        vm_profiler::function_entry!(|| function.name());

        Self {
            start: Instant::now(),
        }
    }
}

impl Drop for ProbeFnGuard {
    fn drop(&mut self) {
        vm_profiler::function_exit!(|| {
            let dt = self.start.elapsed();
            dt.as_nanos() as u64
        });
    }
}

pub struct ProbeInstrGuard {
    start: Instant,
}

impl ProbeInstrGuard {
    #[must_use]
    fn new<I>(instruction: &I) -> Self
    where
        I: ProfilerInstruction,
    {
        vm_profiler::instruction_entry!(|| instruction.name());

        Self {
            start: Instant::now(),
        }
    }
}

impl Drop for ProbeInstrGuard {
    fn drop(&mut self) {
        vm_profiler::instruction_exit!(|| {
            let dt = self.start.elapsed();
            dt.as_nanos() as u64
        });
    }
}
