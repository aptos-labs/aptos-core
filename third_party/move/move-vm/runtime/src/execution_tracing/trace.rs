// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines the trace data structure which is sufficient to replay Move program execution without
//! requiring any data accesses (only access to code loader is needed).

use crate::{execution_tracing::recorders::BytecodeFingerprintRecorder, LoadedFunction};
use bitvec::vec::BitVec;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::function::ClosureMask;
use move_vm_types::instr::Instruction;

/// A non-static call record in the trace. Used for entry-points and closures.
#[derive(Clone)]
pub(crate) enum DynamicCall {
    Entrypoint(LoadedFunction),
    Closure(LoadedFunction, ClosureMask),
}

/// Trace of execution of a program that records information sufficient to replay executed
/// instructions:
///   1. Number of successfully executed instructions (ticks).
///   2. Outcomes of every executed conditional branch.
///   3. A vector of functions called via closures.
#[derive(Clone)]
pub struct Trace {
    /// Number of successfully executed instructions.
    ticks: u64,
    /// Fingerprint of all successfully executed instructions.
    fingerprint: u64,
    /// Record of all outcomes of conditional branches (taken and not taken).
    branch_outcomes: BitVec,
    /// Record of all functions called via closures. Note that the static calls are not recorded to
    /// keep the trace smaller (while giving up the ability to resolve calls without data context
    /// when replaying the trace).
    calls: Vec<DynamicCall>,
}

impl Default for Trace {
    fn default() -> Self {
        Self::empty()
    }
}

impl Trace {
    /// Returns an empty trace.
    pub fn empty() -> Self {
        Self {
            ticks: 0,
            fingerprint: 0,
            branch_outcomes: BitVec::new(),
            calls: vec![],
        }
    }

    /// Returns a trace from recorded data.
    pub(crate) fn from_recorder(
        ticks: u64,
        fingerprint: u64,
        branch_outcomes: BitVec,
        calls: Vec<DynamicCall>,
    ) -> Self {
        Self {
            ticks,
            fingerprint,
            branch_outcomes,
            calls,
        }
    }

    /// Returns true if the trace has no recorded instructions and no branches / calls recorded.
    pub fn is_empty(&self) -> bool {
        self.ticks == 0 && self.branch_outcomes.len() == 0 && self.calls.len() == 0
    }

    /// Returns the number of recorded instructions.
    pub fn num_recorded_instructions(&self) -> u64 {
        self.ticks
    }

    /// Returns the number of recorded conditional branch outcomes.
    pub fn num_recorded_branch_outcomes(&self) -> usize {
        self.branch_outcomes.len()
    }

    /// Returns the number of recorded dynamic calls.
    pub fn num_recorded_calls(&self) -> usize {
        self.calls.len()
    }

    /// For testing purposes only, displays the collected trace.
    #[cfg(any(test, feature = "testing"))]
    pub fn to_string_for_tests(&self) -> String {
        use std::fmt::Write;

        let mut result = String::new();

        writeln!(result, "instructions: {}", self.ticks).unwrap();
        writeln!(result, "fingerprint: {}", self.fingerprint).unwrap();
        write!(result, "branch_outcomes: ").unwrap();
        for bit in self.branch_outcomes.iter() {
            result.push(if *bit { '1' } else { '0' });
        }
        result.push('\n');

        let ty_args_str = |f: &LoadedFunction| {
            if f.ty_args().is_empty() {
                "".to_string()
            } else {
                // We have only runtime types available, so keep as is.
                "<..>".to_string()
            }
        };

        writeln!(result, "calls:").unwrap();
        for call in &self.calls {
            match call {
                DynamicCall::Entrypoint(func) => {
                    writeln!(
                        result,
                        "  entrypoint {}{}",
                        func.name_as_pretty_string(),
                        ty_args_str(func)
                    )
                    .unwrap();
                },
                DynamicCall::Closure(func, mask) => {
                    writeln!(
                        result,
                        "  closure {}{} {}",
                        func.name_as_pretty_string(),
                        ty_args_str(func),
                        mask
                    )
                    .unwrap();
                },
            }
        }

        result
    }
}

/// Replays the trace keeping track of executed branches and dynamic calls so far.
pub struct TraceCursor<'a> {
    /// Trace to replay.
    trace: &'a Trace,
    /// Number of instructions still left to replay.
    instructions_remaining: u64,
    /// Fingerprint of the replayed trace.
    fingerprint_recorder: BytecodeFingerprintRecorder,
    /// Index into next branch target to consume, initially 0.
    branch_cursor: usize,
    /// Index into next call target to consume, initially 0.
    call_cursor: usize,
}

impl<'a> TraceCursor<'a> {
    /// Returns a new cursor for replaying the provided trace.
    pub fn new(trace: &'a Trace) -> Self {
        Self {
            trace,
            instructions_remaining: trace.ticks,
            fingerprint_recorder: BytecodeFingerprintRecorder::default(),
            branch_cursor: 0,
            call_cursor: 0,
        }
    }

    /// Returns true if there are no instructions remaining to replay. The caller is responsible to
    /// ensure the invariant that all branch targets and all closure call targets are consumed
    /// holds. This is a cheap check and should be used in replay interpreter loop instead of
    /// [Self::is_done].
    #[inline(always)]
    pub(crate) fn no_instructions_remaining(&self) -> bool {
        self.instructions_remaining == 0
    }

    /// Returns true if the trace is fully replayed - no instructions remaining, no more branches
    /// to consume, no more dynamic calls to consume. Do not use in the hot interpreter loop and
    /// prefer [Self::no_instructions_remaining] for a faster check.
    pub(crate) fn is_done(&self) -> bool {
        self.no_instructions_remaining()
            && self.fingerprint_recorder.finish() == self.trace.fingerprint
            && self.branch_cursor == self.trace.branch_outcomes.len()
            && self.call_cursor == self.trace.calls.len()
    }

    /// Decrements a tick (equivalent to replay of an instruction). The caller must ensure it does
    /// not underflow.
    #[inline(always)]
    pub(crate) fn consume_instruction_unchecked(&mut self, instr: &Instruction) {
        self.instructions_remaining -= 1;
        self.fingerprint_recorder.record(instr);
    }

    /// Processes a conditional branch. Returns [Err] if branch was not recorded.
    #[inline(always)]
    pub(crate) fn consume_branch(&mut self) -> PartialVMResult<bool> {
        let i = self.branch_cursor;
        if i < self.trace.branch_outcomes.len() {
            self.branch_cursor = i + 1;
            Ok(self.trace.branch_outcomes[i])
        } else {
            Err(PartialVMError::new_invariant_violation(
                "All conditional branches must be recorded",
            ))
        }
    }

    /// Processes an entrypoint. Returns [Err] if entrypoint call was not recorded.
    #[inline(always)]
    pub(crate) fn consume_entrypoint(&mut self) -> PartialVMResult<&LoadedFunction> {
        let target = self
            .trace
            .calls
            .get(self.call_cursor)
            .ok_or_else(|| PartialVMError::new_invariant_violation("Entrypoint not found"))?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Entrypoint(target) => Ok(target),
            DynamicCall::Closure(_, _) => Err(PartialVMError::new_invariant_violation(
                "Expected to consume an entrypoint, but found a closure",
            )),
        }
    }

    /// Processes a closure. Returns [Err] if closure call was not recorded.
    #[inline(always)]
    pub(crate) fn consume_closure_call(
        &mut self,
    ) -> PartialVMResult<(&LoadedFunction, ClosureMask)> {
        let target = self
            .trace
            .calls
            .get(self.call_cursor)
            .ok_or_else(|| PartialVMError::new_invariant_violation("Closure not found"))?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Closure(target, mask) => Ok((target, *mask)),
            DynamicCall::Entrypoint(_) => Err(PartialVMError::new_invariant_violation(
                "Expected to consume a closure, but found an entrypoint",
            )),
        }
    }
}
