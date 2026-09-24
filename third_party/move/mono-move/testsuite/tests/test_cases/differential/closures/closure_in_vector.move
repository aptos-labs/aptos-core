// Function values are stored in vectors, borrowed, and called. Capturing a
// vector in a closure adds another level of heap indirection.

// RUN: publish
module 0x1::test {
    use std::vector;

    public fun add1(x: u64): u64 { x + 1 }
    public fun double(x: u64): u64 { x * 2 }
    public fun add100(x: u64): u64 { x + 100 }

    // Non-capturing function values in a vector.
    fun call_from_vec(i: u64): u64 {
        let v: vector<|u64|u64 has copy+drop+store> = vector[add1, double, add100];
        let f = vector::borrow(&v, i);
        (*f)(10)
    }

    public fun add_k(k: u64, x: u64): u64 { x + k }
    public fun mul_k(k: u64, x: u64): u64 { x * k }

    // Capturing closures in a vector.
    fun capturing_from_vec(k: u64): u64 {
        let v: vector<|u64|u64 has copy+drop+store> =
            vector[|x| add_k(k, x), |x| mul_k(k, x)];
        let f = vector::borrow(&v, 0);
        let g = vector::borrow(&v, 1);
        (*f)(1) * 100 + (*g)(1)
    }

    public fun len_plus(v: vector<u64>, x: u64): u64 { x + vector::length(&v) }

    // The vector element is a closure that captures another vector.
    fun closure_owning_vec(): u64 {
        let payload = vector[3u64, 4, 5];
        let v: vector<|u64|u64 has copy+drop+store> = vector[|x| len_plus(payload, x)];
        let f = vector::borrow(&v, 0);
        (*f)(10)
    }
}

// RUN: execute 0x1::test::call_from_vec --args 0
// CHECK: results: 11

// RUN: execute 0x1::test::call_from_vec --args 1
// CHECK: results: 20

// RUN: execute 0x1::test::call_from_vec --args 2
// CHECK: results: 110

// RUN: execute 0x1::test::capturing_from_vec --args 10
// CHECK: results: 1110

// RUN: execute 0x1::test::closure_owning_vec
// CHECK: results: 13
