// RUN: publish
module 0x1::test_natives {
    native public fun split_bytes(bytes: vector<u8>, chunk: u64): vector<vector<u8>>;
}
module 0x1::main {
    use std::vector;
    use 0x1::test_natives::split_bytes;

    const DATA: vector<u8> = b"abcdefghij";

    // Element lengths, so a ragged final chunk is visible.
    fun lengths(chunks: &vector<vector<u8>>): vector<u64> {
        let out = vector[];
        let i = 0;
        while (i < vector::length(chunks)) {
            vector::push_back(&mut out, vector::length(vector::borrow(chunks, i)));
            i = i + 1;
        };
        out
    }

    public fun split_even(): (u64, vector<u64>) {
        let chunks = split_bytes(DATA, 5);
        (vector::length(&chunks), lengths(&chunks))
    }

    // 10 bytes in chunks of 3: the last chunk holds the single leftover byte.
    public fun split_ragged(): (u64, vector<u64>) {
        let chunks = split_bytes(DATA, 3);
        (vector::length(&chunks), lengths(&chunks))
    }

    public fun split_ragged_last(): vector<u8> {
        let chunks = split_bytes(DATA, 3);
        *vector::borrow(&chunks, 3)
    }

    // One chunk per byte: the widest outer vector this native builds.
    public fun split_singletons(): (u64, vector<u8>, vector<u8>) {
        let chunks = split_bytes(DATA, 1);
        (
            vector::length(&chunks),
            *vector::borrow(&chunks, 0),
            *vector::borrow(&chunks, 9),
        )
    }

    // A chunk wider than the input yields a single full-width element.
    public fun split_oversized(): (u64, vector<u8>) {
        let chunks = split_bytes(DATA, 100);
        (vector::length(&chunks), *vector::borrow(&chunks, 0))
    }

    public fun split_empty_input(): u64 {
        vector::length(&split_bytes(vector[], 4))
    }

    public fun split_zero_chunk(): u64 {
        vector::length(&split_bytes(DATA, 0))
    }

    // The result is an ordinary vector: pushing onto it must keep the element
    // stride the native's descriptor declared.
    public fun split_then_push(): (u64, vector<u8>) {
        let chunks = split_bytes(DATA, 4);
        vector::push_back(&mut chunks, b"tail");
        (vector::length(&chunks), *vector::borrow(&chunks, 3))
    }

    // Allocate after the native returns, so the collector relocates the outer
    // vector and each element, then read every element back.
    public fun split_survives_gc(rounds: u64): (u64, vector<u8>, vector<u8>) {
        let chunks = split_bytes(DATA, 3);
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + vector::length(&junk);
        };
        (
            vector::length(&chunks),
            *vector::borrow(&chunks, 0),
            *vector::borrow(&chunks, 3),
        )
    }
}

// RUN: execute 0x1::main::split_even
// CHECK: results: 2, [5, 5]

// RUN: execute 0x1::main::split_ragged
// CHECK: results: 4, [3, 3, 3, 1]

// RUN: execute 0x1::main::split_ragged_last
// CHECK: results: 0x6a

// RUN: execute 0x1::main::split_singletons
// CHECK: results: 10, 0x61, 0x6a

// RUN: execute 0x1::main::split_oversized
// CHECK: results: 1, 0x6162636465666768696a

// RUN: execute 0x1::main::split_empty_input
// CHECK: results: 0

// RUN: execute 0x1::main::split_zero_chunk
// CHECK: results: 0

// RUN: execute 0x1::main::split_then_push
// CHECK: results: 4, 0x7461696c

// RUN: execute 0x1::main::split_survives_gc --args 0 --heap-size 1024
// CHECK: results: 4, 0x616263, 0x6a
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::split_survives_gc --args 4000 --heap-size 1024
// CHECK: results: 4, 0x616263, 0x6a
// CHECK-GC-COUNT: 58
