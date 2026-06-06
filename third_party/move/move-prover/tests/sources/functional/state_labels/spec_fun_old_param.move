// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that spec functions can use `old(p)` where `p` is a `&mut T` parameter,
// not just `old(global<R>(addr))`. The spec_fun declaration auto-emits dual
// `old_{p}: T, {p}: T` parameters for each `&mut` parameter (see
// spec_translator.rs:627). The call site must mirror this by emitting both the
// pre-state and post-state expressions for each `&mut` argument.

module 0x42::param_old_repro {
    struct Counter has copy, drop, store { value: u64 }

    // =========================================================================
    // Single &mut parameter
    // =========================================================================

    /// Returns true when the counter strictly increased.
    spec fun counter_increased(c: &mut Counter): bool {
        old(c).value < c.value
    }

    fun increment(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec increment {
        ensures counter_increased(c); // should verify
    }

    fun noop(_c: &mut Counter) {}
    spec noop {
        ensures counter_increased(_c); // should fail: counter did not change
    }

    // =========================================================================
    // Two &mut parameters
    // =========================================================================

    spec fun both_increased(a: &mut Counter, b: &mut Counter): bool {
        old(a).value < a.value && old(b).value < b.value
    }

    fun increment_both(a: &mut Counter, b: &mut Counter) {
        a.value = a.value + 1;
        b.value = b.value + 1;
    }
    spec increment_both {
        ensures both_increased(a, b); // should verify
    }
}
