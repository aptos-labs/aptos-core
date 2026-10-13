// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A higher-order function specified by behavioral predicates, and callers
/// passing lambdas with their own specifications.
module 0x42::behavior {
    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque = true;
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    fun increment(x: u64): u64 {
        apply(|y| y + 5 spec {
            aborts_if y + 5 > MAX_U64;
            ensures result == y + 5;
        }, x)
    }
    spec increment {
        aborts_if x + 5 > MAX_U64;
        ensures result == x + 5;
    }

    fun checked(x: u64): u64 {
        apply(|y| if (y == 0) abort 1 else y spec {
            aborts_if y == 0;
            ensures result == y;
        }, x)
    }
    spec checked {
        aborts_if x == 0;
        ensures result == x;
    }
}
