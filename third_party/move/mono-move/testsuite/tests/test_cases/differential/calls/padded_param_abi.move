// RUN: publish --print(bytecode,stackless,micro-ops,frame-layout)
module 0x66::padded_param {
    use std::vector;

    fun keep_u64(flag: bool, x: u64): u64 {
        // Heap-pointer local ⇒ `zero_frame` is set.
        let v = vector::empty<u64>();
        vector::push_back(&mut v, x);
        if (flag) { x } else { vector::pop_back(&mut v) }
    }

    fun driver(flag: bool, x: u64): u64 {
        keep_u64(flag, x)
    }
}

// RUN: execute 0x66::padded_param::keep_u64 --args true, 18446744073709551615
// CHECK: results: 18446744073709551615
// RUN: execute 0x66::padded_param::keep_u64 --args false, 18446744073709551615
// CHECK: results: 18446744073709551615
// RUN: execute 0x66::padded_param::keep_u64 --args true, 12345678901234567890
// CHECK: results: 12345678901234567890

// RUN: execute 0x66::padded_param::driver --args true, 18446744073709551615
// CHECK: results: 18446744073709551615
// RUN: execute 0x66::padded_param::driver --args false, 12345678901234567890
// CHECK: results: 12345678901234567890
