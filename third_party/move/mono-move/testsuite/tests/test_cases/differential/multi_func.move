// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun double(x: u64): u64 {
        x + x
    }

    fun quad(x: u64): u64 {
        double(double(x))
    }
}

// RUN: execute 0x1::test::quad --args 0
// CHECK: results: 0

// RUN: execute 0x1::test::quad --args 1
// CHECK: results: 4

// RUN: execute 0x1::test::quad --args 3
// CHECK: results: 12

// RUN: execute 0x1::test::quad --args 10
// CHECK: results: 40
