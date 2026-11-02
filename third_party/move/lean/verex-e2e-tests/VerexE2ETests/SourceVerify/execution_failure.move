// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::execution_failure {
    fun overflow(x: u64): u64 { x + 1 }
    spec overflow { aborts_if x == MAX_U64 with EXECUTION_FAILURE; }

    fun wrong_overflow(x: u64): u64 { x + 1 }
    spec wrong_overflow { aborts_if x == MAX_U64 with MAX_U64 + 1; }

    fun divide(x: u64): u64 { 10 / x }
    spec divide { aborts_if x == 0 with EXECUTION_FAILURE; }

    fun wrong_divide(x: u64): u64 { 10 / x }
    spec wrong_divide { aborts_if x == 0 with 0; }

    fun narrow(x: u64): u8 { x as u8 }
    spec narrow { aborts_if x > MAX_U8 with EXECUTION_FAILURE; }

    fun shift(x: u64, distance: u8): u64 { x >> distance }
    spec shift { aborts_if distance >= 64 with EXECUTION_FAILURE; }

    fun explicit_zero() { abort 0 }
    spec explicit_zero { aborts_with 0; }

    fun wrong_explicit_zero() { abort 0 }
    spec wrong_explicit_zero { aborts_with EXECUTION_FAILURE; }
}
