// A function value that a mutable reference can replace is not judged by where it was first
// made: the reference may have been written since. Each value below reaches a parameter with
// a frame that does not admit what was stored, and must be rejected. The functions
// taking a function-typed `&mut` parameter also claim `S` is unchanged, which is false at
// runtime.
module 0x42::state_mut_ref {
    struct S has key { v: u64 }
    struct T has key { v: u64 }

    public fun bump() acquires S {
        let s = &mut S[@0x42];
        s.v = s.v + 1;
    }
    spec bump {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + 1;
    }
}

module 0x42::fun_param_frame_mut_ref {
    use 0x42::state_mut_ref::{Self, S, T};

    fun apply(f: |u64| has drop) { f(1) }
    spec apply {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A writer stored through an incoming `&mut` parameter, then forwarded.
    fun through_mut_param(f: &mut |u64| has copy + drop) {
        *f = |_x| state_mut_ref::bump();
        apply(*f)
    }
    spec through_mut_param {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // The same, written through a copy of the reference.
    fun through_mut_param_copy(f: &mut |u64| has copy + drop) {
        let r = f;
        *r = |_x| state_mut_ref::bump();
        apply(*f)
    }
    spec through_mut_param_copy {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A function field framed for `T`; a writer of `S` is stored through the reference the
    // value is then read from.
    struct H has drop { f: |u64| has copy + drop }
    spec H {
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun apply_t(f: |u64| has drop) { f(1) }
    spec apply_t {
        modifies_of<f>(_x: u64) T[@0x42];
    }

    fun through_field_ref(h: &mut H) {
        let r = &mut h.f;
        *r = |_x| state_mut_ref::bump();
        apply_t(*r)
    }

    // The reference is handed to a function, which may store anything behind it.
    fun set(r: &mut |u64| has copy + drop) { *r = |_x| state_mut_ref::bump() }

    fun through_setter(h: &mut H) {
        let r = &mut h.f;
        set(r);
        apply_t(*r)
    }

    // Accepted: an immutable reference forwarded unchanged; the value behind it cannot change,
    // so it is judged at the caller, which passes a pure closure. (Kept here rather than in
    // `fun_param_frame_transparent_ok`: verifying it hits an unrelated Boogie name-resolution
    // error for an invoked `&` function parameter, on main too.)
    fun apply_ref(f: &|u64| has copy + drop) { apply(*f) }

    fun through_ref() {
        let c: |u64| has copy + drop = |_x| ();
        apply_ref(&c)
    }
}
