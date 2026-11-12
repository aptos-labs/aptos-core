// A value of unknown origin handed to a parameter with no `*` frame, directly or as a closure's
// capture: the parameter's predicates are read against the variants the program names, which
// such a value is not. The frame check rejects both sites. Expected: ERR at `probe(h)` and at
// the capture of `h` for `call_g`.
module 0x42::other {
    struct C has key { v: u64 }
    public fun get(): u64 acquires C { if (exists<C>(@0x42)) C[@0x42].v else 0 }
    public fun bump() acquires C { if (exists<C>(@0x42)) C[@0x42].v = C[@0x42].v + 1 }
}
module 0x43::r1 {
    use std::vector;
    fun probe(f: |u64| u64 has copy + drop): |u64| u64 has copy + drop { f }
    spec probe { pragma opaque; ensures result == f; ensures result_of<result>(1) == result_of<f>(1); }
    public fun run(v: vector<|u64| u64 has copy + drop>): bool {
        let h = *vector::borrow(&v, 0);
        probe(h); let a = h(1);
        probe(h); let b = h(1);
        a == b
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}
module 0x43::r2 {
    use 0x42::other;
    use std::vector;
    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 { g(y) }
    spec call_g { pragma opaque; aborts_if aborts_of<g>(y); ensures result == result_of<g>(y); }
    public fun run(v: vector<|u64| u64 has copy + drop>): bool {
        let h = *vector::borrow(&v, 0);
        let k: |u64| u64 has copy + drop = |y| call_g(h, y);
        let before = other::get();
        k(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}

// The frame check does not run under the Lean backend (#20966).
// exclude_for: lean
