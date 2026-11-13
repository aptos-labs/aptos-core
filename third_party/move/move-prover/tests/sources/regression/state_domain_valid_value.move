// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A value picked for a state quantifier satisfies its Move type.

module 0x42::state_domain_valid_value {

    spec fun increments_to(x: &mut u64, value: u64): bool {
        old(x) + 1 == value
    }

    /// Must fail: `s + 1 == 0` has no `u64` solution.
    public fun returns_zero(_x: &mut u64): u64 {
        0
    }
    spec returns_zero {
        ensures exists S in *: S.. |~ increments_to(_x, result);
    }

    /// Must verify: `s + 1 == 1` has the solution `s = 0`.
    public fun returns_one(_x: &mut u64): u64 {
        1
    }
    spec returns_one {
        ensures exists S in *: S.. |~ increments_to(_x, result);
    }
}
