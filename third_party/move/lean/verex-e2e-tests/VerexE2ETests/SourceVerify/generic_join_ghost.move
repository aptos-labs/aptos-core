// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The unit declares a spec ghost, so a generic callee runs in its caller's
// walk: a join states what it owes at the carriers the type arguments induce,
// while the arriving paths state their facts at the caller's, and
// `settle_token` is a parked gap. `pick_amount` is the control.
module 0x42::generic_join_ghost {
    struct Handle<phantom T> has copy, drop, store { inner: address }
    struct Token has key {}
    struct Fast has key { current: u64 }
    struct Slow has key { current: u64, log: vector<u64> }
    spec module { global settled: num; }

    fun settle<T: key>(handle: &Handle<T>, amount: u64) acquires Fast, Slow {
        if (amount == 0) return;
        let owner = handle.inner;
        if (exists<Fast>(owner)) {
            let fast = &mut Fast[owner];
            assert!(fast.current >= amount, 1);
            fast.current = fast.current - amount;
        } else if (exists<Slow>(owner)) {
            let slow = &mut Slow[owner];
            assert!(slow.current >= amount, 2);
            let current = &mut slow.current;
            *current = *current - amount;
        } else {
            abort 3
        }
    }

    fun settle_token(handle: Handle<Token>, amount: u64) acquires Fast, Slow {
        settle(&handle, amount);
    }
    spec settle_token {
        aborts_if amount != 0 && !exists<Fast>(handle.inner) && !exists<Slow>(handle.inner);
        aborts_if amount != 0 && exists<Fast>(handle.inner) && Fast[handle.inner].current < amount;
        aborts_if amount != 0 && !exists<Fast>(handle.inner) && exists<Slow>(handle.inner) && Slow[handle.inner].current < amount;
    }

    fun pick<T: copy + drop>(a: T, b: T, first: bool): T {
        if (first) a else b
    }

    fun pick_amount(a: u64, b: u64, first: bool): u64 {
        pick(a, b, first)
    }
    spec pick_amount {
        aborts_if false;
    }
}
