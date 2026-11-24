// Probe (d): ghost updaters that the reachability might miss.
// (1) an update in an inline function's body spec block (on_impl), resource-free: unknown code can
//     run it (inlined into its own code). Expected: keeps_inl REJECTED.
// (2) an updater reached only through a closure a dependency builds over a private function of gm
//     exposed through a public getter. Expected: keeps_dep REJECTED.
module 0x42::gm {
    struct R has key { v: u64 }
    struct Cb has drop, copy { f: || has copy + drop }
    spec module { global inl: num; global dep: num; }

    public inline fun bump_inl() { spec { update inl = inl + 1; }; }

    fun bump_dep_private() {}
    spec bump_dep_private { update dep = dep + 1; }
    public fun handout(): || has copy + drop { || bump_dep_private() }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    public fun keeps_inl(c: &Cb) { run(c) }
    spec keeps_inl { ensures inl == old(inl); }

    public fun keeps_dep(c: &Cb) { run(c) }
    spec keeps_dep { ensures dep == old(dep); }
}
module 0x42::gm_ctl {
    use 0x42::gm;
    // Control: a direct call of the inline updater. Expected REJECTED (the update is real here).
    public fun direct() { gm::bump_inl() }
    spec direct { ensures gm::inl == old(gm::inl); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
