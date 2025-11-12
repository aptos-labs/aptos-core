// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_vm_types::instr::Instruction;

#[cfg(feature = "probe-profiler")]
pub mod probe;

#[cfg(feature = "probe-profiler")]
pub static VM_PROFILER: Lazy<ProbeProfiler> = Lazy::new(ProbeProfiler::default);

#[cfg(not(feature = "probe-profiler"))]
pub static VM_PROFILER: NoopProfiler = NoopProfiler;

pub trait Profiler {
    type FnGuard;
    type InstrGuard;

    fn function(&self, function: String) -> Self::FnGuard;
    fn instruction(&self, instruction: &Instruction) -> Self::InstrGuard;
}

pub struct NoopProfiler;

impl Profiler for NoopProfiler {
    type FnGuard = ();
    type InstrGuard = ();

    fn function(&self, _function: String) -> Self::FnGuard {}

    fn instruction(&self, _instruction: &Instruction) -> Self::InstrGuard {}
}

#[cfg(test)]
mod tests {
    use crate::{Profiler, VM_PROFILER};
    use move_vm_types::instr::Instruction;
    use std::{thread::sleep, time::Duration};

    #[test]
    fn test_profiler() {
        let _fg = VM_PROFILER.function("foo".to_string());
        sleep(Duration::from_millis(100));
        execute_instruction(&Instruction::And);
        execute_instruction(&Instruction::Or);
        execute_instruction(&Instruction::Not);
        sleep(Duration::from_millis(100));
    }

    fn execute_instruction(instr: &Instruction) {
        let _ig = VM_PROFILER.instruction(instr);
        sleep(Duration::from_millis(100));
    }
}
