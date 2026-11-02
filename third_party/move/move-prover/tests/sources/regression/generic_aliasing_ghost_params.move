// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Ghost type parameters added for writes under a generic invariant stay within the limit on
// aliasing cases.

module 0x42::generic_aliasing_ghost_params {

    struct R<phantom T> has key { value: bool }
    struct S has key { v: u64 }

    spec module {
        invariant<T> exists<R<T>>(@0x42) ==> exists<S>(@0x42);
    }

    /// Must verify.
    public fun seven_writes() acquires S {
        borrow_global_mut<S>(@0x42).v = 1;
        borrow_global_mut<S>(@0x42).v = 2;
        borrow_global_mut<S>(@0x42).v = 3;
        borrow_global_mut<S>(@0x42).v = 4;
        borrow_global_mut<S>(@0x42).v = 5;
        borrow_global_mut<S>(@0x42).v = 6;
        borrow_global_mut<S>(@0x42).v = 7;
    }

    /// Must verify: nine writes give nine ghosts, each of which can coincide with `T`.
    public fun nine_writes_generic<T>(): bool acquires S, R {
        borrow_global_mut<S>(@0x42).v = 1;
        borrow_global_mut<S>(@0x42).v = 2;
        borrow_global_mut<S>(@0x42).v = 3;
        borrow_global_mut<S>(@0x42).v = 4;
        borrow_global_mut<S>(@0x42).v = 5;
        borrow_global_mut<S>(@0x42).v = 6;
        borrow_global_mut<S>(@0x42).v = 7;
        borrow_global_mut<S>(@0x42).v = 8;
        borrow_global_mut<S>(@0x42).v = 9;
        exists<R<T>>(@0x42) && borrow_global<R<T>>(@0x42).value
    }
}
