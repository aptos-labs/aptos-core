// RUN: publish
module 0x42::enums_field_by_ref {
    enum Boxed has drop {
        One { value: u64 },
        Two { value: u64, other: u64 },
    }

    // Read a common field through an immutable reference.
    fun read_field(v: u64): u64 {
        let b = Boxed::One { value: v };
        let r = &b.value;
        *r
    }

    // Mutate a field through a mutable reference, then read it back.
    fun write_field(v: u64, delta: u64): u64 {
        let b = Boxed::One { value: v };
        let r = &mut b.value;
        *r = *r + delta;
        *(&b.value)
    }

    // Borrow a variant-specific field (only present in `Two`).
    fun read_other(v: u64, o: u64): u64 {
        let b = Boxed::Two { value: v, other: o };
        *(&b.other)
    }
}

// RUN: execute 0x42::enums_field_by_ref::read_field --args 42
// CHECK: results: 42

// RUN: execute 0x42::enums_field_by_ref::write_field --args 10, 5
// CHECK: results: 15

// RUN: execute 0x42::enums_field_by_ref::read_other --args 1, 99
// CHECK: results: 99
