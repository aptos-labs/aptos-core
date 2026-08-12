// Generic fields (declared type mentions a type parameter) are int sinks:
// their single datatype declaration renders plainly for every
// instantiation, a bitwise-classified value converts at the pack/unpack
// boundary (value-preserving, asserted below), and — critically — a
// bitwise use of one instantiation must not drag sibling instantiations or
// their cones into the bitvector rendering.
module 0x42::bv_generic_field_sink {
    struct Pair<T1, T2> has copy, drop {
        a: T1,
        b: T2,
    }

    // A genuine bitwise value crossing into and out of a generic field:
    // the boundary conversions must preserve the value.
    fun mask_roundtrip(x: u64): u64 {
        let p = Pair { a: x & 15, b: true };
        p.a
    }
    spec mask_roundtrip {
        aborts_if false;
        ensures result == (x & 15);
        ensures result <= 15;
    }

    // A never-bit-opped instantiation of the same struct verifies plainly,
    // arithmetic included.
    fun plain(x: u8, y: u16): u16 {
        let p = Pair { a: x, b: y };
        (p.a as u16) + p.b
    }
    spec plain {
        aborts_if (x as u16) + y > MAX_U16;
        ensures result == x + y;
    }

    // Same boundary through an enum variant field.
    enum E<T> has copy, drop {
        V { v: T },
    }

    fun enum_roundtrip(x: u64): u64 {
        let e = E::V { v: x & 7 };
        match (e) {
            E::V { v } => v,
        }
    }
    spec enum_roundtrip {
        aborts_if false;
        ensures result == (x & 7);
    }

    // Spec-level pack and field select of a generic struct with a bitwise
    // argument: the spec function has no bitwise slot of its own, so it
    // belongs to the plain world; the crossing converts at its boundary.
    spec fun get_a<T1, T2>(p: Pair<T1, T2>): T1 {
        p.a
    }

    fun mask3(x: u64): u64 {
        x & 3
    }
    spec mask3 {
        aborts_if false;
        ensures get_a(Pair { a: result, b: false }) == (x & 3);
    }
}
