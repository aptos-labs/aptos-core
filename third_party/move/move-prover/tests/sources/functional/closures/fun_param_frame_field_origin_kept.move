// A function value read from a `T`-framed field of a struct that is never borrowed mutably is
// bounded by the field's frame: passed to a `T`-framed parameter, the claim that `S` is
// unchanged verifies. See fun_param_frame_field_origin_overwritten for the rejected reads.
module 0x42::fun_param_frame_field_origin_kept {
    struct S has key { v: u64 }
    struct T has key { v: u64 }

    struct Hd has drop, copy { f: |u64| has drop + copy }
    spec Hd {
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun pure_fn(x: u64) { let _ = x; }

    fun apply_t(f: |u64| has drop + copy) { f(1) }
    spec apply_t {
        pragma opaque;
        modifies_of<f>(_x: u64) T[@0x42];
        ensures global<S>(@0x42) == old(global<S>(@0x42));
    }

    fun local_struct() acquires S {
        let v0 = S[@0x42].v;
        let h = Hd { f: |x| pure_fn(x) };
        apply_t(h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    // An incoming `&mut` parameter that is only read here.
    fun read_only_mut_param(h: &mut Hd) acquires S {
        let v0 = S[@0x42].v;
        apply_t(h.f);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
