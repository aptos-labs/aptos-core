// A transparent function invoking an undeclared function-typed parameter takes whatever its
// callers pass, so every call is checked against the parameter's empty frame: an unframed field
// value is rejected there, with or without a loop in the callee. Expected: ERR at both calls.
module 0x42::other {
    struct C has key { v: u64 }
    public fun get(): u64 acquires C { if (exists<C>(@0x42)) C[@0x42].v else 0 }
    public fun bump() acquires C { if (exists<C>(@0x42)) C[@0x42].v = C[@0x42].v + 1 }
}
module 0x43::lp {
    use 0x42::other;
    struct Cb has copy, drop { f: |u64| u64 has copy + drop }
    public fun each(f: |u64| u64 has copy + drop, n: u64) {
        let i = 0;
        while (i < n) { f(i); i = i + 1 }
    }
    public fun bad(c: &Cb, n: u64): bool {
        let before = other::get(); each(c.f, n); other::get() == before
    }
    spec bad { pragma aborts_if_is_partial; ensures result; }
    // Control without the loop: must be rejected.
    public fun once(f: |u64| u64 has copy + drop) { f(0); }
    public fun bad_once(c: &Cb): bool {
        let before = other::get(); once(c.f); other::get() == before
    }
    spec bad_once { pragma aborts_if_is_partial; ensures result; }
}


// The frame check does not run under the Lean backend (#20966).
// exclude_for: lean
