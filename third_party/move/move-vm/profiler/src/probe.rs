// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A profiler that emits USDT probes.

use move_vm_types::instr::Instruction;
use once_cell::sync::Lazy;
use std::time::Instant;

#[usdt::provider]
mod vm_profiler {
    fn function_entry(function: &str) {}
    fn function_exit(function: &str, nanos: u64) {}

    fn instruction_entry(instruction: &str) {}
    fn instruction_exit(instruction: &str, nanos: u64) {}
}

pub struct ProbeProfiler;

impl Default for ProbeProfiler {
    fn default() -> Self {
        usdt::register_probes().expect("Failed to register probes");
        Self
    }
}

impl Profiler for ProbeProfiler {
    type FnGuard = FunctionProbe;
    type InstrGuard = InstructionProbe;

    #[inline]
    fn function(&self, function: String) -> Self::FnGuard {
        FunctionProbe::new(function)
    }

    #[inline]
    fn instruction(&self, instruction: &Instruction) -> Self::InstrGuard {
        InstructionProbe::new(instruction)
    }
}

pub struct FunctionProbe {
    function: String,
    t0: Instant,
}

impl FunctionProbe {
    #[must_use]
    fn new(function: String) -> Self {
        vm_profiler::function_entry!(|| &function);

        Self {
            function,
            t0: Instant::now(),
        }
    }
}

impl Drop for FunctionProbe {
    fn drop(&mut self) {
        let dt = self.t0.elapsed();
        let nanos = dt.as_nanos() as u64;
        vm_profiler::function_exit!(|| (&self.function, nanos));
    }
}

pub struct InstructionProbe {
    instruction: String,
    t0: Instant,
}

impl InstructionProbe {
    #[must_use]
    fn new(instruction: &Instruction) -> Self {
        let instruction = format!("{instruction:?}");

        vm_profiler::instruction_entry!(|| &instruction);

        Self {
            instruction,
            t0: Instant::now(),
        }
    }
}

impl Drop for InstructionProbe {
    fn drop(&mut self) {
        let dt = self.t0.elapsed();
        let nanos = dt.as_nanos() as u64;
        vm_profiler::instruction_exit!(|| (&self.instruction, nanos));
    }
}
