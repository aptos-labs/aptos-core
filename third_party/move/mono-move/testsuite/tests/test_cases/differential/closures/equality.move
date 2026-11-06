// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}

module 0x99::closure_eq {
    use std::bcs;

    struct Point has copy, drop, store { x: u64, y: u8 }

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun mul(a: u64, b: u64): u64 {
        a * b
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

    public fun different_target_neq(): bool {
        let f: |u64, u64|u64 has copy + drop = add;
        let g: |u64, u64|u64 has copy + drop = mul;
        f != g
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

    // BCS is little-endian, so byte order and value order disagree here.
    public fun capture_byte_order(): bool {
        let f: |u64|u64 has copy + drop = |b| add(1, b);
        let g: |u64|u64 has copy + drop = |b| add(256, b);
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

    // Calling the wire-backed closure resolves its target and materializes its
    // captures, so this compares a resolved closure against an unresolved one.
    public fun resolved_equals_unresolved(): bool {
        let f: |u64|u64 has copy + drop + store = |b| add(7, b);
        let g = wire(&f);
        let _ = g(1);
        f == g
    }

    public fun resolved_equals_wire(): bool {
        let f: |u64|u64 has copy + drop + store = |b| add(7, b);
        let g = wire(&f);
        let h = wire(&f);
        let _ = g(1);
        g == h
    }
}

// RUN: execute 0x99::closure_eq::same_target
// CHECK: results: true

// RUN: execute 0x99::closure_eq::different_target
// CHECK: results: false

// RUN: execute 0x99::closure_eq::different_target_neq
// CHECK: results: true

// RUN: execute 0x99::closure_eq::different_type_arg
// CHECK: results: false

// RUN: execute 0x99::closure_eq::same_capture
// CHECK: results: true

// RUN: execute 0x99::closure_eq::different_capture
// CHECK: results: false

// RUN: execute 0x99::closure_eq::capture_byte_order
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

// RUN: execute 0x99::closure_eq::resolved_equals_unresolved
// CHECK: results: true

// RUN: execute 0x99::closure_eq::resolved_equals_wire
// CHECK: results: true
