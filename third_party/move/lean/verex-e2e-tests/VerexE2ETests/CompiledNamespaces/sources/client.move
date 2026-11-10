// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module fixture::client {
    use fixture::base;

    public fun twice(x: u64): u64 {
        base::add(x, x)
    }

    spec twice {
        ensures result == 2 * x;
        aborts_if 2 * x > MAX_U64;
    }

    /// Fails at the assertion in `base`, reported in `base.move`.
    public fun unchecked(x: u64): u64 {
        base::positive(x)
    }

    spec unchecked {
        ensures result == x;
    }
}
