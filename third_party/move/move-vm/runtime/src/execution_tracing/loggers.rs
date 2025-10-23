// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Interfaces and implementations for [Trace] collection.

use crate::{
    execution_tracing::{
        trace::{CondBrTrace, DynamicCall},
        Trace,
    },
    LoadedFunction,
};
use move_core_types::function::ClosureMask;

/// Interface for recording the trace at runtime. It is sufficient to record branch decisions as
/// well as dynamic function calls originating from closures.
pub trait TraceLogger {
    /// Returns true if the trace is being collected.
    fn is_enabled(&self) -> bool;

    /// Called in the end of execution to produce a final trace, suitable for replay.
    fn finish(self) -> Trace;

    /// Called after successful execution of a bytecode instruction. It is crucial that the trace
    /// records only successful instructions.
    fn record_successful_instruction(&mut self);

    /// Called for every successfully executed conditional branch.
    fn record_branch(&mut self, taken: bool);

    /// Called for every successful set-up of the entrypoint (entry function or script). That is,
    /// setting up frame, stack, and other structures before actually executing the bytecode.
    fn record_entrypoint(&mut self, function: &LoadedFunction);

    /// Called for every successful set-up of the closure call (i.e., immediately before the first
    /// instruction of the callee is executed).
    fn record_call_closure(&mut self, function: &LoadedFunction, mask: ClosureMask);
}

/// Logger that collects the full trace of execution. Records the number of successfully executed
/// instructions, branch outcomes and closure calls.
pub struct FullTraceLogger {
    /// Number of successfully executed instructions.
    ticks: u64,
    /// Branch outcomes.
    branches: CondBrTrace,
    /// Dynamic call outcomes.
    calls: Vec<DynamicCall>,
}

impl FullTraceLogger {
    /// Returns a new empty logger ready for trace collection.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            ticks: 0,
            branches: CondBrTrace::with_capacity(64),
            calls: vec![],
        }
    }
}

impl TraceLogger for FullTraceLogger {
    #[inline(always)]
    fn is_enabled(&self) -> bool {
        true
    }

    fn finish(self) -> Trace {
        Trace::from_logger(self.ticks, self.branches, self.calls)
    }

    #[inline(always)]
    fn record_successful_instruction(&mut self) {
        self.ticks += 1;
    }

    #[inline(always)]
    fn record_branch(&mut self, taken: bool) {
        self.branches.push(taken);
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

/// No-op instance of logger in case there is no need to collect execution trace at runtime.
pub struct NoOpTraceLogger;

impl TraceLogger for NoOpTraceLogger {
    #[inline(always)]
    fn is_enabled(&self) -> bool {
        false
    }

    fn finish(self) -> Trace {
        Trace::empty()
    }

    #[inline(always)]
    fn record_successful_instruction(&mut self) {}

    #[inline(always)]
    fn record_branch(&mut self, _taken: bool) {}

    #[inline(always)]
    fn record_entrypoint(&mut self, _function: &LoadedFunction) {}

    #[inline(always)]
    fn record_call_closure(&mut self, _function: &LoadedFunction, _mask: ClosureMask) {}
}

#[cfg(test)]
mod testing {
    use super::*;
    use crate::execution_tracing::TraceCursor;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn test_full_loger_is_enabled() {
        let logger = FullTraceLogger::new();
        assert!(logger.is_enabled());

        let logger = NoOpTraceLogger;
        assert!(!logger.is_enabled());
    }

    #[test]
    fn test_ticks_recorded() {
        let mut logger = FullTraceLogger::new();
        assert_eq!(logger.ticks, 0);

        logger.record_successful_instruction();
        assert_eq!(logger.ticks, 1);

        for _ in 0..10 {
            logger.record_successful_instruction();
        }
        assert_eq!(logger.ticks, 11);
    }

    #[test]
    fn test_branches_recorded() {
        let mut logger = FullTraceLogger::new();

        let expected = [
            true, true, false, true, false, false, false, true, false, false, true, true, true,
        ];
        for taken in expected {
            logger.record_branch(taken);
        }

        let trace = logger.finish();
        assert!(!trace.is_empty());

        let mut cursor = TraceCursor::new(&trace);
        for taken in expected {
            let recorded = cursor.consume_cond_br();
            assert_some_eq!(recorded, taken);
        }
        assert_none!(cursor.consume_cond_br());
        assert!(cursor.is_done());
    }
}
