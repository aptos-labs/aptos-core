// A function value read out of a container field is of unknown origin even when no field has
// its type directly. Expected: `run` rejected.
module 0x42::other {
    struct C has key { v: u64 }
    public fun get(): u64 acquires C { if (exists<C>(@0x42)) C[@0x42].v else 0 }
    public fun bump() acquires C { if (exists<C>(@0x42)) C[@0x42].v = C[@0x42].v + 1 }
}
module 0x43::cf {
    use 0x42::other;
    use std::vector;
    struct Hooks has key { v: vector<|u64| u64 has copy + drop + store> }
    public fun run(a: address): bool acquires Hooks {
        let before = other::get();
        let f = *vector::borrow(&borrow_global<Hooks>(a).v, 0);
        f(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
