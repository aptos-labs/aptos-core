// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x99::capture_alignment {
    use std::vector;

    fun combine(flag: bool, v: vector<u64>, extra: u64): u64 {
        let base = vector::length(&v) + extra;
        if (flag) base + 1 else base
    }

    fun run(flag: bool): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 10);
        vector::push_back(&mut v, 20);
        let f: |u64|u64 has drop = |extra| combine(flag, v, extra);
        f(100)
    }
}

// RUN: execute 0x99::capture_alignment::run --args true
// CHECK: results: 103

// RUN: execute 0x99::capture_alignment::run --args false
// CHECK: results: 102
