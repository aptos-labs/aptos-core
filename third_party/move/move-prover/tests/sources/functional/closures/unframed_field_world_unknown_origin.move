// A function value of unknown origin can have any function type: no declared field bounds the
// types such a value may have. Each `run` reads one from a place that declares no frame for it
// (a vector parameter, the result of an invoked parameter, a generic field instantiated at a
// function type, the result of an opaque function, a vector field beside a `reads_of *` field)
// and claims another module's resource unchanged across invoking it. Expected: every `run`
// rejected.
module 0x42::other {
    struct C has key { v: u64 }
    public fun get(): u64 acquires C { if (exists<C>(@0x42)) C[@0x42].v else 0 }
    public fun bump() acquires C { if (exists<C>(@0x42)) C[@0x42].v = C[@0x42].v + 1 }
}
module 0x43::qa {
    use 0x42::other;
    use std::vector;
    public fun run(v: vector<|u64| u64 has copy + drop>): bool {
        let before = other::get();
        let f = *vector::borrow(&v, 0);
        f(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}
module 0x43::qb {
    use 0x42::other;
    public fun run(mk: || (|u64| u64 has drop) has drop): bool {
        let g = mk();
        let before = other::get();
        g(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}
module 0x43::qc {
    use 0x42::other;
    use std::vector;
    struct Box<T: store> has key { v: vector<T> }
    public fun put<T: store>(s: &signer, x: T) { move_to(s, Box<T> { v: vector[x] }) }
    public fun run(a: address): bool acquires Box {
        let before = other::get();
        let g = *vector::borrow(&Box<|u64| u64 has copy + drop + store>[a].v, 0);
        g(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}
module 0x42::dep {
    use 0x42::other;
    #[persistent]
    fun bumper(x: u64): u64 { other::bump(); x }
    public fun hook(): |u64| u64 has drop { bumper }
    spec hook { pragma opaque; }
}
module 0x43::qd {
    use 0x42::other;
    use 0x42::dep;
    public fun run(): bool {
        let g = dep::hook();
        let before = other::get();
        g(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}
module 0x43::qe {
    use 0x42::other;
    use std::vector;
    struct Cb has key { f: |u64| u64 has copy + drop + store, v: vector<|u64| u64 has copy + drop + store> }
    spec Cb { reads_of<f> *; }
    public fun run(a: address): bool acquires Cb {
        let before = other::get();
        let g = *vector::borrow(&Cb[a].v, 0);
        g(1);
        other::get() == before
    }
    spec run { pragma aborts_if_is_partial; ensures result; }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
