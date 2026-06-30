// RUN: publish --print(bytecode,stackless,micro-ops,frame-layout)
module 0x66::vec_pushback_safe_point {
    use std::vector;

    fun fresh(): vector<u8> {
        vector::empty<u8>()
    }

    fun take(_v: vector<u8>) {}

    public fun caller(val: u64): u64 {
        let saved = fresh();
        let v = vector::empty<u64>();
        vector::push_back(&mut v, val);
        take(saved);
        vector::pop_back(&mut v)
    }
}

// RUN: execute 0x66::vec_pushback_safe_point::caller --args 42
// CHECK: results: 42

// RUN: execute 0x66::vec_pushback_safe_point::caller --args 0
// CHECK: results: 0

// RUN: execute 0x66::vec_pushback_safe_point::caller --args 18446744073709551615
// CHECK: results: 18446744073709551615
