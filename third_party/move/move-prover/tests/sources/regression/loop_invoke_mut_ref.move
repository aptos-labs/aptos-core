// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop must forget any value its body can change, including the value behind a
// `&mut` argument passed to a call through a function value.
//
// The loop analysis asks each instruction what it modifies. A direct call reports
// its `&mut` arguments as well as its results, but a call through a function value
// used to report only the explicit results. The value behind the reference was
// therefore not havoced at the loop head, and the prover kept the value it had
// before the loop. The backend already emits the implicit `x := f(x)` return for a
// `&mut` argument of such a call, so only the modification query was out of step.
//
// `apply_in_loop` writes 0 through `x`, then calls `set_one` through a function value
// inside a loop, which writes 1, so `result == 0` must be rejected.
// `invoke_with_destination_in_loop` is the same with a call that also returns a value,
// which pins that the `&mut` arguments are reported even when `dests` is non-empty.

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

    /// Control, outside any loop: the return value of a call through a function value
    /// must still be handled precisely, so this has to keep verifying. It pins that
    /// the fix reports the `&mut` arguments without over-havocking the results.
    public fun invoke_with_destination_control(x: &mut u64): u64 {
        let f: |&mut u64| u64 has copy + drop = set_two_and_return;
        f(x)
    }

    spec invoke_with_destination_control {
        ensures result == 7;
    }
}
