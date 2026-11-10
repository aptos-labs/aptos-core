// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module fixture::base {
    public fun add(x: u64, y: u64): u64 {
        x + y
    }

    spec add {
        ensures result == x + y;
        aborts_if x + y > MAX_U64;
    }

    /// Unspecified, so a caller inlines it and owes its assertion.
    public fun positive(x: u64): u64 {
        spec {
            assert x > 0;
        };
        x
    }
}
