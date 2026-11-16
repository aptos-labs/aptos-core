// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A meter with an interrupt listener lets its source stop a run once per
//! interval. A meter without one never asks.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NativeExtensions, types::EMPTY_TYPE_LIST, Code, CodeOffset, FrameLayoutInfo, Function,
    FunctionDefinitionIndex, GasMeter, InterruptListener, InterruptSource, MeteringError, MicroOp,
    NoResourceProvider, SortedSafePointEntries, VMInternalError, VMResult, FRAME_METADATA_SIZE,
};
use mono_move_runtime::InterpreterContext;
use std::cell::Cell;

const BUDGET: u64 = 1_000_000;
const INTERVAL_GAS_UNITS: u64 = 100;
const JUMP_GAS: u64 = 2;

/// Requests an interrupt from its `fire_at`-th poll on, counting from 1.
struct InterruptAt {
    fire_at: u64,
    polls: Cell<u64>,
}

impl InterruptAt {
    fn new(fire_at: u64) -> Self {
        Self {
            fire_at,
            polls: Cell::new(0),
        }
    }
}

impl InterruptSource for InterruptAt {
    fn interrupt_requested(&self) -> bool {
        let polls = self.polls.get() + 1;
        self.polls.set(polls);
        polls >= self.fire_at
    }
}

fn function(code: Vec<MicroOp>) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        def_idx: FunctionDefinitionIndex(0),
        has_module_lock: false,
        code: Code::from_vec(code),
        entry_gas: 0,
        param_slots: vec![],
        param_tys: vec![],
        return_slots: vec![],
        return_tys: EMPTY_TYPE_LIST,
        param_region_size: 0,
        param_and_local_sizes_sum: 0,
        extended_frame_size: FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

fn spin() -> Function {
    function(vec![MicroOp::Jump {
        target: CodeOffset(0),
        gas: JUMP_GAS,
    }])
}

fn listening_meter(source: &InterruptAt) -> GasMeter<'_> {
    GasMeter::new(BUDGET)
        .with_interrupt_listener(InterruptListener::new(source, INTERVAL_GAS_UNITS))
}

fn run<'guard>(ctx: &mut InterpreterContext<'guard>, entry: &'guard Function) -> VMResult<()> {
    ctx.build_call(entry)?.run()?;
    Ok(())
}

/// Result of a run: its error and the balance.
struct Ran {
    err: Option<VMInternalError>,
    balance: u64,
}

impl Ran {
    fn metering_error(&self) -> Option<MeteringError> {
        self.err
            .as_ref()
            .and_then(|err| err.downcast_ref::<MeteringError>())
            .copied()
    }
}

fn run_with(
    entry: &Function,
    gas_meter: GasMeter,
    f: impl for<'guard> FnOnce(&mut InterpreterContext<'guard>, &'guard Function) -> VMResult<()>,
) -> Ran {
    common::with_test_interpreter_using(
        entry,
        gas_meter,
        &NoResourceProvider,
        NativeExtensions::new(),
        |ctx, entry| Ran {
            err: f(ctx, entry).err(),
            balance: ctx.gas_balance(),
        },
    )
}

fn assert_interrupted(ran: &Ran) {
    assert_eq!(
        ran.metering_error(),
        Some(MeteringError::Interrupted),
        "{:?}",
        ran.err
    );
}

#[test]
fn loop_is_interrupted_at_first_poll() {
    let source = InterruptAt::new(1);
    let ran = run_with(&spin(), listening_meter(&source), run);
    assert_interrupted(&ran);
    assert_eq!(source.polls.get(), 1);
    // The first interval is spent; the interrupted charge changes nothing.
    assert_eq!(ran.balance, BUDGET - INTERVAL_GAS_UNITS);
}

#[test]
fn loop_is_interrupted_after_several_polls() {
    let source = InterruptAt::new(3);
    let ran = run_with(&spin(), listening_meter(&source), run);
    assert_interrupted(&ran);
    assert_eq!(source.polls.get(), 3);
    // Three intervals, plus the two charges that started the second and third.
    assert_eq!(ran.balance, BUDGET - 3 * INTERVAL_GAS_UNITS - 2 * JUMP_GAS);
}

#[test]
fn unmetered_loop_is_interrupted() {
    let source = InterruptAt::new(2);
    let ran = run_with(&spin(), listening_meter(&source), |ctx, entry| {
        ctx.unmetered(|ctx| run(ctx, entry))
    });
    assert_interrupted(&ran);
    assert_eq!(source.polls.get(), 2);
    assert_eq!(ran.balance, BUDGET);
}

#[test]
fn nested_unmetered_loop_is_interrupted() {
    let source = InterruptAt::new(2);
    let ran = run_with(&spin(), listening_meter(&source), |ctx, entry| {
        ctx.unmetered(|ctx| ctx.unmetered(|ctx| run(ctx, entry)))
    });
    assert_interrupted(&ran);
    assert_eq!(source.polls.get(), 2);
    assert_eq!(ran.balance, BUDGET);
}

#[test]
fn meter_without_listener_runs_out_of_gas() {
    let budget = 101;
    let ran = run_with(&spin(), GasMeter::new(budget), run);
    assert_eq!(ran.metering_error(), Some(MeteringError::OutOfGas));
    assert_eq!(ran.balance, budget % JUMP_GAS);
}

#[test]
fn listening_meter_runs_out_of_gas() {
    let source = InterruptAt::new(u64::MAX);
    let budget = 3 * INTERVAL_GAS_UNITS + 1;
    let gas_meter = GasMeter::new(budget)
        .with_interrupt_listener(InterruptListener::new(&source, INTERVAL_GAS_UNITS));
    let ran = run_with(&spin(), gas_meter, run);
    assert_eq!(ran.metering_error(), Some(MeteringError::OutOfGas));
    assert_eq!(ran.balance, budget % JUMP_GAS);
}

#[test]
fn jump_spanning_several_intervals_polls_once() {
    let source = InterruptAt::new(u64::MAX);
    let jump_gas = 3 * INTERVAL_GAS_UNITS + 7;
    let entry = function(vec![
        MicroOp::Jump {
            target: CodeOffset(1),
            gas: jump_gas,
        },
        MicroOp::Return,
    ]);
    let ran = run_with(&entry, listening_meter(&source), run);
    assert!(ran.err.is_none(), "{:?}", ran.err);
    assert_eq!(source.polls.get(), 1);
    assert_eq!(ran.balance, BUDGET - jump_gas);
}

#[test]
fn completed_run_is_not_interrupted() {
    let source = InterruptAt::new(1);
    let entry = function(vec![MicroOp::Return]);
    let ran = run_with(&entry, listening_meter(&source), run);
    assert!(ran.err.is_none(), "{:?}", ran.err);
    assert_eq!(source.polls.get(), 0);
}
