// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::state::{self, VMState};

/// With the node's panic hook installed, a panic inside a contained deserializer or verifier
/// region must unwind to the caller instead of exiting the process.
#[test]
fn contained_panic_does_not_exit_process() {
    aptos_crash_handler::setup_panic_handler();
    for contained_state in [VMState::DESERIALIZER, VMState::VERIFIER] {
        let result: std::thread::Result<()> =
            state::catch_unwind_in(contained_state, || panic!("injected"));
        assert!(result.is_err());
    }
}
