// RUN: publish
module 0x66::vec_literals {
    use std::vector;

    public fun sum_u8(a: u8, b: u8, c: u8): u64 {
        let v = vector[a, b, c];
        let s = 0u64;
        let i = 0;
        while (i < vector::length(&v)) {
            s = s + (*vector::borrow(&v, i) as u64);
            i = i + 1;
        };
        s
    }

    public fun sum_u64(a: u64, b: u64): u64 {
        let v = vector[a, b];
        *vector::borrow(&v, 0) + *vector::borrow(&v, 1)
    }

    public fun last_u256(a: u256, b: u256, c: u256, d: u256): u256 {
        let v = vector[a, b, c, d];
        *vector::borrow(&v, 3)
    }

    public fun nested(a: u8, b: u8): u64 {
        let v = vector[vector[a], vector[b, b], vector[]];
        (vector::length(vector::borrow(&v, 0))
            + vector::length(vector::borrow(&v, 1))
            + vector::length(vector::borrow(&v, 2))) * 100
            + (*vector::borrow(vector::borrow(&v, 1), 1) as u64)
    }

    struct Pair has copy, drop {
        first: u64,
        second: u64,
    }

    public fun structs(x: u64, y: u64): u64 {
        let v = vector[Pair { first: x, second: y }, Pair { first: y, second: x }];
        let p = vector::borrow(&v, 1);
        p.first * 1000 + p.second
    }

    public fun single(x: u64): u64 {
        let v = vector[x];
        vector::pop_back(&mut v)
    }

    fun take(v: vector<u64>): u64 {
        *vector::borrow(&v, 0) * 10 + vector::length(&v)
    }

    public fun literal_as_arg(a: u64, b: u64): u64 {
        take(vector[a, b])
    }

    fun bump(x: u64): u64 {
        x + 1
    }

    public fun elements_from_calls(a: u64, b: u64): u64 {
        let v = vector[bump(a), bump(b)];
        *vector::borrow(&v, 0) * 100 + *vector::borrow(&v, 1)
    }
}

// RUN: execute 0x66::vec_literals::sum_u8 --args 1, 2, 3
// CHECK: results: 6

// RUN: execute 0x66::vec_literals::sum_u8 --args 255, 255, 255
// CHECK: results: 765

// RUN: execute 0x66::vec_literals::sum_u64 --args 40, 2
// CHECK: results: 42

// RUN: execute 0x66::vec_literals::sum_u64 --args 18446744073709551615, 0
// CHECK: results: 18446744073709551615

// RUN: execute 0x66::vec_literals::last_u256 --args 1, 2, 3, 115792089237316195423570985008687907853269984665640564039457584007913129639935
// CHECK: results: 115792089237316195423570985008687907853269984665640564039457584007913129639935

// RUN: execute 0x66::vec_literals::nested --args 7, 9
// CHECK: results: 309

// RUN: execute 0x66::vec_literals::structs --args 5, 6
// CHECK: results: 6005

// RUN: execute 0x66::vec_literals::single --args 99
// CHECK: results: 99

// RUN: execute 0x66::vec_literals::literal_as_arg --args 7, 3
// CHECK: results: 72

// RUN: execute 0x66::vec_literals::elements_from_calls --args 4, 8
// CHECK: results: 509
