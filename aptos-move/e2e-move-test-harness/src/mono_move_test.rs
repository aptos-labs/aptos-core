// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs a `#[run_mono_move]` test body twice, once per VM. Not meant to be
//! called directly; see the macro's documentation.

use std::{any::Any, cell::Cell, panic, thread};

thread_local! {
    /// Whether harnesses built on this thread should enable `ENABLE_MONO_MOVE`.
    /// A thread-local rather than a parameter, because tests build the harness
    /// inside helpers that the macro cannot rewrite.
    static MONO_MOVE: Cell<bool> = const { Cell::new(false) };
}

/// Whether the harness constructor should turn MonoMove on.
///
/// Only meaningful on the thread running the test body. A harness built on a
/// spawned thread always gets the V1 VM.
pub(crate) fn mono_move_enabled() -> bool {
    MONO_MOVE.with(|mono_move| mono_move.get())
}

fn set_mono_move(enabled: bool) {
    MONO_MOVE.with(|mono_move| mono_move.set(enabled));
}

/// Runs `body` on the V1 VM, and then, if that passed, on MonoMove.
///
/// `should_fail` marks a test known to fail under MonoMove; its reason is
/// documentation and is never matched against the failure.
pub fn run(name: &str, should_fail: Option<&str>, body: &dyn Fn()) {
    match (run_both_passes(body), should_fail) {
        (Ok(()), None) | (Err(_), Some(_)) => {},
        (Ok(()), Some(reason)) => panic!(
            "[MonoMove] `{name}` passed under MonoMove but is marked \
             `should_fail = \"{reason}\"`; remove the marker"
        ),
        (Err(payload), None) => panic!(
            "[MonoMove] `{name}` passed on the V1 VM but failed under MonoMove: {}",
            panic_message(payload.as_ref())
        ),
    }
}

/// Runs `body` on the V1 VM, and then, if that passed, on MonoMove.
///
/// For test bodies `#[run_mono_move]` cannot reach, above all those inside
/// `proptest!`, which is a function-like macro with nowhere to hang an
/// attribute. Two rules apply at every call site:
///
/// * Build the harness *inside* `body`. A harness built before the call runs
///   the V1 VM in both passes and checks nothing.
/// * Signal failure by panicking. `prop_assert!` returns an error instead,
///   which the MonoMove pass would discard.
#[track_caller]
pub fn both(body: impl Fn()) {
    let location = panic::Location::caller();
    if let Err(payload) = run_both_passes(&body) {
        panic!(
            "[MonoMove] the body at {location} passed on the V1 VM but failed \
             under MonoMove: {}",
            panic_message(payload.as_ref())
        );
    }
}

fn run_both_passes(body: &dyn Fn()) -> thread::Result<()> {
    // The V1 pass is not caught: a failure here is an ordinary test failure and
    // must look exactly like one.
    set_mono_move(false);
    body();

    set_mono_move(true);
    // `body` may capture the test's arguments by reference, but it only reads
    // and clones them, and a caught panic is always re-raised below.
    let result = panic::catch_unwind(panic::AssertUnwindSafe(body));
    set_mono_move(false);
    result
}

fn panic_message(payload: &dyn Any) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return msg.to_string();
    }
    payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| "<non-string panic>".to_string())
}
