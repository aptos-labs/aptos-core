// Existence of a generic resource across unknown code: a module with no `move_from` of the
// resource in any instantiation keeps every instantiation in place; one `move_from` of one
// instantiation disqualifies the resource in all of them. Each function states its required
// verdict.
module 0x50::keep {
    struct R<phantom T> has key { v: u64 }
    public fun make(s: &signer) { move_to(s, R<u64> { v: 0 }); move_to(s, R<bool> { v: 0 }); }
    public fun set<T>(v: u64) acquires R { if (exists<R<T>>(@0x50)) R<T>[@0x50].v = v; }
    public fun has<T>(): bool { exists<R<T>>(@0x50) }
}

module 0x51::rem {
    struct R<phantom T> has key { v: u64 }
    public fun make(s: &signer) { move_to(s, R<u64> { v: 0 }); move_to(s, R<bool> { v: 0 }); }
    public fun take(): u64 acquires R { let R { v } = move_from<R<bool>>(@0x51); v }
    public fun has<T>(): bool { exists<R<T>>(@0x51) }
}

module 0x54::user {
    use 0x50::keep;
    use 0x51::rem;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // (a) Nothing removes `keep::R` in any instantiation. Verifies.
    public fun kept<T>(c: &Cb): bool {
        (c.f)(0);
        keep::has<T>()
    }
    spec kept {
        requires exists<keep::R<T>>(@0x50);
        ensures result;
    }

    // (b) The named instantiation too. Verifies.
    public fun kept_u64(c: &Cb): bool {
        (c.f)(0);
        keep::has<u64>()
    }
    spec kept_u64 {
        requires exists<keep::R<u64>>(@0x50);
        ensures result;
    }

    // (c) `rem` removes `R<bool>`; `R<u64>` is not kept either. Rejected.
    public fun removed_u64(c: &Cb): bool {
        (c.f)(0);
        rem::has<u64>()
    }
    spec removed_u64 {
        requires exists<rem::R<u64>>(@0x51);
        ensures result;
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
