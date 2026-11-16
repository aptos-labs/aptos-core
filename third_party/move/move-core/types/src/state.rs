// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::{cell::RefCell, panic::UnwindSafe};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VMState {
    DESERIALIZER,
    VERIFIER,
    RUNTIME,
    OTHER,
}

thread_local! {
    static STATE: RefCell<VMState> = const { RefCell::new(VMState::OTHER) };
}

/// Runs `body` with this thread's state set to `state`, catching any unwinding panic, then restores
/// the previous state. `state` must be `DESERIALIZER` or `VERIFIER`, the states in which the
/// node's panic hook lets a panic unwind.
pub fn catch_unwind_in<R>(
    state: VMState,
    body: impl FnOnce() -> R + UnwindSafe,
) -> std::thread::Result<R> {
    debug_assert!(contains_panics(state), "{state:?} does not contain panics");
    let prev_state = set_state(state);
    let result = std::panic::catch_unwind(body);
    set_state(prev_state);
    result
}

/// Whether a panic on this thread is caught by an enclosing `catch_unwind_in`, so the node's
/// panic hook must not exit the process.
pub fn panic_is_contained() -> bool {
    contains_panics(get_state())
}

pub fn get_state() -> VMState {
    STATE.with(|cell| *cell.borrow())
}

fn contains_panics(state: VMState) -> bool {
    match state {
        VMState::DESERIALIZER | VMState::VERIFIER => true,
        VMState::RUNTIME | VMState::OTHER => false,
    }
}

fn set_state(state: VMState) -> VMState {
    STATE.with(|cell| cell.replace(state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_and_restores_state_on_return() {
        assert_eq!(get_state(), VMState::OTHER);
        let inner = catch_unwind_in(VMState::DESERIALIZER, get_state).expect("no panic");
        assert_eq!(inner, VMState::DESERIALIZER);
        assert_eq!(get_state(), VMState::OTHER);
    }

    #[test]
    fn nested_calls_restore_outer_state() {
        let outer = catch_unwind_in(VMState::VERIFIER, || {
            let inner: std::thread::Result<()> =
                catch_unwind_in(VMState::DESERIALIZER, || panic!("injected"));
            (inner.is_err(), get_state())
        })
        .expect("no panic");
        assert_eq!(outer, (true, VMState::VERIFIER));
        assert_eq!(get_state(), VMState::OTHER);
    }
}
