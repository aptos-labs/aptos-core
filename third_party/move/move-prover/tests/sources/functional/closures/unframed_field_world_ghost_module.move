// While a function of a module runs, code a function value runs may still call the module's
// functions, but re-entry aborts at any access to the module's resources. A spec variable every
// updater of which accesses such a resource on every path keeps its value across unknown code in
// the module's own functions; elsewhere, or with an updater free of resource accesses, it does not.
module 0x42::gm {
    struct R has key { v: u64 }
    struct Cb has drop, copy { f: || has copy + drop }

    spec module {
        global locked: num;
        global free: num;
    }

    public fun bump_locked() acquires R { R[@0x42].v = R[@0x42].v + 1; }
    spec bump_locked { pragma aborts_if_is_partial; update locked = locked + 1; }

    public fun bump_free() {}
    spec bump_free { update free = free + 1; }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    // True: re-entry aborts `bump_locked`. Verifies.
    public fun keeps_locked(c: &Cb) { run(c) }
    spec keeps_locked { ensures locked == old(locked); }

    // False: the callback may call `bump_free`. Rejected.
    public fun keeps_free(c: &Cb) { run(c) }
    spec keeps_free { ensures free == old(free); }
}

module 0x43::other {
    struct Cb has drop, copy { f: || has copy + drop }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    // False: outside `gm`, the callback may call `gm::bump_locked`. Rejected.
    public fun keeps_locked_elsewhere(c: &Cb) { run(c) }
    spec keeps_locked_elsewhere { ensures 0x42::gm::locked == old(0x42::gm::locked); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
