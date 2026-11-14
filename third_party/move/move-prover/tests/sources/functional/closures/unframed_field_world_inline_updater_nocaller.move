// Probe (d): a ghost updated only by a public inline function nothing in the program inlines.
// Unknown code may inline it (the commit message counts inline functions as reachable updaters).
// Expected (per the stated design): keeps_body and keeps_fspec REJECTED.
module 0x42::gm {
    struct Cb has drop, copy { f: || has copy + drop }
    spec module { global inl: num; global inl2: num; }

    public inline fun bump_inl() { spec { update inl = inl + 1; }; }
    public inline fun bump_inl2() { }
    spec bump_inl2 { update inl2 = inl2 + 1; }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    public fun keeps_body(c: &Cb) { run(c) }
    spec keeps_body { ensures inl == old(inl); }

    public fun keeps_fspec(c: &Cb) { run(c) }
    spec keeps_fspec { ensures inl2 == old(inl2); }

    // Vacuity control: rejected.
    public fun keeps_false(c: &Cb) { run(c) }
    spec keeps_false { ensures false; }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
