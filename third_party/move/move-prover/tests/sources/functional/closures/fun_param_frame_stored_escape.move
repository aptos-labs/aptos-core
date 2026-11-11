// A function-typed parameter is exempt from the call-site frame check only when the callee
// returns it, drops it, or packs it into a struct that is returned, stored or dropped and
// never read back in the callee. Any other use relies on the frame.
// Expected: ERR at `via_vector`, `via_pack_unpack` and `via_set_field`; the other callers are
// accepted.
module 0x42::fun_param_frame_stored_escape {
    struct S has key { v: u64 }
    struct Box has drop { f: |u64| has copy + drop }
    struct Held has key { f: |u64| has copy + drop + store }
    enum Slot has drop { One { f: |u64| has copy + drop } }

    fun bump(_x: u64) acquires S { let s = &mut S[@0x42]; s.v = s.v + 1; }
    spec bump {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
    }

    public fun bump_stored(x: u64) acquires S { bump(x) }
    spec bump_stored {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
    }

    // The parameter reaches an invocation through a vector.
    fun hidden(f: |u64| has copy + drop) {
        let fs = vector[f];
        let g = fs[0];
        g(0)
    }
    spec hidden {
        pragma opaque;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // The argument is a local and neither caller has a spec naming `S`, so the compiler-side
    // check, which compares a local against the caller's spec, does not see the write; the false
    // claims are made one call up.
    fun via_vector() {
        let c = |x| bump(x);
        hidden(c)
    }
    fun client_vector() { via_vector() }
    spec client_vector {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // Packed, then unpacked and invoked in the same function.
    fun pack_unpack(f: |u64| has copy + drop) {
        let b = Box { f };
        let Box { f: g } = b;
        g(0)
    }
    spec pack_unpack {
        pragma opaque;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun via_pack_unpack() {
        let c = |x| bump(x);
        pack_unpack(c)
    }
    fun client_pack_unpack() { via_pack_unpack() }
    spec client_pack_unpack {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A direct field write is not one of the escapes.
    fun set_field(b: &mut Box, f: |u64| has copy + drop) { b.f = f }

    fun via_set_field(b: &mut Box) { set_field(b, |x| bump(x)) }

    // Escapes: accepted.
    fun new_box(f: |u64| has copy + drop): Box { Box { f } }
    fun new_slot(f: |u64| has copy + drop): Slot { Slot::One { f } }
    fun ret_packed(f: |u64| has copy + drop): Box {
        let b = Box { f };
        b
    }
    fun pack_drop(f: |u64| has copy + drop) { let _b = Box { f }; }
    fun store(s: &signer, f: |u64| has copy + drop + store) { move_to(s, Held { f }) }
    fun set_box(b: &mut Box, f: |u64| has copy + drop) { *b = Box { f } }

    fun via_new_box(): Box { new_box(|x| bump(x)) }
    fun via_new_slot(): Slot { new_slot(|x| bump(x)) }
    fun via_ret_packed(): Box { ret_packed(|x| bump(x)) }
    fun via_pack_drop() { pack_drop(|x| bump(x)) }
    fun via_store(s: &signer) { store(s, bump_stored) }
    fun via_set_box(b: &mut Box) { set_box(b, |x| bump(x)) }
}
