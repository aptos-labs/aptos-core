// Tests decompilation of mutable field borrows on public structs across modules.
// Exercises the borrow_mut$S$N API path which is distinct from borrow$S$N.
// Also tests sequential borrows, reborrow of the same field, and vector fields.
// Based on compiler-v2 tests: struct_borrow_field_api.move, struct_mutate_vector.move,
//   struct_nested_mutate.move

module 0x42::borrow_defs {
    /// Inner struct with a single field.
    public struct Inner has copy, drop {
        h: u64,
    }

    /// Outer struct referencing Inner and having a plain u64 field.
    public struct Outer has copy, drop {
        f: u64,
        g: Inner,
    }

    /// Struct whose field is a vector.
    public struct VecHolder has copy, drop {
        data: vector<u8>,
    }
}

module 0x42::borrow_consumer {
    use 0x42::borrow_defs::{Inner, Outer, VecHolder};

    // -----------------------------------------------------------------------
    // Immutable borrows
    // -----------------------------------------------------------------------

    fun read_fields(s: &Outer): (u64, u64) {
        let f = *&s.f;
        let h = *&s.g.h;
        (f, h)
    }

    // -----------------------------------------------------------------------
    // Mutable borrows — scalar fields
    // -----------------------------------------------------------------------

    fun increment_f(s: &mut Outer) {
        let r = &mut s.f;
        *r = *r + 1;
    }

    fun increment_h(s: &mut Outer) {
        let r = &mut s.g.h;
        *r = *r + 1;
    }

    /// Sequential mutable borrows of two different fields.
    fun update_both(s: &mut Outer, new_f: u64, new_h: u64) {
        let rf = &mut s.f;
        *rf = new_f;
        let rh = &mut s.g.h;
        *rh = new_h;
    }

    /// Reborrow of the same field sequentially.
    fun reborrow_f(s: &mut Outer) {
        let r1 = &mut s.f;
        *r1 = 88;
        let r2 = &mut s.f;
        *r2 = *r2 + 1;
    }

    /// Immutable borrow of a field through an owned mutable reference.
    fun read_f_through_mut_ref(): u64 {
        let s = Outer { f: 6, g: Inner { h: 5 } };
        let r = &mut s;
        *&r.f
    }

    // -----------------------------------------------------------------------
    // Vector field access
    // -----------------------------------------------------------------------

    fun make_holder(byte: u8): VecHolder {
        VecHolder { data: vector[byte] }
    }

    fun read_first_byte(v: &VecHolder): u8 {
        *&v.data[0]
    }

    fun set_first_byte(v: &mut VecHolder, byte: u8) {
        let r = &mut v.data[0];
        *r = byte;
    }

    // -----------------------------------------------------------------------
    // End-to-end
    // -----------------------------------------------------------------------

    fun round_trip(): u64 {
        let s = Outer { f: 10, g: Inner { h: 20 } };
        increment_f(&mut s);
        increment_h(&mut s);
        let (f, h) = read_fields(&s);
        f + h
    }
}
