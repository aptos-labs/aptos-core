// Code a function value nothing bounds runs may call any public function, so it may update a spec
// variable that public code updates. One only private code of its module updates keeps its value.
module 0x42::gc {
    struct Cb has drop, copy { f: || has copy + drop }

    spec module {
        global draws: num;
        global quiet: num;
    }

    public fun draw() {}
    spec draw { update draws = draws + 1; }

    fun bump_quiet() {}
    spec bump_quiet { update quiet = quiet + 1; }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    // False: the callback may call `draw`. Rejected.
    public fun keeps_draws(c: &Cb) { (c.f)() }
    spec keeps_draws { ensures draws == old(draws); }

    // False, the callback run by an opaque callee. Rejected.
    public fun keeps_draws_opaque(c: &Cb) { run(c) }
    spec keeps_draws_opaque { ensures draws == old(draws); }

    // True: only private code updates `quiet`. Verifies.
    public fun keeps_quiet(c: &Cb) { (c.f)() }
    spec keeps_quiet { ensures quiet == old(quiet); }

    // True, the callback run by an opaque callee. Verifies.
    public fun keeps_quiet_opaque(c: &Cb) { run(c) }
    spec keeps_quiet_opaque { ensures quiet == old(quiet); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
