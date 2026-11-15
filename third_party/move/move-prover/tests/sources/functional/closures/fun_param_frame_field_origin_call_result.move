// A function value read from a field of a struct reached through a call result has an unknown
// origin: the call's referent is not tracked, so a mutable borrow of it elsewhere (here
// `vector::borrow_mut`) may have replaced the field. The first two clients store a writer of `S`
// in the `T`-framed field, read it back through `vector::borrow`, and claim `S` unchanged, which
// is false at runtime; each read must be rejected. The last client never borrows the vector
// mutably and its claim holds, but the read is rejected as well: a completeness cost.
module 0x42::fun_param_frame_field_origin_call_result {
    use std::vector;

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

    // Codex's shape: captured into a closure over apply_t, passed to an opaque HOF.
    fun client_captured() acquires S {
        let v0 = S[@0x42].v;
        let v = vector[Hd { f: |x| pure_fn(x) }];
        *(&mut vector::borrow_mut(&mut v, 0).f) = |x| bump_s(x);
        let g = vector::borrow(&v, 0).f;
        call_h(|x| apply_t(g, x), 1);
        spec { assert global<S>(@0x42).v == v0; };
    }

    // The read passed straight to an opaque T-framed function.
    fun client_direct() acquires S {
        let v0 = S[@0x42].v;
        let v = vector[Hd { f: |x| pure_fn(x) }];
        *(&mut vector::borrow_mut(&mut v, 0).f) = |x| bump_s(x);
        apply_t_opaque(vector::borrow(&v, 0).f);
        spec { assert global<S>(@0x42).v == v0; };
    }

    // Rejected conservatively: the vector is never borrowed mutably.
    fun client_kept() acquires S {
        let v0 = S[@0x42].v;
        let v = vector[Hd { f: |x| pure_fn(x) }];
        apply_t_opaque(vector::borrow(&v, 0).f);
        spec { assert global<S>(@0x42).v == v0; };
    }
}
