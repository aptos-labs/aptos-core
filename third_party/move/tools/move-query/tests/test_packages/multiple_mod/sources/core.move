// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Core module with friend-restricted functions.
module test_addr::core {
    friend test_addr::admin;

    use test_addr::math;

    /// Internal state struct.
    struct State has key {
        value: u64,
        initialized: bool,
    }

    /// Error code for uninitialized state.
    const E_NOT_INITIALIZED: u64 = 1;

    /// Public function anyone can call.
    public fun get_value(state: &State): u64 {
        state.value
    }

    /// Public function using math module.
    public fun add_to_value(state: &State, amount: u64): u64 {
        math::add(state.value, amount)
    }

    /// Friend-only function for setting value.
    public(friend) fun set_value(state: &mut State, new_value: u64) {
        state.value = new_value;
    }

    /// Friend-only function for initialization.
    public(friend) fun initialize(account: &signer): State {
        let _ = account;
        State { value: 0, initialized: true }
    }

    /// Private internal helper.
    fun validate(state: &State): bool {
        state.initialized
    }
}
