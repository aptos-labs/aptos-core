// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines the trace data structure which is sufficient to replay Move program execution without
//! requiring any data accesses (only access to code loader is needed).

use crate::LoadedFunction;
use bitvec::vec::BitVec;
use move_core_types::function::ClosureMask;
use std::rc::Rc;

/// Records the history of conditional branches (taken or not).
#[derive(Clone)]
pub(crate) struct CondBrTrace {
    /// Bit-vector storing the branch history for conditional branches. The vector consists of 64-
    /// bit blocks that should usually be the fastest on 64-bit CPUs.
    bits: BitVec<u64>,
}

impl CondBrTrace {
    /// Returns an empty branch history.
    pub(crate) fn empty() -> Self {
        let bits = BitVec::new();
        Self { bits }
    }

    /// Returns an empty history with pre-allocated capacity (number of bits).
    pub(crate) fn with_capacity(n: usize) -> Self {
        let bits = BitVec::with_capacity(n);
        Self { bits }
    }

    /// Records outcome of a conditional branch.
    #[inline(always)]
    pub(crate) fn push(&mut self, taken: bool) {
        self.bits.push(taken);
    }
}

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
    /// Log of all branches taken and not taken.
    branches: CondBrTrace,
    /// Log of all functions called via closures. Note that the static calls are not logged to keep
    /// the log smaller (while giving up the ability to resolve calls without data context when
    /// replaying the trace).
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
            branches: CondBrTrace::empty(),
            calls: vec![],
        }
    }

    /// Returns a trace from logged data.
    pub(crate) fn from_logger(ticks: u64, branches: CondBrTrace, calls: Vec<DynamicCall>) -> Self {
        Self {
            ticks,
            branches,
            calls,
        }
    }

    /// Returns true if the trace was fully replayed: all instructions were executed, all branches
    /// taken / not taken, and all dynamic calls processed.
    pub fn is_empty(&self) -> bool {
        self.ticks == 0 && self.branches.bits.len() == 0 && self.calls.len() == 0
    }

    /// For testing purposes only, displays the collected trace.
    #[cfg(any(test, feature = "testing"))]
    pub fn to_string_for_tests(&self) -> String {
        use std::fmt::Write;

        let mut result = String::new();

        writeln!(result, "instructions: {}", self.ticks).unwrap();
        write!(result, "branches: ").unwrap();
        for bit in self.branches.bits.iter() {
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
            && self.branch_cursor == self.trace.branches.bits.len()
            && self.call_cursor == self.trace.calls.len()
    }

    /// Decrements a tick (equivalent to replay of an instruction). The caller must ensure it does
    /// not underflow.
    #[inline(always)]
    pub(crate) fn consume_instruction_unchecked(&mut self) {
        self.instructions_remaining -= 1;
    }

    /// Processes a conditional branch. Returns [None] if branch was not recorded.
    #[inline(always)]
    pub(crate) fn consume_cond_br(&mut self) -> Option<bool> {
        let i = self.branch_cursor;
        if i < self.trace.branches.bits.len() {
            self.branch_cursor = i + 1;
            Some(self.trace.branches.bits[i])
        } else {
            None
        }
    }

    /// Processes an entrypoint. Returns [None] if entrypoint call was not recorded.
    #[inline(always)]
    pub(crate) fn consume_entrypoint(&mut self) -> Option<Rc<LoadedFunction>> {
        let target = self.trace.calls.get(self.call_cursor)?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Entrypoint(target) => Some(Rc::new(target.clone())),
            DynamicCall::Closure(_, _) => None,
        }
    }

    /// Processes a closure. Returns [None] if closure call was not recorded.
    #[inline(always)]
    pub(crate) fn consume_closure_call(&mut self) -> Option<(Rc<LoadedFunction>, ClosureMask)> {
        let target = self.trace.calls.get(self.call_cursor)?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Closure(target, mask) => Some((Rc::new(target.clone()), *mask)),
            DynamicCall::Entrypoint(_) => None,
        }
    }
}
