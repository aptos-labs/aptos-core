// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A function the export leaves out, as it constructs a function value, is
/// not verified, and verification says so.
module 0x42::closure_errors { // error: `add_one` is left out of the export
    fun add_one(x: u64): u64 {
        let f = |y: u64| y + 1;
        f(x)
    }
    spec add_one {
        ensures result == x + 1;
    }

    fun one(): u64 {
        1
    }
    spec one {
        ensures result == 1;
    }
}
