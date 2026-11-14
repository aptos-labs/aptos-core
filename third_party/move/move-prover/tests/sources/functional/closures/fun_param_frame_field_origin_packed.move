// A function value read from a field of a struct is trusted only along explicitly tracked
// definitions, which include packs: each struct operand of a pack is traced, so a mutable borrow
// of the packed value before it was packed is seen. Each client stores a writer of `S` in the
// `T`-framed field of `h` through a mutable borrow, packs `h` (one level, two levels, an enum
// variant), reads the field back through the outer value and claims `S` unchanged, which is
// false at runtime; each read must be rejected. See fun_param_frame_field_origin_packed_kept for
// the accepted reads.
module 0x42::fun_param_frame_field_origin_packed {
    struct S has key { v: u64 }
    struct T has key { v: u64 }

    struct Hd has drop, copy { f: |u64| has drop + copy }
    spec Hd {
        modifies_of<f>(_x: u64) T[@0x42];
    }

    struct Outer has drop, copy { h: Hd }
    struct Outer2 has drop, copy { o: Outer }
    enum E has drop, copy {
        A { h: Hd },
        B { h: Hd, n: u64 },
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

    fun apply_t(g: |u64| has drop + copy, x: u64) { g(x) }
    spec apply_t {
        modifies_of<g>(_x: u64) T[@0x42];
    }

    fun apply_t_opaque(g: |u64| has drop + copy) { g(1) }
    spec apply_t_opaque {
        pragma opaque;
        modifies_of<g>(_x: u64) T[@0x42];
    }

    fun call_h(f: |u64| has drop + copy, x: u64) { f(x) }
    spec call_h {
        pragma opaque;
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun packed_captured() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        *(&mut h.f) = |x| bump_s(x);
        let o = Outer { h };
        let g = o.h.f;
        call_h(|x| apply_t(g, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun packed_twice() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        *(&mut h.f) = |x| bump_s(x);
        let o2 = Outer2 { o: Outer { h } };
        apply_t_opaque(o2.o.h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun packed_variant() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        *(&mut h.f) = |x| bump_s(x);
        let e = E::B { h, n: 1 };
        apply_t_opaque(e.h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
