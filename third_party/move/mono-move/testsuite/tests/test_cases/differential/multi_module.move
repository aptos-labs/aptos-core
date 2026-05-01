// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::math {
    public fun double(x: u64): u64 {
        x + x
    }
}

module 0x1::utils {
    public fun triple(x: u64): u64 {
        x + x + x
    }
}

// RUN: execute 0x1::math::double --args 7
// CHECK: results: 14

// RUN: execute 0x1::utils::triple --args 5
// CHECK: results: 15

// RUN: execute 0x1::math::double --args 0
// CHECK: results: 0

// RUN: execute 0x1::utils::triple --args 3
// CHECK: results: 9
