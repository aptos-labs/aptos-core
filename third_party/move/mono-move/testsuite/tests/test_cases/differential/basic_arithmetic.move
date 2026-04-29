// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun add(a: u64, b: u64): u64 {
        a + b
    }
}

// RUN: execute 0x1::test::add --args 3, 5
// CHECK: results: 8

// RUN: execute 0x1::test::add --args 0, 0
// CHECK: results: 0
