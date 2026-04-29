// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun add_three(a: u64, b: u64, c: u64): u64 {
        a + b + c
    }
}

// RUN: execute 0x1::test::add_three --args 1, 2, 3
// CHECK: results: 6

// RUN: execute 0x1::test::add_three --args 0, 0, 0
// CHECK: results: 0

// RUN: execute 0x1::test::add_three --args 10, 20, 30
// CHECK: results: 60
