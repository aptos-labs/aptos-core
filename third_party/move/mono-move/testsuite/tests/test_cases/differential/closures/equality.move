// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}

module 0x99::closure_eq {
    use std::bcs;

    struct Point has copy, drop, store { x: u64, y: u8 }

    struct Wrapper has copy, drop { f: |u64|u64 has copy + drop }

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun mul(a: u64, b: u64): u64 {
        a * b
    }

    #[persistent]
    fun sub(a: u64, b: u64): u64 {
        a - b
    }

    #[persistent]
    fun apply(f: |u64|u64 has copy + drop + store, x: u64): u64 {
        f(x)
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }

    #[persistent]
    fun norm(p: Point, k: u64): u64 {
        p.x + (p.y as u64) + k
    }

    #[persistent]
    fun pick<T: drop>(a: T, b: u64): u64 {
        let _ = a;
        b
    }

    fun wire(f: &|u64|u64 has copy + drop + store): |u64|u64 has copy + drop + store {
        0x1::from_bcs::from_bytes<|u64|u64 has copy + drop + store>(bcs::to_bytes(f))
    }

    public fun same_target(): bool {
        let f: |u64, u64|u64 has copy + drop = add;
        let g: |u64, u64|u64 has copy + drop = add;
        f == g
    }

    public fun different_target(): bool {
        let f: |u64, u64|u64 has copy + drop = add;
        let g: |u64, u64|u64 has copy + drop = mul;
        f == g
    }

    public fun different_type_arg(): bool {
        let a = true;
        let b = 1u8;
        let f: |u64|u64 has copy + drop = |k| pick<bool>(a, k);
        let g: |u64|u64 has copy + drop = |k| pick<u8>(b, k);
        f == g
    }

    public fun same_capture(): bool {
        let f: |u64|u64 has copy + drop = |b| add(7, b);
        let g: |u64|u64 has copy + drop = |b| add(7, b);
        f == g
    }

    public fun different_capture(): bool {
        let f: |u64|u64 has copy + drop = |b| add(7, b);
        let g: |u64|u64 has copy + drop = |b| add(8, b);
        f == g
    }

    public fun vector_capture(): bool {
        let d1 = vector[1u8, 2u8];
        let d2 = vector[1u8, 2u8];
        let d3 = vector[2u8, 1u8];
        let f: |u64|u64 has copy + drop = |k| tally(d1, k);
        let g: |u64|u64 has copy + drop = |k| tally(d2, k);
        let h: |u64|u64 has copy + drop = |k| tally(d3, k);
        f == g && f != h
    }

    // The shorter vector is a prefix of the longer one, so only length
    // separates them.
    public fun vector_capture_prefix(): bool {
        let d1 = vector[1u8];
        let d2 = vector[1u8, 1u8];
        let f: |u64|u64 has copy + drop = |k| tally(d1, k);
        let g: |u64|u64 has copy + drop = |k| tally(d2, k);
        f == g
    }

    public fun struct_capture(): bool {
        let p1 = Point { x: 5, y: 7 };
        let p2 = Point { x: 5, y: 7 };
        let p3 = Point { x: 7, y: 5 };
        let f: |u64|u64 has copy + drop = |k| norm(p1, k);
        let g: |u64|u64 has copy + drop = |k| norm(p2, k);
        let h: |u64|u64 has copy + drop = |k| norm(p3, k);
        f == g && f != h
    }

    public fun wire_equals_packed(): bool {
        let f: |u64|u64 has copy + drop + store = |b| add(7, b);
        f == wire(&f)
    }

    public fun wire_differs_from_packed(): bool {
        let f: |u64|u64 has copy + drop + store = |b| add(7, b);
        let g: |u64|u64 has copy + drop + store = |b| add(8, b);
        f == wire(&g)
    }

    public fun wire_equals_wire(): bool {
        let data = vector[1u8, 2u8];
        let f: |u64|u64 has copy + drop + store = |k| tally(data, k);
        wire(&f) == wire(&f)
    }

    public fun wire_noncapturing(): bool {
        let f: |u64, u64|u64 has copy + drop + store = add;
        let g = 0x1::from_bcs::from_bytes<|u64, u64|u64 has copy + drop + store>(
            bcs::to_bytes(&f),
        );
        f == g
    }

    // The same target and capture, captured at different positions.
    public fun different_mask(): bool {
        let f: |u64|u64 has copy + drop = |b| sub(7, b);
        let g: |u64|u64 has copy + drop = |a| sub(a, 7);
        f == g
    }

    public fun nested_closures(): bool {
        let inner: |u64|u64 has copy + drop + store = |b| add(7, b);
        let other: |u64|u64 has copy + drop + store = |b| add(8, b);
        let f: |u64|u64 has copy + drop + store = |x| apply(inner, x);
        let g: |u64|u64 has copy + drop + store = |x| apply(inner, x);
        let h: |u64|u64 has copy + drop + store = |x| apply(other, x);
        f == g && f != h
    }

    public fun nested_closures_on_the_wire(): bool {
        let inner: |u64|u64 has copy + drop + store = |b| add(7, b);
        let other: |u64|u64 has copy + drop + store = |b| add(8, b);
        let f: |u64|u64 has copy + drop + store = |x| apply(inner, x);
        let h: |u64|u64 has copy + drop + store = |x| apply(other, x);
        f == wire(&f) && wire(&f) != h
    }

    public fun closures_in_vectors(): bool {
        let f: |u64|u64 has copy + drop = |b| add(7, b);
        let g: |u64|u64 has copy + drop = |b| add(7, b);
        let h: |u64|u64 has copy + drop = |b| add(8, b);
        vector[f, h] == vector[g, h] && vector[f] != vector[h]
    }

    public fun closures_in_structs(): bool {
        let a = Wrapper { f: |b| add(7, b) };
        let b = Wrapper { f: |b| add(7, b) };
        let c = Wrapper { f: |b| add(8, b) };
        a == b && a != c
    }

    public fun wire_struct_capture_with_type_args(): bool {
        let p = Point { x: 1, y: 2 };
        let f: |u64|u64 has copy + drop + store = |b| pick<Point>(p, b);
        f == wire(&f)
    }

    public fun wire_differs_from_wire(): bool {
        let f: |u64|u64 has copy + drop + store = |b| add(7, b);
        let g: |u64|u64 has copy + drop + store = |b| add(8, b);
        wire(&f) == wire(&g)
    }

    public fun wire_on_the_left(): bool {
        let f: |u64|u64 has copy + drop + store = |b| add(7, b);
        let g: |u64|u64 has copy + drop + store = |b| add(8, b);
        wire(&f) == f && wire(&g) != f
    }

    // Equality never loads the target, so it need not exist.
    public fun missing_target(): bool {
        let bytes = x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730c6e6f745f615f6d6574686f64000000000000000000";
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(bytes);
        let g = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(bytes);
        f == g
    }

    public fun compare_across_gc(): bool {
        let d1 = vector[1u8, 2u8];
        let d2 = vector[1u8, 2u8];
        let f: |u64|u64 has copy + drop + store = |k| tally(d1, k);
        let g: |u64|u64 has copy + drop + store = |k| tally(d2, k);
        let w = wire(&g);
        0x0::test_utils::force_gc();
        f == g && f == w
    }
}

// RUN: execute 0x99::closure_eq::same_target
// CHECK: results: true

// RUN: execute 0x99::closure_eq::different_target
// CHECK: results: false

// RUN: execute 0x99::closure_eq::different_type_arg
// CHECK: results: false

// RUN: execute 0x99::closure_eq::same_capture
// CHECK: results: true

// RUN: execute 0x99::closure_eq::different_capture
// CHECK: results: false

// RUN: execute 0x99::closure_eq::vector_capture
// CHECK: results: true

// RUN: execute 0x99::closure_eq::vector_capture_prefix
// CHECK: results: false

// RUN: execute 0x99::closure_eq::struct_capture
// CHECK: results: true

// RUN: execute 0x99::closure_eq::wire_equals_packed
// CHECK: results: true

// RUN: execute 0x99::closure_eq::wire_differs_from_packed
// CHECK: results: false

// RUN: execute 0x99::closure_eq::wire_equals_wire
// CHECK: results: true

// RUN: execute 0x99::closure_eq::wire_noncapturing
// CHECK: results: true

// RUN: execute 0x99::closure_eq::different_mask
// CHECK: results: false

// RUN: execute 0x99::closure_eq::nested_closures
// CHECK: results: true

// RUN: execute 0x99::closure_eq::nested_closures_on_the_wire
// CHECK: results: true

// RUN: execute 0x99::closure_eq::closures_in_vectors
// CHECK: results: true

// RUN: execute 0x99::closure_eq::closures_in_structs
// CHECK: results: true

// RUN: execute 0x99::closure_eq::wire_struct_capture_with_type_args
// CHECK: results: true

// RUN: execute 0x99::closure_eq::wire_differs_from_wire
// CHECK: results: false

// RUN: execute 0x99::closure_eq::wire_on_the_left
// CHECK: results: true

// RUN: execute 0x99::closure_eq::missing_target
// CHECK: results: true

// RUN: execute 0x99::closure_eq::compare_across_gc
// CHECK: results: true
// CHECK-GC-COUNT: 1
