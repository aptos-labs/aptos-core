// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An included schema compares at the type it declared (`num`) values the
// inclusion gives another type (`u64`); the comparison is at the operands'.
// An inclusion may also leave operands of two other types (`num` and a
// defaulted literal), neither the declared one (`u64`).
module 0x42::schema_equality {
    fun add_as_spec_fun(x: u64, y: u64): u64 { x + y }

    fun add_fun(x: u64, y: u64): u64 { x + y }
    spec add_fun {
        include AddOk;
    }

    spec schema AddOk {
        x: num;
        y: num;
        result: num;
        ensures result == add_as_spec_fun(x, y);
        ensures result != add_as_spec_fun(x, y) + 1;
    }

    fun first(x: u64, _y: u64): u64 { x }
    spec first {
        let total = x + _y;
        include NonZero { amount: total };
    }

    spec schema NonZero {
        amount: u64;
        ensures amount != 0 ==> amount >= 1;
    }
}
