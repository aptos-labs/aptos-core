// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun clamp(n: u64): u64 {
        if (n <= 10) {
            n
        } else {
            10
        }
    }
}

// RUN: execute 0x1::test::clamp --args 0
// CHECK: results: 0

// RUN: execute 0x1::test::clamp --args 5
// CHECK: results: 5

// RUN: execute 0x1::test::clamp --args 10
// CHECK: results: 10

// RUN: execute 0x1::test::clamp --args 100
// CHECK: results: 10
