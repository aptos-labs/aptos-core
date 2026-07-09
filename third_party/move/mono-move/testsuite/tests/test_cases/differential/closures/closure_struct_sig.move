// RUN: publish
module 0x43::closure_struct_sig {
    use std::vector;

    struct S has copy, drop {
        v: u64,
    }

    struct P has drop {
        xs: vector<u64>,
    }

    fun make_s(v: u64): S {
        S { v }
    }

    fun read_s(s: S): u64 {
        let S { v } = s;
        v
    }

    // `S` appears in the closure signatures.
    fun chain_s(v: u64): u64 {
        let producer: || S has drop = || make_s(v);
        let consumer: |S| u64 has drop = |s| read_s(s);
        consumer(producer())
    }

    fun make_p(a: u64, b: u64): P {
        let xs = vector::empty<u64>();
        vector::push_back(&mut xs, a);
        vector::push_back(&mut xs, b);
        P { xs }
    }

    fun read_p(p: P): u64 {
        let P { xs } = p;
        *vector::borrow(&xs, 0) + *vector::borrow(&xs, 1)
    }

    fun chain_p(a: u64, b: u64): u64 {
        let producer: || P has drop = || make_p(a, b);
        let consumer: |P| u64 has drop = |p| read_p(p);
        consumer(producer())
    }
}

// RUN: execute 0x43::closure_struct_sig::chain_s --args 7
// CHECK: results: 7

// RUN: execute 0x43::closure_struct_sig::chain_p --args 3, 5
// CHECK: results: 8
