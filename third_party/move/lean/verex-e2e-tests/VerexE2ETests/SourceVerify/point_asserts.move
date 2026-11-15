// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// An `assert` of a proof's point step is an obligation of its own, keyed by
/// the point, as an `assert` of a Move proof block is: the same assert written
/// in either host owes and gives the same fact.
module 0x42::point_asserts {
    // An entry assert in the Move proof block.
    fun double(x: u64): u64 {
        x + x
    }
    spec double {
        requires x + x <= MAX_U64;
        ensures result == 2 * x;
    } proof {
        assert x + x == 2 * x;
    }

    // The same assert as a point step of the proof file.
    fun double_steps(x: u64): u64 {
        x + x
    }
    spec double_steps {
        requires x + x <= MAX_U64;
        ensures result == 2 * x;
    }

    fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        pragma opaque;
        requires x < MAX_U64;
        ensures result == x + 1;
    }

    // An assert after a call, with a closer.
    fun twice(x: u64): u64 {
        let y = inc(x);
        inc(y)
    }
    spec twice {
        requires x + 2 <= MAX_U64;
        ensures result == x + 2;
    }

    // A point assert that does not hold.
    fun successor(x: u64): u64 {
        x + 1
    }
    spec successor {
        requires x < MAX_U64;
        ensures result == x + 1;
    }
}
