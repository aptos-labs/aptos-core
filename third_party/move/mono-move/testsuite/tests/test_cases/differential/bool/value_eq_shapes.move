// Equality on enums (same and different variants, differing fields, vector fields), Option,
// structs with mixed-width fields read out of a vector against ones built locally, vectors
// of such structs, nested vectors, and empty vectors never pushed to against ones emptied by
// pop_back. Both == and != have true and false cases.

// RUN: publish
module 0x66::eq_shapes {
    use std::vector;
    use std::option;

    enum Shape has copy, drop { Circle { r: u64 }, Rect { w: u64, h: u64 }, Dot }

    enum Tagged has copy, drop { Bytes { b: vector<u8> }, Num { n: u8 } }

    struct Flat has copy, drop { a: u8, b: u64 }

    struct Mixed has copy, drop { a: u8, b: u32, c: bool, d: u128 }

    fun shape(tag: u64, first: u64, second: u64): Shape {
        if (tag == 0) Shape::Circle { r: first } else if (tag == 1) Shape::Rect { w: first, h: second } else Shape::Dot
    }

    public fun enum_eq(t1: u64, x1: u64, y1: u64, t2: u64, x2: u64, y2: u64): bool {
        shape(t1, x1, y1) == shape(t2, x2, y2)
    }

    public fun enum_neq(t1: u64, x1: u64, t2: u64, x2: u64): bool {
        shape(t1, x1, 0) != shape(t2, x2, 0)
    }

    public fun enum_vec_field_eq(left: u8, right: u8): bool {
        Tagged::Bytes { b: vector[left] } == Tagged::Bytes { b: vector[right] }
    }

    public fun enum_tag_vs_payload(byte: u8): bool {
        Tagged::Num { n: byte } == Tagged::Bytes { b: vector[byte] }
    }

    public fun option_eq(left: u64, right: u64): bool { option::some(left) == option::some(right) }

    public fun option_none_some(value: u64): bool { option::none<u64>() == option::some(value) }

    public fun flat_eq(a1: u8, b1: u64, a2: u8, b2: u64): bool {
        let stored = vector[Flat { a: a1, b: b1 }];
        let heap = *vector::borrow(&stored, 0);
        let frame = Flat { a: a2, b: b2 };
        heap == frame
    }

    public fun flat_vec_eq(field_a: u8, field_b: u64): bool {
        let pushed = vector::empty<Flat>();
        vector::push_back(&mut pushed, Flat { a: field_a, b: field_b });
        vector::push_back(&mut pushed, Flat { a: field_a + 1, b: field_b });
        pushed == vector[Flat { a: field_a, b: field_b }, Flat { a: field_a + 1, b: field_b }]
    }

    public fun mixed_eq(field_a: u8, field_b: u32, field_d: u128): bool {
        let stored = vector[Mixed { a: field_a, b: field_b, c: true, d: field_d }];
        *vector::borrow(&stored, 0) == Mixed { a: field_a, b: field_b, c: true, d: field_d }
    }

    public fun mixed_neq_bool(field_a: u8): bool {
        Mixed { a: field_a, b: 1, c: true, d: 2 } == Mixed { a: field_a, b: 1, c: false, d: 2 }
    }

    public fun empty_vs_drained(value: u64): bool {
        let drained = vector[value];
        vector::pop_back(&mut drained);
        vector::empty<u64>() == drained
    }

    public fun nested_empty_vs_drained(value: u64): bool {
        let drained = vector[value];
        vector::pop_back(&mut drained);
        vector[vector::empty<u64>()] == vector[drained]
    }

    public fun nested_vec_eq(left: u64, right: u64): bool {
        vector[vector[left], vector[]] == vector[vector[right], vector[]]
    }
}

// Same tag, same fields.
// RUN: execute 0x66::eq_shapes::enum_eq --args 1, 2, 3, 1, 2, 3
// CHECK: results: true

// Same tag, one field differs.
// RUN: execute 0x66::eq_shapes::enum_eq --args 1, 2, 3, 1, 2, 4
// CHECK: results: false

// Different tags.
// RUN: execute 0x66::eq_shapes::enum_eq --args 0, 2, 3, 1, 2, 3
// CHECK: results: false

// Fieldless variant.
// RUN: execute 0x66::eq_shapes::enum_eq --args 2, 5, 6, 2, 7, 8
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::enum_eq --args 0, 5, 6, 0, 5, 8
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::enum_neq --args 0, 5, 0, 6
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::enum_neq --args 0, 5, 2, 5
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::enum_neq --args 0, 5, 0, 5
// CHECK: results: false

// Vector payloads.
// RUN: execute 0x66::eq_shapes::enum_vec_field_eq --args 1, 1
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::enum_vec_field_eq --args 1, 2
// CHECK: results: false

// RUN: execute 0x66::eq_shapes::enum_tag_vs_payload --args 3
// CHECK: results: false

// RUN: execute 0x66::eq_shapes::option_eq --args 4, 4
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::option_eq --args 4, 5
// CHECK: results: false

// RUN: execute 0x66::eq_shapes::option_none_some --args 0
// CHECK: results: false

// Mixed-width structs.
// RUN: execute 0x66::eq_shapes::flat_eq --args 200, 7, 200, 7
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::flat_eq --args 200, 7, 201, 7
// CHECK: results: false

// RUN: execute 0x66::eq_shapes::flat_eq --args 200, 7, 200, 8
// CHECK: results: false


// RUN: execute 0x66::eq_shapes::flat_vec_eq --args 3, 9
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::mixed_eq --args 1, 2, 3
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::mixed_neq_bool --args 1
// CHECK: results: false

// A vector never pushed to against one emptied by pop_back.
// RUN: execute 0x66::eq_shapes::empty_vs_drained --args 5
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::nested_empty_vs_drained --args 5
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::nested_vec_eq --args 1, 1
// CHECK: results: true

// RUN: execute 0x66::eq_shapes::nested_vec_eq --args 1, 2
// CHECK: results: false
