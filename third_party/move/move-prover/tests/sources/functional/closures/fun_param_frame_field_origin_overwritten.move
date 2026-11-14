// A field with a declared frame is trusted only while nothing can have replaced it: a function
// value read from a struct that the function borrows mutably has an unknown origin. Each client
// stores a writer of `S` in the `T`-framed field through a mutable borrow other than the read,
// then passes the field to a `T`-framed parameter of an opaque function claiming `S` unchanged;
// each assert is false at runtime, and each read must be rejected.
// See fun_param_frame_field_origin_kept for the accepted reads.
module 0x42::fun_param_frame_field_origin_overwritten {
    struct S has key { v: u64 }
    struct T has key { v: u64 }

    struct Hd has drop, copy { f: |u64| has drop + copy }
    spec Hd {
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun bump_s(x: u64) acquires S { let s = &mut S[@0x42]; s.v = s.v + x; }
    spec bump_s {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + x > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + x;
    }

    fun pure_fn(x: u64) { let _ = x; }

    fun apply_t(f: |u64| has drop + copy) { f(1) }
    spec apply_t {
        pragma opaque;
        modifies_of<f>(_x: u64) T[@0x42];
        ensures global<S>(@0x42) == old(global<S>(@0x42));
    }

    fun set_fn(r: &mut |u64| has drop + copy) { *r = |x| bump_s(x) }

    fun through_field_borrow() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        *(&mut h.f) = |x| bump_s(x);
        apply_t(h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun through_named_ref() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        let r = &mut h.f;
        *r = |x| bump_s(x);
        apply_t(h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun through_mut_param(h: &mut Hd) acquires S {
        let v0 = S[@0x42].v;
        let r = &mut h.f;
        *r = |x| bump_s(x);
        apply_t(h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    // `set_fn`'s parameter is rejected too: the reference passed has an unknown origin.
    fun through_setter() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        set_fn(&mut h.f);
        apply_t(h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
