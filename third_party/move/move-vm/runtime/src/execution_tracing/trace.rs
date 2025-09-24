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
    /// Bit-vector storing the branch history for conditional branches.
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
    /// Index into next branch target to consume, initially 0.
    branch_cursor: usize,

    /// Log of all functions called via closures. Note that the static calls are not logged to keep
    /// the log smaller (while giving up the ability to resolve calls without data context when
    /// replaying the trace).
    calls: Vec<DynamicCall>,
    /// Index into next call target to consume, initially 0.
    call_cursor: usize,
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
            branch_cursor: 0,
            calls: vec![],
            call_cursor: 0,
        }
    }

    /// Returns a trace from logged data, ready for replay.
    pub(crate) fn from_logger(ticks: u64, branches: CondBrTrace, calls: Vec<DynamicCall>) -> Self {
        Self {
            ticks,
            branches,
            branch_cursor: 0,
            calls,
            call_cursor: 0,
        }
    }

    /// Returns true if all instructions from the trace have been replayed (i.e., the number of
    /// ticks has dropped to 0). The caller is responsible to ensure the invariant that all branch
    /// targets and all closure call targets are consumed holds. This is a cheap check and should
    /// be used in replay interpreter loop instead of [Self::is_empty].
    #[inline(always)]
    pub(crate) fn is_done(&self) -> bool {
        self.ticks == 0
    }

    /// Returns true if the trace was fully replayed: all instructions were executed, all branches
    /// taken / not taken, and all dynamic calls processed.
    pub fn is_empty(&self) -> bool {
        self.ticks == 0
            && self.branch_cursor == self.branches.bits.len()
            && self.call_cursor == self.calls.len()
    }

    /// Decrements a tick (equivalent to replay of an instruction). The caller must ensure it does
    /// not underflow.
    #[inline(always)]
    pub(crate) fn consume_instruction_unchecked(&mut self) {
        self.ticks -= 1;
    }

    /// Processes a conditional branch. Returns [None] if branch was not recorded.
    #[inline(always)]
    pub(crate) fn consume_cond_br(&mut self) -> Option<bool> {
        let i = self.branch_cursor;
        if i < self.branches.bits.len() {
            self.branch_cursor = i + 1;
            Some(self.branches.bits[i])
        } else {
            None
        }
    }

    /// Processes a dynamic call (from closure). Returns [None] if call was not recorded.
    #[inline(always)]
    pub(crate) fn consume_entrypoint(&mut self) -> Option<Rc<LoadedFunction>> {
        let target = self.calls.get(self.call_cursor)?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Entrypoint(target) => Some(Rc::new(target.clone())),
            DynamicCall::Closure(_, _) => None,
        }
    }

    /// Processes a dynamic call (from closure). Returns [None] if call was not recorded.
    #[inline(always)]
    pub(crate) fn consume_closure_call(&mut self) -> Option<(Rc<LoadedFunction>, ClosureMask)> {
        let target = self.calls.get(self.call_cursor)?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Closure(target, mask) => Some((Rc::new(target.clone()), *mask)),
            DynamicCall::Entrypoint(_) => None,
        }
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
