// MonoMove's heap limits, which V1 does not have: a live set larger than the heap after
// a collection, a single request larger than the heap, and a single request larger than
// the 4 MiB allocation cap at the default heap size (growing a 262144-element u64 vector
// requests 4 MiB + 16 bytes). V1 runs all of them.

// RUN: publish
module 0x1::heap_limits {
    use std::vector;

    fun fill(count: u64): vector<u64> {
        let values = vector::empty<u64>();
        let index = 0;
        while (index < count) { vector::push_back(&mut values, index); index = index + 1; };
        values
    }

    public fun live_len(count: u64): u64 {
        let values = fill(count);
        vector::length(&values)
    }
}

// Growing from capacity 8 (80 bytes) to 16 (144 bytes) needs both live: 224 > 200.
// RUN: execute 0x1::heap_limits::live_len --args 16 --heap-size 200
// CHECK-V1: results: 16
// CHECK-V2: error: out of heap memory after GC (requested 144 bytes)
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::heap_limits::live_len --args 16 --heap-size 224
// CHECK: results: 16
// CHECK-GC-COUNT: 1

// A 272-byte request on a 256-byte heap.
// RUN: execute 0x1::heap_limits::live_len --args 17 --heap-size 256
// CHECK-V1: results: 17
// CHECK-V2: error: out of heap memory after GC (requested 272 bytes)

// RUN: execute 0x1::heap_limits::live_len --args 262144
// CHECK: results: 262144
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::heap_limits::live_len --args 262145
// CHECK-V1: results: 262145
// CHECK-V2: error: heap_alloc: size 4194320 exceeds maximum single allocation size
