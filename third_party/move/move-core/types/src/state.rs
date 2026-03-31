// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::cell::RefCell;

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

pub fn set_state(state: VMState) -> VMState {
    STATE.with(|s| s.replace(state))
}

pub fn get_state() -> VMState {
    STATE.with(|s| *s.borrow())
}
