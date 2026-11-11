// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A meter with interrupt polling lets the resource provider stop a run once
//! per gas chunk. A meter without polling never asks.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NativeExtensions,
    storage::resource_provider::InMemoryStorageKey,
    types::{InternedType, EMPTY_TYPE_LIST},
    Code, CodeOffset, FrameLayoutInfo, Function, FunctionDefinitionIndex, GasExhaustedError,
    GasMeter, InterruptedError, MicroOp, ResourceProvider, ResourceProviderError,
    SortedSafePointEntries, StorageRead, VMInternalError, VMResult, FRAME_METADATA_SIZE,
};
use mono_move_runtime::InterpreterContext;
use std::cell::Cell;

const BUDGET: u64 = 1_000_000;
const CHUNK_SIZE: u64 = 100;
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

impl ResourceProvider for InterruptAt {
    fn get_resource(
        &self,
        _key: &InMemoryStorageKey,
        _group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        Ok(StorageRead::DoesNotExist)
    }

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

fn polling_meter() -> GasMeter {
    GasMeter::new(BUDGET).with_interrupt_polling(CHUNK_SIZE)
}

fn run<'guard>(ctx: &mut InterpreterContext<'guard>, entry: &'guard Function) -> VMResult<()> {
    ctx.build_call(entry)?.run()?;
    Ok(())
}

/// Result of a run: its error, whether it was interrupted, and the balance.
struct Ran {
    err: Option<VMInternalError>,
    interrupted: bool,
    balance: u64,
}

fn run_with(
    entry: &Function,
    gas_meter: GasMeter,
    provider: &InterruptAt,
    f: impl for<'guard> FnOnce(&mut InterpreterContext<'guard>, &'guard Function) -> VMResult<()>,
) -> Ran {
    common::with_test_interpreter_using(
        entry,
        gas_meter,
        provider,
        NativeExtensions::new(),
        |ctx, entry| Ran {
            err: f(ctx, entry).err(),
            interrupted: ctx.is_interrupted(),
            balance: ctx.gas_balance(),
        },
    )
}

fn assert_interrupted(ran: &Ran) {
    let err = ran.err.as_ref().expect("the run should fail");
    assert!(err.downcast_ref::<InterruptedError>().is_some(), "{err:?}");
    assert!(ran.interrupted);
}

#[test]
fn loop_is_interrupted_at_first_poll() {
    let provider = InterruptAt::new(1);
    let ran = run_with(&spin(), polling_meter(), &provider, run);
    assert_interrupted(&ran);
    assert_eq!(provider.polls.get(), 1);
    // The first chunk is spent; the interrupted charge changes nothing.
    assert_eq!(ran.balance, BUDGET - CHUNK_SIZE);
}

#[test]
fn loop_is_interrupted_after_several_polls() {
    let provider = InterruptAt::new(3);
    let ran = run_with(&spin(), polling_meter(), &provider, run);
    assert_interrupted(&ran);
    assert_eq!(provider.polls.get(), 3);
    // Three chunks, plus the two charges that started the second and third.
    assert_eq!(ran.balance, BUDGET - 3 * CHUNK_SIZE - 2 * JUMP_GAS);
}

#[test]
fn unmetered_loop_is_interrupted() {
    let provider = InterruptAt::new(2);
    let ran = run_with(&spin(), polling_meter(), &provider, |ctx, entry| {
        ctx.unmetered(|ctx| run(ctx, entry))
    });
    assert_interrupted(&ran);
    assert_eq!(provider.polls.get(), 2);
    assert_eq!(ran.balance, BUDGET);
}

#[test]
fn nested_unmetered_loop_is_interrupted() {
    let provider = InterruptAt::new(2);
    let ran = run_with(&spin(), polling_meter(), &provider, |ctx, entry| {
        ctx.unmetered(|ctx| ctx.unmetered(|ctx| run(ctx, entry)))
    });
    assert_interrupted(&ran);
    assert_eq!(provider.polls.get(), 2);
    assert_eq!(ran.balance, BUDGET);
}

#[test]
fn meter_without_polling_runs_out_of_gas() {
    let provider = InterruptAt::new(1);
    let budget = 101;
    let ran = run_with(&spin(), GasMeter::new(budget), &provider, run);
    let err = ran.err.expect("the run should fail");
    assert!(err.downcast_ref::<GasExhaustedError>().is_some(), "{err:?}");
    assert!(!ran.interrupted);
    assert_eq!(provider.polls.get(), 0);
    assert_eq!(ran.balance, budget % JUMP_GAS);
}

#[test]
fn polling_run_out_of_gas_is_not_interrupted() {
    let provider = InterruptAt::new(u64::MAX);
    let budget = 3 * CHUNK_SIZE + 1;
    let gas_meter = GasMeter::new(budget).with_interrupt_polling(CHUNK_SIZE);
    let ran = run_with(&spin(), gas_meter, &provider, run);
    let err = ran.err.expect("the run should fail");
    assert!(err.downcast_ref::<GasExhaustedError>().is_some(), "{err:?}");
    assert!(!ran.interrupted);
    assert_eq!(ran.balance, budget % JUMP_GAS);
}

#[test]
fn completed_run_is_not_interrupted() {
    let provider = InterruptAt::new(1);
    let entry = function(vec![MicroOp::Return]);
    let ran = run_with(&entry, polling_meter(), &provider, run);
    assert!(ran.err.is_none(), "{:?}", ran.err);
    assert!(!ran.interrupted);
    assert_eq!(provider.polls.get(), 0);
}
