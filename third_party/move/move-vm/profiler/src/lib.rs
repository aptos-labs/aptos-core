// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_vm_types::instr::Instruction;
use once_cell::sync::Lazy;
#[cfg(feature = "probe-profiler")]
use probe::ProbeProfiler;

#[cfg(feature = "probe-profiler")]
pub mod probe;

#[cfg(feature = "probe-profiler")]
pub type ActiveProfiler = ProbeProfiler;

#[cfg(not(feature = "probe-profiler"))]
pub type ActiveProfiler = NoopProfiler;

pub type FnGuard = <ActiveProfiler as Profiler>::FnGuard;

pub static VM_PROFILER: Lazy<ActiveProfiler> = Lazy::new(ActiveProfiler::default);

/// A function that can be profiled.
pub trait ProfilerFunction {
    fn name(&self) -> String;
}

/// An instruction that can be profiled.
pub trait ProfilerInstruction {
    fn name(&self) -> String;
}

impl ProfilerInstruction for Instruction {
    fn name(&self) -> String {
        self.name().to_string()
    }
}

/// A profiler for Move VM execution.
pub trait Profiler {
    type FnGuard;
    type InstrGuard;

    /// Start profiling a function and return a guard.
    /// The guard ends profiling when dropped, so it should be held for the duration of the function execution.
    fn function_start<F>(&self, function: &F) -> Self::FnGuard
    where
        F: ProfilerFunction;

    /// Start profiling an instruction and return a guard.
    /// The guard ends profiling when dropped, so it should be held for the duration of the instruction execution.
    fn instruction_start<I>(&self, instruction: &I) -> Self::InstrGuard
    where
        I: ProfilerInstruction;
}

pub struct NoopFnGuard;
pub struct NoopInstrGuard;

/// A no-op profiler that does nothing.
#[derive(Default)]
pub struct NoopProfiler;

impl Profiler for NoopProfiler {
    type FnGuard = NoopFnGuard;
    type InstrGuard = NoopInstrGuard;

    fn function_start<F>(&self, _function: &F) -> Self::FnGuard
    where
        F: ProfilerFunction,
    {
        NoopFnGuard
    }

    fn instruction_start<I>(&self, _instruction: &I) -> Self::InstrGuard
    where
        I: ProfilerInstruction,
    {
        NoopInstrGuard
    }
}

#[cfg(test)]
mod tests {
    use crate::{Profiler, ProfilerFunction, VM_PROFILER};
    use move_vm_types::instr::Instruction;
    use std::{thread::sleep, time::Duration};

    struct DummyFunction<'a>(&'a str);

    impl ProfilerFunction for DummyFunction<'_> {
        fn name(&self) -> String {
            self.0.to_string()
        }
    }

    #[test]
    fn test_profiler() {
        let _fg = VM_PROFILER.function_start(&DummyFunction("foo"));
        sleep(Duration::from_millis(100));
        execute_instruction(&Instruction::And);
        execute_instruction(&Instruction::Or);
        execute_instruction(&Instruction::Not);
        sleep(Duration::from_millis(100));
    }

    fn execute_instruction(instr: &Instruction) {
        let _ig = VM_PROFILER.instruction_start(instr);
        sleep(Duration::from_millis(100));
    }
}
