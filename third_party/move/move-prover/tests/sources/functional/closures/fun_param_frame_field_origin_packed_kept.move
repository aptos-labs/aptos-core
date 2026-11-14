// Reads through tracked packs of a struct that is never borrowed mutably keep the field's
// frame: the claims that `S` is unchanged verify. See fun_param_frame_field_origin_packed.
module 0x42::fun_param_frame_field_origin_packed_kept {
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
        let o = Outer { h };
        let g = o.h.f;
        call_h(|x| apply_t(g, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun packed_twice() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        let o2 = Outer2 { o: Outer { h } };
        apply_t_opaque(o2.o.h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    fun packed_variant() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        let e = E::B { h, n: 1 };
        apply_t_opaque(e.h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
