// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x99::capture_vector {
    use std::vector;

    fun len_plus(v: vector<u64>, extra: u64): u64 {
        vector::length(&v) + extra
    }

    fun len_sum(v1: vector<u64>, v2: vector<u64>, extra: u64): u64 {
        vector::length(&v1) + vector::length(&v2) + extra
    }

    fun capture_one(extra: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 10);
        vector::push_back(&mut v, 20);
        vector::push_back(&mut v, 30);
        let f: |u64|u64 has drop = |e| len_plus(v, e);
        f(extra)
    }

    fun capture_two(extra: u64): u64 {
        let v1 = vector::empty<u64>();
        vector::push_back(&mut v1, 1);
        vector::push_back(&mut v1, 2);
        let v2 = vector::empty<u64>();
        vector::push_back(&mut v2, 3);
        vector::push_back(&mut v2, 4);
        vector::push_back(&mut v2, 5);
        let f: |u64|u64 has drop = |e| len_sum(v1, v2, e);
        f(extra)
    }
}

// RUN: execute 0x99::capture_vector::capture_one --args 100
// CHECK: results: 103

// RUN: execute 0x99::capture_vector::capture_two --args 100
// CHECK: results: 105
