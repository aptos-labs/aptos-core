// Resources with type parameters under unknown code. A `*` field callback may change a
// generic resource of another module in any instantiation, named or abstract; the module that
// owns the resource keeps every instantiation while it is on the stack (re-entry aborts on
// access), and nothing else. Each function states its required verdict.
module 0x42::owner {
    struct R<phantom T> has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    spec Cb { modifies_of<f> *; }

    public fun make(s: &signer) {
        move_to(s, R<u64> { v: 0 });
        move_to(s, R<bool> { v: 0 });
    }
    public fun set<T>(v: u64) acquires R {
        if (exists<R<T>>(@0x42)) R<T>[@0x42].v = v;
    }

    // Own resources, every instantiation, are locked while this module runs. Verifies.
    public fun run_own<T>(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec run_own {
        ensures global<R<T>>(@0x42) == old(global<R<T>>(@0x42));
        ensures global<R<u64>>(@0x42) == old(global<R<u64>>(@0x42));
        ensures global<R<bool>>(@0x42) == old(global<R<bool>>(@0x42));
    }

    // Another module's generic resource is not locked. Rejected.
    public fun run_other<T>(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec run_other {
        ensures global<0x43::other::S<T>>(@0x43) == old(global<0x43::other::S<T>>(@0x43));
    }
}

module 0x43::other {
    struct S<phantom T> has key { v: u64 }
    public fun make(s: &signer) { move_to(s, S<u64> { v: 0 }); }
    public fun set<T>(v: u64) acquires S {
        if (exists<S<T>>(@0x43)) S<T>[@0x43].v = v;
    }
}

module 0x44::user {
    use 0x42::owner;
    use 0x43::other;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    spec Cb { modifies_of<f> *; }

    // The callback may call `owner::set<u64>`. Rejected.
    public fun run_u64(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec run_u64 {
        ensures global<owner::R<u64>>(@0x42) == old(global<owner::R<u64>>(@0x42));
    }

    // Or `owner::set<T>` for the caller's instantiation. Rejected.
    public fun run_gen<T>(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec run_gen {
        ensures global<owner::R<T>>(@0x42) == old(global<owner::R<T>>(@0x42));
    }

    // Or `other::set<bool>`, an instantiation no code in the program names. Rejected.
    public fun run_bool(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec run_bool {
        ensures global<other::S<bool>>(@0x43) == old(global<other::S<bool>>(@0x43));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
