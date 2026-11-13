// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A specification function's tuple result holds logical integers, a type
// argument's at its own type; a call reads it at the call's type.
module 0x42::spec_tuple_results {
    spec fun mixed<T>(x: T, y: u64): (T, u64) { (x, y) }

    spec fun counted<T>(x: T, n: u64): (T, u64) {
        if (n == 0) (x, 0) else {
            let (_, c) = counted<T>(x, n - 1);
            (x, c + 1)
        }
    }

    fun at_address(): bool { true }
    spec at_address {
        ensures mixed<address>(@0x1, 42) == (@0x1, 42);
    }

    fun at_integer(): bool { true }
    spec at_integer {
        ensures mixed<u64>(1, 42) == (1, 42);
    }

    fun recursive_at_integer(): bool { true }
    spec recursive_at_integer {
        ensures counted<u64>(7, 0) == (7, 0);
    }

    fun recursive_at_address(): bool { true }
    spec recursive_at_address {
        ensures counted<address>(@0x1, 0) == (@0x1, 0);
    }

    fun wrong(): bool { true }
    spec wrong {
        ensures mixed<u64>(1, 42) == (1, 43); // error: another component
    }
}
