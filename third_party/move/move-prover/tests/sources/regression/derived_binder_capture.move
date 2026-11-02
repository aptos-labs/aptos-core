// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Binders the translator derives from Move names (the pre-state of a `&mut` parameter, the
// value of a state quantifier) are not captured by user binders spelled the same way.

module 0x42::derived_binder_capture {

    struct Counter has key, drop { value: u64 }

    spec fun same_as_old(c: &mut Counter): bool {
        let old_c = c;
        old(c).value == old_c.value
    }

    spec fun same_as_old_renamed(c: &mut Counter): bool {
        let oc = c;
        old(c).value == oc.value
    }

    /// Must fail: `increment` changes `value`.
    public fun increment(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec increment {
        aborts_if c.value + 1 > MAX_U64;
        ensures same_as_old(c);
    }

    /// Must fail: the same claim with the `let` renamed.
    public fun increment_renamed(c: &mut Counter) {
        c.value = c.value + 1;
    }
    spec increment_renamed {
        aborts_if c.value + 1 > MAX_U64;
        ensures same_as_old_renamed(c);
    }
}
