// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for the `unfold` and `unfold depth N` proof hints.
module 0x42::TestUnfold {

    // ==========================================================================
    // Basic unfold: non-recursive spec function.

    spec fun double(x: u64): u64 { x + x }

    fun test_unfold(x: u64): u64 {
        x + x
    }
    spec test_unfold {
        ensures result == double(x);

        proof {
            unfold double;
        }
    }

    // ==========================================================================
    // Recursive spec function for depth tests.

    spec fun sum(n: u64): u64 {
        if (n == 0) { 0 } else { n + sum(n - 1) }
    }

    // Depth 1 (= plain unfold): one level of expansion.
    fun test_depth_1(n: u64): u64 {
        if (n == 0) { 0 } else { n }
    }
    spec test_depth_1 {
        requires n <= 0;
        ensures result == sum(n);

        proof {
            unfold sum depth 1;
        }
    }

    // Depth 2: two levels of expansion.
    fun test_depth_2(n: u64): u64 {
        if (n == 0) { 0 } else if (n == 1) { 1 } else { n }
    }
    spec test_depth_2 {
        requires n <= 1;
        ensures result == sum(n);

        proof {
            unfold sum depth 2;
        }
    }

    // Depth 3: three levels of expansion.
    fun test_depth_3(n: u64): u64 {
        if (n == 0) { 0 }
        else if (n == 1) { 1 }
        else if (n == 2) { 3 }
        else { n }
    }
    spec test_depth_3 {
        requires n <= 2;
        ensures result == sum(n);

        proof {
            unfold sum depth 3;
        }
    }
}
