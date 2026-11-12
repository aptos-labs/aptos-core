// Code run by a function value nothing bounds may change any resource, but it cannot remove
// one whose module never moves it from storage: only the defining module can `move_from` it,
// and a native of the module could bypass that. So across such code, a resource of a module
// with no `move_from` of it and no native still exists where it existed; its value may change.
//
// Each function states its required verdict. A false claim verifying here is unsound.
module 0x50::keep {
    struct R has key { v: u64 }
    public fun make(s: &signer) { move_to(s, R { v: 0 }) }
    public fun set(v: u64) acquires R { if (exists<R>(@0x50)) R[@0x50].v = v; }
}

module 0x51::rem {
    struct R has key { v: u64 }
    public fun make(s: &signer) { move_to(s, R { v: 0 }) }
    public fun take(): u64 acquires R { let R { v } = move_from<R>(@0x51); v }
}

module 0x52::nat {
    struct R has key { v: u64 }
    native fun tick(): u64;
    public fun make(s: &signer) { move_to(s, R { v: 0 }) }
}

// Runs a function value read from storage, opaque.
module 0x53::hook {
    struct Hook has key { f: |u64| u64 has copy + drop + store }
    fun run_hook(x: u64): u64 acquires Hook { let f = Hook[@0x53].f; f(x) }
    spec run_hook { pragma opaque; }
    public fun dispatched(x: u64): u64 acquires Hook { run_hook(x) }
}

module 0x54::user {
    use 0x50::keep;
    use 0x51::rem;
    use 0x52::nat;
    use 0x53::hook;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // (a) No module removes `keep::R`: it still exists after the invocation. Verifies.
    public fun keep_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keep_invoke {
        requires exists<keep::R>(@0x50);
        ensures exists<keep::R>(@0x50);
    }

    // (a) Same, across an opaque call that runs a stored function value. Verifies.
    public fun keep_dispatch(x: u64): u64 { hook::dispatched(x) }
    spec keep_dispatch {
        requires exists<keep::R>(@0x50);
        ensures exists<keep::R>(@0x50);
    }

    // (a) Same, at the head of a loop invoking the field. Verifies.
    public fun keep_loop(c: &Cb, n: u64): u64 {
        let i = 0;
        let s = 0;
        while (i < n) {
            s = (c.f)(i);
            i = i + 1;
        } spec {
            invariant exists<keep::R>(@0x50);
        };
        s
    }
    spec keep_loop {
        requires exists<keep::R>(@0x50);
        ensures exists<keep::R>(@0x50);
    }

    // The value of `keep::R` may change (`keep::set`). Rejected.
    public fun keep_value(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keep_value {
        requires exists<keep::R>(@0x50);
        ensures global<keep::R>(@0x50) == old(global<keep::R>(@0x50));
    }

    // Existence is not created either: absence is not kept. Rejected.
    public fun keep_absent(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keep_absent {
        requires !exists<keep::R>(@0x50);
        ensures !exists<keep::R>(@0x50);
    }

    // Vacuity control. Rejected.
    public fun keep_false(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keep_false {
        requires exists<keep::R>(@0x50);
        ensures false;
    }

    // (b) `rem::take` removes `rem::R`, and the callback may call it. Rejected.
    public fun rem_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec rem_invoke {
        requires exists<rem::R>(@0x51);
        ensures exists<rem::R>(@0x51);
    }

    // (b) Same across the opaque dispatch. Rejected.
    public fun rem_dispatch(x: u64): u64 { hook::dispatched(x) }
    spec rem_dispatch {
        requires exists<rem::R>(@0x51);
        ensures exists<rem::R>(@0x51);
    }

    // (c) `nat` declares a native, which may access its storage. Rejected.
    public fun nat_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec nat_invoke {
        requires exists<nat::R>(@0x52);
        ensures exists<nat::R>(@0x52);
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
