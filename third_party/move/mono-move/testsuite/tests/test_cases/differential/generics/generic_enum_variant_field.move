// RUN: publish --print(micro-ops)
module 0x42::generic_enum_variant_field {
    // `v` sits at the same offset in both variants for any T (uniform fast
    // path), but the offset itself depends on the instantiation.
    enum Uniform<T> has drop {
        M { a: T, v: u64 },
        N { b: T, v: u64 },
    }

    // `v` is at offset 0 in Tail but after `a` in Lead: divergent offsets,
    // tag-dispatched access.
    enum Divergent<T> has drop {
        Lead { a: T, v: u64 },
        Tail { v: u64 },
    }

    fun uniform_u8(sel: u64, v: u64): u64 {
        let e: Uniform<u8> = if (sel == 0) {
            Uniform::M { a: 1, v }
        } else {
            Uniform::N { b: 2, v: v + 1 }
        };
        e.v
    }

    fun uniform_u128(sel: u64, v: u64): u64 {
        let e: Uniform<u128> = if (sel == 0) {
            Uniform::M { a: 1, v }
        } else {
            Uniform::N { b: 2, v: v + 1 }
        };
        e.v
    }

    fun divergent_u8(sel: u64, v: u64): u64 {
        let e: Divergent<u8> = if (sel == 0) {
            Divergent::Lead { a: 9, v }
        } else {
            Divergent::Tail { v: v + 100 }
        };
        e.v
    }

    fun divergent_u128(sel: u64, v: u64): u64 {
        let e: Divergent<u128> = if (sel == 0) {
            Divergent::Lead { a: 9, v }
        } else {
            Divergent::Tail { v: v + 100 }
        };
        e.v
    }

    // Write to a variant field through &mut.
    fun write_uniform(v: u64): u64 {
        let e = Uniform::M<u64> { a: 5, v };
        let r = &mut e;
        r.v = r.v + 7;
        e.v
    }
}

// RUN: execute 0x42::generic_enum_variant_field::uniform_u8 --args 0, 11
// CHECK: results: 11

// RUN: execute 0x42::generic_enum_variant_field::uniform_u8 --args 1, 11
// CHECK: results: 12

// RUN: execute 0x42::generic_enum_variant_field::uniform_u128 --args 0, 22
// CHECK: results: 22

// RUN: execute 0x42::generic_enum_variant_field::uniform_u128 --args 1, 22
// CHECK: results: 23

// RUN: execute 0x42::generic_enum_variant_field::divergent_u8 --args 0, 33
// CHECK: results: 33

// RUN: execute 0x42::generic_enum_variant_field::divergent_u8 --args 1, 33
// CHECK: results: 133

// RUN: execute 0x42::generic_enum_variant_field::divergent_u128 --args 0, 44
// CHECK: results: 44

// RUN: execute 0x42::generic_enum_variant_field::divergent_u128 --args 1, 44
// CHECK: results: 144

// RUN: execute 0x42::generic_enum_variant_field::write_uniform --args 100
// CHECK: results: 107
