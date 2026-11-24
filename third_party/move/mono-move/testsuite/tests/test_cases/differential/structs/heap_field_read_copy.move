// Reading a vector field by value (through a local field chain, a reference chain, a
// single reference, and an enum variant) makes an independent copy: writing an element
// through one owner leaves the other unchanged. The golden pins how each read lowers.

// RUN: publish --print(stackless,micro-ops)
module 0x1::heap_field_read_copy {
    use std::vector;

    struct Inner has copy, drop { items: vector<u64> }
    struct Mid has copy, drop { inner: Inner }
    struct Outer has copy, drop { mid: Mid }

    enum H has copy, drop {
        One { v: vector<u64> },
        Two { v: vector<u64>, k: u64 },
    }

    fun outer(): Outer {
        Outer { mid: Mid { inner: Inner { items: vector[1, 2] } } }
    }

    // Each test writes 9 to copy[0] and 8 to original[1], then returns (original, copy).

    public fun local_chain(): (vector<u64>, vector<u64>) {
        let orig = outer();
        let copied = orig.mid.inner.items;
        *vector::borrow_mut(&mut copied, 0) = 9;
        *vector::borrow_mut(&mut orig.mid.inner.items, 1) = 8;
        (orig.mid.inner.items, copied)
    }

    fun via_ref(src: &Outer): vector<u64> { src.mid.inner.items }

    public fun ref_chain(): (vector<u64>, vector<u64>) {
        let orig = outer();
        let copied = via_ref(&orig);
        *vector::borrow_mut(&mut copied, 0) = 9;
        *vector::borrow_mut(&mut orig.mid.inner.items, 1) = 8;
        (orig.mid.inner.items, copied)
    }

    fun one(src: &Inner): vector<u64> { src.items }

    public fun ref_field(): (vector<u64>, vector<u64>) {
        let orig = Inner { items: vector[1, 2] };
        let copied = one(&orig);
        *vector::borrow_mut(&mut copied, 0) = 9;
        *vector::borrow_mut(&mut orig.items, 1) = 8;
        (orig.items, copied)
    }

    fun get(src: &H): vector<u64> { src.v }

    public fun variant_field(sel: u64): (vector<u64>, vector<u64>) {
        let orig = if (sel == 0) H::One { v: vector[1, 2] } else H::Two { v: vector[1, 2], k: 9 };
        let copied = get(&orig);
        *vector::borrow_mut(&mut copied, 0) = 9;
        *vector::borrow_mut(&mut orig.v, 1) = 8;
        (orig.v, copied)
    }
}

// RUN: execute 0x1::heap_field_read_copy::local_chain
// CHECK: results: [1, 8], [9, 2]

// RUN: execute 0x1::heap_field_read_copy::ref_chain
// CHECK: results: [1, 8], [9, 2]

// RUN: execute 0x1::heap_field_read_copy::ref_field
// CHECK: results: [1, 8], [9, 2]

// RUN: execute 0x1::heap_field_read_copy::variant_field --args 0
// CHECK: results: [1, 8], [9, 2]

// RUN: execute 0x1::heap_field_read_copy::variant_field --args 1
// CHECK: results: [1, 8], [9, 2]
