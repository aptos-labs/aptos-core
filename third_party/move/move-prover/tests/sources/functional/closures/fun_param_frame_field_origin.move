// A function value read from a struct field is bounded by the field's declared frame. A field
// without a declaration has no known bound, so its value may only go to a parameter declaring
// `modifies_of<p> *`. Kept apart from `fun_param_frame_transparent`: a declared field frame is
// havocked by every loop that invokes a function value in the program.
module 0x42::fun_param_frame_field_origin {
    struct S has key { v: u64 }

    struct Cb has drop { f: |u64| has drop + copy }

    struct CbS has drop { f: |u64| has drop + copy }
    spec CbS {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    enum CbE has drop {
        Plain { f: |u64| has drop + copy },
        Framed { f: |u64| has drop + copy },
    }

    fun apply(f: |u64| has drop) { f(1) }
    spec apply {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_s(f: |u64| has drop) { f(1) }
    spec apply_s {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    fun apply_w(f: |u64| has drop) { f(1) }
    spec apply_w {
        modifies_of<f> *;
    }

    // Rejected: no bound is known for an undeclared field.
    fun unframed_to_empty(cb: Cb) { apply(cb.f) }

    fun unframed_unpacked_to_empty(cb: Cb) {
        let Cb { f } = cb;
        apply(f)
    }

    fun unframed_variant_to_empty(cb: CbE) { apply(cb.f) }

    // Rejected: the declared field frame is wider than the parameter's.
    fun framed_to_empty(cb: CbS) { apply(cb.f) }

    // Accepted.
    fun unframed_to_wild(cb: Cb) { apply_w(cb.f) }

    fun framed_to_framed(cb: CbS) { apply_s(cb.f) }

    fun framed_to_wild(cb: CbS) { apply_w(cb.f) }
}
