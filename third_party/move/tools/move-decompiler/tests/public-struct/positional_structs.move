// Tests decompilation of public positional (tuple) structs used across modules.
// Positional struct fields are stored as _0, _1, ... in the binary.
// Based on compiler-v2 tests: positional_pack_unpack_api.move, positional_field_api.move

module 0x42::pos_defs {
    /// Simple positional struct with two fields.
    public struct Pair(u64, bool) has copy, drop;

    /// Generic positional struct.
    public struct Wrapper<T: copy + drop>(T) has copy, drop;

    /// Nested positional: holds a Pair and a Wrapper.
    public struct NestedPair<T: copy + drop>(Pair, Wrapper<T>) has copy, drop;

    /// Positional struct wrapping a vector.
    public struct VecWrap<T: copy + drop>(vector<T>) has copy, drop;
}

module 0x42::pos_consumer {
    use 0x42::pos_defs::{Pair, Wrapper, NestedPair, VecWrap};

    // -----------------------------------------------------------------------
    // Pack and unpack
    // -----------------------------------------------------------------------

    fun make_pair(x: u64, b: bool): Pair {
        Pair(x, b)
    }

    fun unpack_pair(p: Pair): (u64, bool) {
        let Pair(x, y) = p;
        (x, y)
    }

    fun make_wrapper(v: u64): Wrapper<u64> {
        Wrapper(v)
    }

    fun unpack_wrapper(w: Wrapper<u64>): u64 {
        let Wrapper(x) = w;
        x
    }

    fun unpack_nested(n: NestedPair<u64>): (u64, bool, u64) {
        let NestedPair(Pair(a, b), Wrapper(v)) = n;
        (a, b, v)
    }

    fun unpack_vecwrap(vw: VecWrap<u64>): vector<u64> {
        let VecWrap(v) = vw;
        v
    }

    // -----------------------------------------------------------------------
    // Immutable field borrows
    // -----------------------------------------------------------------------

    fun borrow_pair_fields(p: &Pair): (u64, bool) {
        let x = *&p.0;
        let y = *&p.1;
        (x, y)
    }

    fun borrow_wrapper_field(w: &Wrapper<u64>): u64 {
        *&w.0
    }

    fun borrow_nested_fields(n: &NestedPair<u64>): (u64, u64) {
        let inner_pair_0 = *&n.0.0;
        let wrapped_val = *&n.1.0;
        (inner_pair_0, wrapped_val)
    }

    fun borrow_vecwrap_field(v: &VecWrap<u64>): u64 {
        *&v.0[0]
    }

    // -----------------------------------------------------------------------
    // Mutable field borrows
    // -----------------------------------------------------------------------

    fun mutate_pair_field(p: &mut Pair, new_x: u64) {
        let r = &mut p.0;
        *r = new_x;
    }

    fun mutate_wrapper_field(w: &mut Wrapper<u64>, new_v: u64) {
        let r = &mut w.0;
        *r = new_v;
    }

    // -----------------------------------------------------------------------
    // End-to-end
    // -----------------------------------------------------------------------

    fun round_trip(): u64 {
        let p = make_pair(10, true);
        let (x, _) = unpack_pair(p);
        let w = make_wrapper(20);
        let v = unpack_wrapper(w);
        x + v
    }
}
