// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A profiler that emits USDT (Userland Statically Defined Tracing) probes.
//! See [usdt](https://crates.io/crates/usdt).

use crate::{Profiler, ProfilerFunction, ProfilerInstruction};
use std::time::Instant;

#[usdt::provider]
mod vm_profiler {
    fn function_entry(function_name: &str) {}
    fn function_exit(function_name: &str, nanos: u64) {}

    fn instruction_entry(instruction_name: &str) {}
    fn instruction_exit(instruction_name: &str, nanos: u64) {}
}

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
    fn function<F>(&self, function: &F) -> Self::FnGuard
    where
        F: ProfilerFunction,
    {
        ProbeFnGuard::new(function)
    }

    #[inline]
    fn instruction<I>(&self, instruction: &I) -> Self::InstrGuard
    where
        I: ProfilerInstruction,
    {
        ProbeInstrGuard::new(instruction)
    }
}

pub struct ProbeFnGuard {
    function_name: String,
    start: Instant,
}

impl ProbeFnGuard {
    #[must_use]
    fn new<F>(function: &F) -> Self
    where
        F: ProfilerFunction,
    {
        let function_name = function.name();

        vm_profiler::function_entry!(|| &function_name);

        Self {
            function_name,
            start: Instant::now(),
        }
    }
}

impl Drop for ProbeFnGuard {
    fn drop(&mut self) {
        vm_profiler::function_exit!(|| {
            let dt = self.start.elapsed();
            let nanos = dt.as_nanos() as u64;
            (&self.function_name, nanos)
        });
    }
}

pub struct ProbeInstrGuard {
    instruction_name: String,
    start: Instant,
}

impl ProbeInstrGuard {
    #[must_use]
    fn new<I>(instruction: &I) -> Self
    where
        I: ProfilerInstruction,
    {
        let instruction_name = instruction.name();

        vm_profiler::instruction_entry!(|| &instruction_name);

        Self {
            instruction_name,
            start: Instant::now(),
        }
    }
}

impl Drop for ProbeInstrGuard {
    fn drop(&mut self) {
        vm_profiler::instruction_exit!(|| {
            let dt = self.start.elapsed();
            let nanos = dt.as_nanos() as u64;
            (&self.instruction_name, nanos)
        });
    }
}
