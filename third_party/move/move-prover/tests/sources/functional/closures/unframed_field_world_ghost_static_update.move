// A spec variable is kept across unknown code when every updater that code can reach must
// access a resource of the module on the stack: re-entry aborts first. An opaque callee of
// the same module that calls such an updater itself is no re-entry, so its summary does not
// keep the variable.
module 0x42::ghost_static_update {
    struct R has key { v: u64 }
    struct Cb has drop, copy { f: || has copy + drop }
    spec module { global g: num; global d: num; }

    public fun bump() {
        assert!(exists<R>(@0x42), 1);
    }
    spec bump { update g = g + 1; }

    public fun draw() {}
    spec draw { update d = d + 1; }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    // Verifies: the callback reaching `bump` aborts.
    public fun keeps_g(c: &Cb) { run(c) }
    spec keeps_g { ensures g == old(g); }

    // Rejected: `draw` accesses no resource.
    public fun keeps_d(c: &Cb) { run(c) }
    spec keeps_d { ensures d == old(d); }

    fun run_bump(c: &Cb) { (c.f)(); bump() }
    spec run_bump { pragma opaque; }

    // Rejected: `run_bump` calls `bump` itself.
    public fun keeps_g_static(c: &Cb) { run_bump(c) }
    spec keeps_g_static { ensures g == old(g); }

    fun run_draw(c: &Cb) { (c.f)(); draw() }
    spec run_draw { pragma opaque; }

    // Rejected.
    public fun keeps_d_static(c: &Cb) { run_draw(c) }
    spec keeps_d_static { ensures d == old(d); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
