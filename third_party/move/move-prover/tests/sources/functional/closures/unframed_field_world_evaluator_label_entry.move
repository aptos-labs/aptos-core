// A behavioral target whose specification names a state label in a predicate over an unframed
// field value, where the label walks back to the entry state for the memories the predicate
// ranges over. Inside the target's evaluators (Boogie functions) the entry state is a parameter
// or a bound state, never `old(..)`, which Boogie rejects in a function; before this was fixed
// the whole verification batch failed to compile.
module 0x42::el {
    use std::signer;

    struct R has key { v: u64 }
    struct S has key { v: u64 }
    struct Cb has copy, drop { f: |u64| u64 has copy + drop }

    fun reads_r(x: u64): u64 acquires R { R[@0x42].v + x }
    spec reads_r {
        aborts_if !exists<R>(@0x42);
        aborts_if R[@0x42].v + x > MAX_U64;
        ensures result == R[@0x42].v + x;
    }

    // Publishes `S`, then runs the continuation; the abort clause names the state after the
    // publish, at which `R` is unchanged, so the label resolves to the entry state for `R`.
    fun t(s: &signer, c: &Cb, x: u64): u64 {
        move_to(s, S { v: x });
        (c.f)(x)
    }
    spec t {
        pragma opaque;
        aborts_if exists<S>(signer::address_of(s));
        aborts_if S1 |~ aborts_of<c.f>(x);
        ensures ..S1 |~ publish<S>(signer::address_of(s), S { v: x });
        ensures result == (S1.. |~ result_of<c.f>(x));
    }

    // Names `t` in behavioral predicates, so its evaluators are emitted.
    fun client(s: &signer, c: &Cb, x: u64): u64 { t(s, c, x) }
    spec client {
        aborts_if aborts_of<t>(s, c, x);
        ensures ensures_of<t>(s, c, x, result);
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
