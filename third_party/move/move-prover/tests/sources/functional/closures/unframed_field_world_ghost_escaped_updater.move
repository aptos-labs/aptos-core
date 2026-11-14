// A private function packed into a stored closure escapes to code that can invoke it, even when
// the function that packs it also calls it directly: it updates its spec variable for that code.
module 0x42::eu {
    struct Hold has key { f: || has copy + drop + store }

    spec module {
        global g: num;
    }

    #[persistent]
    fun upd() {}
    spec upd { update g = g + 1; }

    entry fun init(s: &signer) {
        upd();
        move_to(s, Hold { f: upd });
    }

    public fun get(): || has copy + drop + store acquires Hold { Hold[@0x42].f }
}

// The same updater, never packed: only private code reaches it.
module 0x42::ek {
    spec module {
        global k: num;
    }

    fun upd() {}
    spec upd { update k = k + 1; }

    entry fun init() { upd() }
}

module 0x43::user {
    struct Cb has drop, copy { f: || has copy + drop }

    fun run(c: &Cb) { (c.f)() }
    spec run { pragma opaque; }

    // False: the callback may invoke `eu::get()`'s closure. Rejected.
    public fun keeps_escaped(c: &Cb) { run(c) }
    spec keeps_escaped { ensures 0x42::eu::g == old(0x42::eu::g); }

    // True: nothing outside `ek` reaches its updater. Verifies.
    public fun keeps_private(c: &Cb) { run(c) }
    spec keeps_private { ensures 0x42::ek::k == old(0x42::ek::k); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
