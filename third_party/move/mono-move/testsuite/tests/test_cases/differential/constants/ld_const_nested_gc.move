// Loading the nested-vector constant NESTED (outer vector 48 bytes, inner 32, 40 and 24; the
// empty one allocates nothing) after the dead 96-byte FILLER. A 144-byte heap runs out at the
// first inner vector and a 216-byte heap at the last; each collects once and the value loads
// intact. At 240 bytes, or without FILLER, nothing collects.

// RUN: publish
module 0x42::ld_const_nested_gc {
    use std::vector;

    const FILLER: vector<u64> = vector[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    const NESTED: vector<vector<u64>> = vector[vector[1, 2], vector[3, 4, 5], vector[], vector[6]];

    fun fill_and_measure(): u64 {
        let filler = FILLER;
        vector::length(&filler)
    }

    fun digest(nested: &vector<vector<u64>>): u64 {
        let total = 0;
        let index = 0;
        while (index < vector::length(nested)) {
            let inner = vector::borrow(nested, index);
            let inner_index = 0;
            while (inner_index < vector::length(inner)) { total = total * 10 + *vector::borrow(inner, inner_index); inner_index = inner_index + 1; };
            total = total * 10;
            index = index + 1;
        };
        total
    }

    public fun load_nested(with_filler: bool): u64 {
        let filler_len = if (with_filler) fill_and_measure() else 0;
        let nested = NESTED;
        filler_len * 10000000000 + digest(&nested)
    }
}

// RUN: execute 0x42::ld_const_nested_gc::load_nested --args true --heap-size 144
// CHECK: results: 101203450060
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::ld_const_nested_gc::load_nested --args true --heap-size 216
// CHECK: results: 101203450060
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::ld_const_nested_gc::load_nested --args true --heap-size 240
// CHECK: results: 101203450060
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::ld_const_nested_gc::load_nested --args false --heap-size 144
// CHECK: results: 1203450060
// CHECK-GC-COUNT: 0
