// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Error tests for `split` proof statement.
module 0x42::TestSplitErrors {

    // ==========================================================================
    // Error: split on a non-bool, non-enum type.

    fun test_split_invalid(x: u64): u64 { x }
    spec test_split_invalid {
        ensures result == x;
    } proof {
        split x;
    }
}
