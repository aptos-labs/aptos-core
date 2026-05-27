// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Interfaces and implementations for [Trace] collection.

use crate::{
    execution_tracing::{trace::DynamicCall, Trace},
    LoadedFunction,
};
use bitvec::vec::BitVec;
use fxhash::FxHasher64;
use move_core_types::function::ClosureMask;
use move_vm_types::instr::Instruction;
use std::hash::{Hash, Hasher};

/// Interface for recording the trace at runtime. It is sufficient to record branch decisions as
/// well as dynamic function calls originating from closures.
pub trait TraceRecorder {
    /// Returns true if the trace is being collected.
    fn is_enabled(&self) -> bool;

    /// Called in the end of execution to produce a final trace, suitable for replay.
    fn finish(self) -> Trace;

    /// Called after successful execution of a bytecode instruction. It is crucial that the trace
    /// records only successful instructions.
    fn record_successful_instruction(&mut self, instr: &Instruction);

    /// Called for every successfully executed conditional branch.
    fn record_branch_outcome(&mut self, taken: bool);

    /// Called for every successful set-up of the entrypoint (entry function or script). That is,
    /// setting up frame, stack, and other structures before actually executing the bytecode.
    fn record_entrypoint(&mut self, function: &LoadedFunction);

    /// Called for every successful set-up of the closure call (i.e., immediately before the first
    /// instruction of the callee is executed).
    fn record_call_closure(&mut self, function: &LoadedFunction, mask: ClosureMask);
}

/// Records the fingerprint of executed bytecode instructions to check trace replay integrity.
#[derive(Default)]
pub(crate) struct BytecodeFingerprintRecorder {
    // Use fast hasher as we do not care about collisions but mostly about performance.
    hasher: FxHasher64,
}

impl BytecodeFingerprintRecorder {
    pub(crate) fn record(&mut self, instr: &Instruction) {
        instr.hash(&mut self.hasher);
    }

    pub(crate) fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

/// Recorder that collects the full trace of execution. Records the number of successfully executed
/// instructions, branch outcomes and closure calls.
pub struct FullTraceRecorder {
    /// Number of successfully executed instructions.
    ticks: u64,
    /// Records the fingerprint of the trace, for extra security.
    fingerprint_recorder: BytecodeFingerprintRecorder,
    /// Branch outcomes (taken or not taken), stored as a bit-vector.
    branch_outcomes: BitVec,
    /// Dynamic call outcomes.
    calls: Vec<DynamicCall>,
}

impl FullTraceRecorder {
    /// Returns a new empty recorder ready for trace collection.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            ticks: 0,
            fingerprint_recorder: BytecodeFingerprintRecorder::default(),
            branch_outcomes: BitVec::with_capacity(64),
            calls: vec![],
        }
    }
}

impl TraceRecorder for FullTraceRecorder {
    #[inline(always)]
    fn is_enabled(&self) -> bool {
        true
    }

    fn finish(self) -> Trace {
        Trace::from_recorder(
            self.ticks,
            self.fingerprint_recorder.finish(),
            self.branch_outcomes,
            self.calls,
        )
    }

    #[inline(always)]
    fn record_successful_instruction(&mut self, instr: &Instruction) {
        self.ticks += 1;
        self.fingerprint_recorder.record(instr);
    }

    #[inline(always)]
    fn record_branch_outcome(&mut self, taken: bool) {
        self.branch_outcomes.push(taken);
    }

    #[inline(always)]
    fn record_entrypoint(&mut self, function: &LoadedFunction) {
        self.calls.push(DynamicCall::Entrypoint(function.clone()));
    }

    #[inline(always)]
    fn record_call_closure(&mut self, function: &LoadedFunction, mask: ClosureMask) {
        self.calls
            .push(DynamicCall::Closure(function.clone(), mask));
    }
}

/// No-op instance of recorder in case there is no need to collect execution trace at runtime.
pub struct NoOpTraceRecorder;

impl TraceRecorder for NoOpTraceRecorder {
    #[inline(always)]
    fn is_enabled(&self) -> bool {
        false
    }

    fn finish(self) -> Trace {
        Trace::empty()
    }

    #[inline(always)]
    fn record_successful_instruction(&mut self, _instr: &Instruction) {}

    #[inline(always)]
    fn record_branch_outcome(&mut self, _taken: bool) {}

    #[inline(always)]
    fn record_entrypoint(&mut self, _function: &LoadedFunction) {}

    #[inline(always)]
    fn record_call_closure(&mut self, _function: &LoadedFunction, _mask: ClosureMask) {}
}

#[cfg(test)]
mod testing {
    use super::*;
    use crate::execution_tracing::TraceCursor;
    use claims::assert_ok_eq;

    #[test]
    fn test_full_recorder_is_enabled() {
        let recorder = FullTraceRecorder::new();
        assert!(recorder.is_enabled());

        let recorder = NoOpTraceRecorder;
        assert!(!recorder.is_enabled());
    }

    #[test]
    fn test_ticks_recorded() {
        let mut recorder = FullTraceRecorder::new();
        assert_eq!(recorder.ticks, 0);

        recorder.record_successful_instruction(&Instruction::Nop);
        assert_eq!(recorder.ticks, 1);

        for _ in 0..10 {
            recorder.record_successful_instruction(&Instruction::Nop);
        }
        assert_eq!(recorder.ticks, 11);
    }

    #[test]
    fn test_branches_recorded() {
        let mut recorder = FullTraceRecorder::new();

        let expected = [
            true, true, false, true, false, false, false, true, false, false, true, true, true,
        ];
        for taken in expected {
            recorder.record_branch_outcome(taken);
        }

        let trace = recorder.finish();
        assert!(!trace.is_empty());

        let mut cursor = TraceCursor::new(&trace);
        for taken in expected {
            let recorded = cursor.consume_branch();
            assert_ok_eq!(recorded, taken);
        }
        assert!(cursor.consume_branch().is_err());
        assert!(cursor.is_done());
    }
}
