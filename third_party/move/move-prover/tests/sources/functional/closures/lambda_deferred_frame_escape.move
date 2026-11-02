// A callee that leaves a closure parameter's frame to the call site is accounted for by
// havocing what the closure writes across the call, which is only sound if the closure runs
// during the call. A callee whose body stores the closure, here into a field whose frame
// allows only D, lets it run later, after the havoc; the claim that C is unchanged across
// that later run would verify. Such a callee does not leave the frame to the call site, so
// the closure is checked against the empty frame and rejected.
module 0x42::lambda_deferred_frame_escape {
    struct C has key { n: u64 }
    struct D has key { n: u64 }
    struct Holder has key { f: |address| has store + drop + copy }
    spec Holder {
        modifies_of<f>(a: address) D[a];
    }

    public fun bump(a: address) acquires C {
        C[a].n = C[a].n + 1;
    }
    spec bump {
        pragma opaque;
        modifies global<C>(a);
        aborts_if !exists<C>(a);
        aborts_if global<C>(a).n + 1 > MAX_U64;
        ensures global<C>(a).n == old(global<C>(a).n) + 1;
    }

    // Unverified and opaque, but stores f instead of invoking it.
    fun stash(f: |address| has store + drop + copy) acquires Holder {
        Holder[@0x42].f = f;
    }
    spec stash {
        pragma opaque;
        pragma verify = false;
    }

    fun run_t(a: address) acquires Holder {
        let f = Holder[@0x42].f;
        f(a)
    }

    fun caller_t(a: address): bool acquires Holder, C {
        stash(bump);
        let b = C[a].n;
        run_t(a);
        C[a].n == b
    }
    spec caller_t {
        pragma aborts_if_is_partial;
        ensures result;
    }
}
