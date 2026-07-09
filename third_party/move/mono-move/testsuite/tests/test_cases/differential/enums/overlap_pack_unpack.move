// RUN: publish
module 0x77::enum_overlap {
    enum E has drop { V { f: u64 } }
    enum Pair has drop { P { a: u64, b: u64 } }
    enum Tri has drop { T { a: u64, b: u64, c: u64 } }

    fun id(x: u64): u64 { x }
    fun add(x: u64, y: u64): u64 { x + y }
    fun add3(x: u64, y: u64, z: u64): u64 { x + y + z }

    fun get_v(e: E): u64 { match (e) { V { f } => f } }
    fun get_pair(p: Pair): u64 { match (p) { P { a, b } => a * 100 + b } }

    // Pack a call result, immediately consume via call.
    fun pack_callresult_to_call(): u64 {
        get_v(E::V { f: id(7) })
    }

    // Multi-field pack from call results, consumed via call.
    fun pack_multi_callresult(): u64 {
        get_pair(Pair::P { a: id(3), b: id(4) })
    }

    // Unpack outputs passed directly as next-call args.
    fun unpack_to_call(): u64 {
        let p = Pair::P { a: 10, b: 20 };
        let Pair::P { a, b } = p;
        add(a, b)
    }

    fun make_tri(x: u64): Tri { Tri::T { a: x, b: x + 1, c: x + 2 } }

    fun unpack_callenum_to_call(x: u64): u64 {
        let Tri::T { a, b, c } = make_tri(x);
        add3(a, b, c)
    }

    // Unpack then repack into a new enum consumed by a call (swapped fields).
    fun unpack_repack(): u64 {
        let Pair::P { a, b } = Pair::P { a: 5, b: 6 };
        get_pair(Pair::P { a: b, b: a })
    }

    // Pack result stored where an arg source lived (call result, then dead).
    fun pack_reuse_local(n: u64): u64 {
        let t = id(n);
        let e = E::V { f: t };
        get_v(e)
    }
}

// RUN: execute 0x77::enum_overlap::pack_callresult_to_call
// CHECK: results: 7

// RUN: execute 0x77::enum_overlap::pack_multi_callresult
// CHECK: results: 304

// RUN: execute 0x77::enum_overlap::unpack_to_call
// CHECK: results: 30

// RUN: execute 0x77::enum_overlap::unpack_callenum_to_call --args 10
// CHECK: results: 33

// RUN: execute 0x77::enum_overlap::unpack_repack
// CHECK: results: 605

// RUN: execute 0x77::enum_overlap::pack_reuse_local --args 42
// CHECK: results: 42
