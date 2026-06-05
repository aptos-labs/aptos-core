// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    const NESTED_EMPTY: vector<vector<u8>> = vector[];
    const NESTED: vector<vector<u8>> = vector[vector[10], vector[20, 30, 40]];

    fun is_empty(): bool {
        std::vector::length(&NESTED_EMPTY) == 0
    }

    fun outer_len(): u64 {
        std::vector::length(&NESTED)
    }

    fun inner_0_len(): u64 {
        std::vector::length(std::vector::borrow(&NESTED, 0))
    }

    fun inner_1_len(): u64 {
        std::vector::length(std::vector::borrow(&NESTED, 1))
    }

    fun elem(a: u64, b: u64): u8 {
        *std::vector::borrow(std::vector::borrow(&NESTED, a), b)
    }
}

// RUN: execute 0x1::test::is_empty
// CHECK: results: true

// RUN: execute 0x1::test::outer_len
// CHECK: results: 2

// RUN: execute 0x1::test::inner_0_len
// CHECK: results: 1

// RUN: execute 0x1::test::inner_1_len
// CHECK: results: 3

// RUN: execute 0x1::test::elem --args 0, 0
// CHECK: results: 10

// RUN: execute 0x1::test::elem --args 1, 1
// CHECK: results: 30
