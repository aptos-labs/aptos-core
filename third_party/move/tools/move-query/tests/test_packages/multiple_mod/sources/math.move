// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Math utilities module.
module test_addr::math {
    /// Adds two numbers.
    public fun add(a: u64, b: u64): u64 {
        a + b
    }

    /// Multiplies two numbers.
    public fun multiply(a: u64, b: u64): u64 {
        a * b
    }

    /// Returns the larger of two numbers.
    public fun max(a: u64, b: u64): u64 {
        if (a > b) { a } else { b }
    }

    /// Returns the smaller of two numbers.
    public fun min(a: u64, b: u64): u64 {
        if (a < b) { a } else { b }
    }
}
