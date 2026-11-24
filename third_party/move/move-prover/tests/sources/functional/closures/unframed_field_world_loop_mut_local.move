// A local function value rewritten through a mutable reference has no origin the loop analysis
// can see. Expected: `bad` rejected.
module 0x42::other {
    struct C has key { v: u64 }
    public fun get(): u64 acquires C { if (exists<C>(@0x42)) C[@0x42].v else 0 }
    public fun bump() acquires C { if (exists<C>(@0x42)) C[@0x42].v = C[@0x42].v + 1 }
}
module 0x43::lm {
    use 0x42::other;
    struct Cb has copy, drop { f: |u64| u64 has copy + drop }
    public fun bad(c: &Cb, n: u64): bool {
        let before = other::get();
        let g: |u64| u64 has copy + drop = |x| x;
        let r = &mut g;
        *r = c.f;
        let i = 0;
        while (i < n) { g(i); i = i + 1 };
        other::get() == before
    }
    spec bad { pragma aborts_if_is_partial; ensures result; }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
