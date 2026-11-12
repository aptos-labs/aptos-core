// While a module's code runs, the VM rejects every access to the module's resources from code
// reached through a function value or a call into another module: that code can reach the
// module only as a re-entry, which locks the module's resources. So such a call, if it returns,
// leaves them unchanged, however unknown the code it runs. Nothing more is claimed: other
// modules' resources, those of the callee's module, and those of a module declaring a native
// (which may access them without the check) may change.
//
// Each function states its required verdict. A false claim verifying here is unsound.
module 0x47::ext {
    struct Other has key { v: u64 }
    public fun set(v: u64) acquires Other { if (exists<Other>(@0x47)) Other[@0x47].v = v; }
}

module 0x45::disp {
    struct Hook has key { f: |u64| u64 has copy + drop + store }
    struct State has key { v: u64 }
    public fun set_state(v: u64) acquires State { State[@0x45].v = v; }

    // Runs a function value read from storage.
    #[module_lock]
    fun run_hook(x: u64): u64 acquires Hook { let f = Hook[@0x45].f; f(x) }
    spec run_hook { pragma opaque; }

    public fun dispatched(x: u64): u64 acquires Hook { run_hook(x) }

    public fun bump_then_dispatch(r: &mut u64, x: u64): u64 acquires Hook {
        *r = *r + 1;
        run_hook(x)
    }
}

module 0x46::coinish {
    use 0x45::disp;
    use 0x47::ext;
    struct Info has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // Own resource across the dispatch. Verifies.
    public fun check(x: u64): u64 { disp::dispatched(x) }
    spec check {
        ensures exists<Info>(@0x46) == old(exists<Info>(@0x46));
        ensures global<Info>(@0x46) == old(global<Info>(@0x46));
    }

    // Another module's resource, which the hook may write. Rejected.
    public fun check_other(x: u64): u64 { disp::dispatched(x) }
    spec check_other { ensures global<ext::Other>(@0x47) == old(global<ext::Other>(@0x47)); }

    // The dispatching module's resource: locked too at runtime, but not claimed. Rejected.
    public fun check_disp(x: u64): u64 { disp::dispatched(x) }
    spec check_disp { ensures global<disp::State>(@0x45) == old(global<disp::State>(@0x45)); }

    // Vacuity control. Rejected.
    public fun check_false(x: u64): u64 { disp::dispatched(x) }
    spec check_false { ensures false; }

    // The dispatch may abort. Rejected.
    public fun check_aborts(x: u64): u64 { disp::dispatched(x) }
    spec check_aborts { aborts_if false; }

    // A write before the dispatch survives it. Verifies.
    public fun write_then_check(x: u64): u64 acquires Info {
        if (exists<Info>(@0x46)) Info[@0x46].v = 7;
        disp::dispatched(x)
    }
    spec write_then_check { ensures exists<Info>(@0x46) ==> global<Info>(@0x46).v == 7; }

    // A write through a `&mut` argument is the caller's, and is kept. Verifies.
    public fun bump_via(x: u64): u64 acquires Info {
        disp::bump_then_dispatch(&mut Info[@0x46].v, x)
    }
    spec bump_via {
        requires exists<Info>(@0x46) && global<Info>(@0x46).v < 100;
        ensures global<Info>(@0x46).v == old(global<Info>(@0x46).v) + 1;
    }

    // Same, claiming the write is lost. Rejected.
    public fun bump_via_false(x: u64): u64 acquires Info {
        disp::bump_then_dispatch(&mut Info[@0x46].v, x)
    }
    spec bump_via_false {
        requires exists<Info>(@0x46) && global<Info>(@0x46).v < 100;
        ensures global<Info>(@0x46).v == old(global<Info>(@0x46).v);
    }

    // A field callback, then the dispatch. Verifies.
    public fun invoke_then_dispatch(c: &Cb, x: u64): u64 {
        let _a = (c.f)(x);
        disp::dispatched(x)
    }
    spec invoke_then_dispatch { ensures global<Info>(@0x46) == old(global<Info>(@0x46)); }

    fun run(c: &Cb, x: u64): u64 { (c.f)(x) }

    // The callback runs in a transparent function of this module. Verifies.
    public fun via_local_wrapper(c: &Cb, x: u64): u64 { run(c, x) }
    spec via_local_wrapper { ensures global<Info>(@0x46) == old(global<Info>(@0x46)); }
}

// A module declaring a native gets no such guarantee. Rejected.
module 0x48::nat {
    struct Own has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    native fun tick(): u64;
    public fun keeps_own(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keeps_own { ensures global<Own>(@0x48) == old(global<Own>(@0x48)); }
}

// An opaque callee of the same module may write the module's resources itself, so nothing is
// claimed across it. Rejected (true here, a precision loss).
module 0x49::selfcall {
    struct Own has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    public fun set_own(v: u64) acquires Own { Own[@0x49].v = v; }
    fun inner(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec inner { pragma opaque; }
    public fun keeps_own_via_inner(c: &Cb, x: u64): u64 { inner(c, x) }
    spec keeps_own_via_inner { ensures global<Own>(@0x49) == old(global<Own>(@0x49)); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
