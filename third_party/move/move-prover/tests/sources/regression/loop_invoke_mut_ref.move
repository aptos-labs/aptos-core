// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop havocs the value behind a `&mut` argument passed to a call through a function value,
// including when the call also returns a value.

module 0x42::loop_invoke_mut_ref {

    fun set_one(x: &mut u64) {
        *x = 1;
    }

    fun set_two_and_return(x: &mut u64): u64 {
        *x = 2;
        7
    }

    public fun apply_in_loop(x: &mut u64): u64 {
        *x = 0;
        let f: |&mut u64| has copy + drop = set_one;
        let i = 0;
        while (i < 1) {
            f(x);
            i += 1;
        };
        *x
    }

    spec apply_in_loop {
        ensures result == 0; // error: the loop body writes 1 through `x`
    }

    public fun invoke_with_destination_in_loop(x: &mut u64): u64 {
        *x = 0;
        let f: |&mut u64| u64 has copy + drop = set_two_and_return;
        let i = 0;
        while (i < 1) {
            let _call_result = f(x);
            i += 1;
        };
        *x
    }

    spec invoke_with_destination_in_loop {
        ensures result == 0; // error: the loop body writes 2 through `x`
    }

    /// Must verify: outside a loop, the result of a call through a function value stays
    /// precise.
    public fun invoke_with_destination_control(x: &mut u64): u64 {
        let f: |&mut u64| u64 has copy + drop = set_two_and_return;
        f(x)
    }

    spec invoke_with_destination_control {
        ensures result == 7;
    }
}
