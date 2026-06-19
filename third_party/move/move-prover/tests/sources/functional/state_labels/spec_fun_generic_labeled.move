// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Generic spec functions under a labeled state range need the witness type
// to come from the call-site instantiation, not the declaration's type
// parameters. Without that, the bound Boogie variable's type doesn't match
// the monomorphized call.

module 0x42::spec_fun_generic_labeled {
    struct Counter has copy, drop, store { value: u64 }

    spec fun changed<T: copy>(c: &mut T): bool {
        old(c) != c
    }

    fun inc(c: &mut Counter) {
        c.value = c.value + 1;
    }

    fun inc_twice(c: &mut Counter) {
        inc(c);
        inc(c);
    }
    spec inc_twice {
        ensures exists S in *:
            (..S |~ changed<Counter>(c)) &&
            (S.. |~ changed<Counter>(c));
    }
}
