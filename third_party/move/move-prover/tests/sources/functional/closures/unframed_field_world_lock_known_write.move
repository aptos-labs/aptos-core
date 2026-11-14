// The re-entrancy lock keeps a module's resources across an invocation of a function value
// nothing bounds, because a write of them there aborts. When the value is a function the
// program names, its write is translated: the abort has to be translated with it, or the
// paths on which it wrote vanish and anything is proved about them.
module 0x42::lock_known_write {
    struct R has key { v: u64 }
    struct Cb has drop, copy { f: || has copy + drop }

    fun write_r() acquires R {
        let r = borrow_global_mut<R>(@0x42);
        r.v = if (r.v == 0) 1 else 0;
    }

    // Rejected: the closure re-enters this module and writes `R`, which aborts.
    public fun g() acquires R {
        let c = Cb { f: || write_r() };
        (c.f)();
    }
    spec g {
        aborts_if !exists<R>(@0x42);
    }

    // Verifies: a value of unknown origin leaves `R` as it was when it returns.
    public fun unknown(c: &Cb): u64 acquires R {
        let before = R[@0x42].v;
        (c.f)();
        R[@0x42].v - before
    }
    spec unknown {
        pragma aborts_if_is_partial;
        ensures result == 0;
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
