// An inline function of gm calls an opaque callee of another module that runs an unframed
// callback. Inlined into a client, gm is not on the stack, so the callback's call to
// gm::bump_locked succeeds and `locked` grows: the inline function's claim must be rejected.
module 0x43::other {
    struct Cb has drop, copy { f: || has copy + drop }
    public fun make(f: || has copy + drop): Cb { Cb { f } }
    spec make { aborts_if false; }
    public fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }
}
module 0x42::gm {
    use 0x43::other;
    struct R has key { v: u64 }
    spec module { global locked: num; }
    public fun bump_locked() acquires R { R[@0x42].v = R[@0x42].v + 1; }
    spec bump_locked { pragma aborts_if_is_partial; update locked = locked + 1; }

    // Verifies: gm is on the stack, so the callback cannot re-enter bump_locked.
    public fun keeps(c: &other::Cb) { other::run(c) }
    spec keeps { ensures locked == old(locked); }

    // Rejected: false once inlined outside gm.
    public inline fun keeps_inline(c: &other::Cb) { other::run(c) }
    spec keeps_inline { pragma opaque; ensures locked == old(locked); }

    // Rejected: transparent, verified on its own.
    public inline fun keeps_inline_t(c: &other::Cb) { other::run(c) }
    spec keeps_inline_t { ensures locked == old(locked); }

    // Rejected: the inline function is verified at all.
    public inline fun keeps_inline_false(c: &other::Cb) { other::run(c) }
    spec keeps_inline_false { pragma opaque; ensures false; }
}
module 0x44::client {
    use 0x42::gm;
    use 0x43::other;
    // False at runtime (locked + 1); verifies only through keeps_inline's summary.
    fun witness() {
        let c = other::make(|| gm::bump_locked());
        gm::keeps_inline(&c)
    }
    spec witness { ensures gm::locked == old(gm::locked); }
}
// A ghost whose updater touches no resource is not kept by its module being on the stack.
module 0x45::gf {
    use 0x43::other;
    spec module { global free: num; }
    public fun bump_free() {}
    spec bump_free { pragma aborts_if_is_partial; update free = free + 1; }

    // Rejected.
    public fun keeps(c: &other::Cb) { other::run(c) }
    spec keeps { ensures free == old(free); }

    // Rejected.
    public inline fun keeps_inline(c: &other::Cb) { other::run(c) }
    spec keeps_inline { pragma opaque; ensures free == old(free); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
