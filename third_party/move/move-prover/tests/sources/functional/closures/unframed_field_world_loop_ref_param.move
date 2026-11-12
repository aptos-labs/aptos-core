// A parameter that is a reference to a function value does not bound what a loop invoking
// through it runs: a write through the reference may store a value read from a field without a
// frame. The loop then changes any memory.
module 0x42::other {
    struct C has key { v: u64 }
    public fun get(): u64 acquires C { if (exists<C>(@0x42)) C[@0x42].v else 0 }
    public fun bump() acquires C { if (exists<C>(@0x42)) C[@0x42].v = C[@0x42].v + 1 }
}
module 0x43::p {
    use 0x42::other;
    struct Cb has copy, drop { f: |u64| u64 has copy + drop }

    // Rejected: the stored field value may change any memory.
    public fun run(r: &mut |u64| u64 has copy + drop, c: &Cb, n: u64) {
        *r = c.f;
        let i = 0;
        while (i < n) { (*r)(i); i = i + 1 }
    }
    spec run {
        pragma aborts_if_is_partial;
        ensures global<other::C>(@0x42) == old(global<other::C>(@0x42));
    }

    // Rejected: the same, observed through a read.
    public fun run_bool(r: &mut |u64| u64 has copy + drop, c: &Cb, n: u64): bool {
        let before = other::get();
        *r = c.f;
        let i = 0;
        while (i < n) { (*r)(i); i = i + 1 };
        other::get() == before
    }
    spec run_bool { pragma aborts_if_is_partial; ensures result; }

    // Rejected: the store inside the loop.
    public fun run_store_in_loop(r: &mut |u64| u64 has copy + drop, c: &Cb, n: u64) {
        let i = 0;
        while (i < n) { *r = c.f; (*r)(i); i = i + 1 }
    }
    spec run_store_in_loop {
        pragma aborts_if_is_partial;
        ensures global<other::C>(@0x42) == old(global<other::C>(@0x42));
    }

    // Rejected: the value copied out of the reference before the loop.
    public fun run_copied(r: &mut |u64| u64 has copy + drop, c: &Cb, n: u64) {
        *r = c.f;
        let g = *r;
        let i = 0;
        while (i < n) { g(i); i = i + 1 }
    }
    spec run_copied {
        pragma aborts_if_is_partial;
        ensures global<other::C>(@0x42) == old(global<other::C>(@0x42));
    }

    // Rejected: the same through an opaque function.
    public fun run_opaque(r: &mut |u64| u64 has copy + drop, c: &Cb, n: u64) {
        *r = c.f;
        let i = 0;
        while (i < n) { (*r)(i); i = i + 1 }
    }
    spec run_opaque {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures global<other::C>(@0x42) == old(global<other::C>(@0x42));
    }

    // Rejected: through an immutable reference, which a caller may point at any value.
    public fun run_imm(r: &|u64| u64 has copy + drop, n: u64) {
        let i = 0;
        while (i < n) { (*r)(i); i = i + 1 }
    }
    spec run_imm {
        pragma aborts_if_is_partial;
        ensures global<other::C>(@0x42) == old(global<other::C>(@0x42));
    }

    // Verifies: the loop is reached and left, so the rejections above are not vacuous.
    public fun reaches_end(r: &mut |u64| u64 has copy + drop, c: &Cb, n: u64): u64 {
        *r = c.f;
        let i = 0;
        while ({ spec { invariant i <= n; }; i < n }) { (*r)(i); i = i + 1 };
        i
    }
    spec reaches_end { pragma aborts_if_is_partial; ensures result == n; }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
