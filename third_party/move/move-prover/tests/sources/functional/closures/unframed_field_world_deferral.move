// An opaque, unverified, memory-free function leaves its closure parameter's frame to the call
// site, which havocs what the closure writes. An unframed field elsewhere in the program brings in
// the world memory; the parameter's predicates still do not range over it, so the call defers.
module 0x42::dw {
    struct R has key { v: u64 }
    struct S has key { v: u64 }
    struct Hold has drop, copy { meta: |&u64| vector<u8> has copy + drop }
    public fun apply<V, T>(v: &mut V, f: |&mut V|T): T { f(v) }
    spec apply {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<f>(v);
        ensures ensures_of<f>(old(v), result, v);
    }

    // True: the closure writes only `R`.
    public fun keeps_other(x: &mut u64): u64 acquires R {
        apply(x, |p: &mut u64| { if (exists<R>(@0x42)) R[@0x42].v = *p; *p })
    }
    spec keeps_other {
        requires exists<S>(@0x42);
        ensures global<S>(@0x42) == old(global<S>(@0x42));
    }

    // False: the closure may write `R`, which the call site havocs.
    public fun keeps_written(x: &mut u64): u64 acquires R {
        apply(x, |p: &mut u64| { if (exists<R>(@0x42)) R[@0x42].v = *p; *p })
    }
    spec keeps_written {
        requires exists<R>(@0x42);
        ensures global<R>(@0x42) == old(global<R>(@0x42));
    }

    public fun keep(h: Hold): Hold { h }
}

// Verex does not encode a frame left to the call site and verifies `keeps_written` (#20966).
// exclude_for: lean
