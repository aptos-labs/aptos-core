// flag: --language-version=2.4

// A lemma has no compiled definition. The scan for modules that never remove a resource skips it,
// so a resource of a module declaring one still exists after code nothing bounds runs.
module 0x50::keep {
    struct R has key { v: u64 }
    public fun make(s: &signer) { move_to(s, R { v: 0 }) }
    public fun set(v: u64) acquires R { if (exists<R>(@0x50)) R[@0x50].v = v; }

    spec module {
        fun double(x: num): num { x + x }
        lemma double_nonneg(x: num) {
            ensures x >= 0 ==> double(x) >= 0;
        }
    }
}

module 0x54::user {
    use 0x50::keep;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // Nothing removes `keep::R`: it still exists after the invocation. Verifies.
    public fun keep_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keep_invoke {
        requires exists<keep::R>(@0x50);
        ensures exists<keep::R>(@0x50);
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
