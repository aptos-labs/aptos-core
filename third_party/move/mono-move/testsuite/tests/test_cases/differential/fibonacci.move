// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// RUN: publish
module 0x1::test {
    fun fib(n: u64): u64 {
        if (n <= 1) { n } else { fib(n - 1) + fib(n - 2) }
    }
}

// RUN: execute 0x1::test::fib --args 0
// CHECK: results: 0

// RUN: execute 0x1::test::fib --args 1
// CHECK: results: 1

// RUN: execute 0x1::test::fib --args 5
// CHECK: results: 5

// RUN: execute 0x1::test::fib --args 10
// CHECK: results: 55
