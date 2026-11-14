// exclude_for: lean
// A function value that takes a callback gives that callback no frame, so handing it a
// callback that reads a resource is rejected before the result of the call could be bound
// to a carrier that omits what the callback reads.
module 0x42::apply_carrier_unframed_callback {
    struct R has key { w: u64 }

    fun flip() acquires R {
        if (exists<R>(@0x42)) {
            let r = &mut R[@0x42];
            r.w = if (r.w % 2 == 0) 1 else 0
        }
    }
    spec flip { pragma opaque; modifies global<R>(@0x42); aborts_if false; }

    fun read_r(): u64 acquires R { if (exists<R>(@0x42)) R[@0x42].w else 0 }
    spec read_r {
        pragma opaque;
        aborts_if false;
        ensures result == (if (exists<R>(@0x42)) global<R>(@0x42).w else 0);
    }

    // Higher-order targets: their effects omit what the callback they run reads.
    fun run_unsat(f: || u64 has copy + drop): u64 { f() }
    spec run_unsat { pragma opaque; requires false; aborts_if false; }

    fun run(f: || u64 has copy + drop): u64 { f() }
    spec run { pragma opaque, verify = false; aborts_if false; }

    // Rejected at both calls of `h`.
    fun callback_across_write_unsat(): (u64, u64) acquires R {
        let h: |(|| u64 has copy + drop)| u64 has copy + drop = run_unsat;
        let a = h(read_r);
        flip();
        let b = h(read_r);
        (a, b)
    }
    spec callback_across_write_unsat {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2;
    }

    fun callback_across_write(): (u64, u64) acquires R {
        let h: |(|| u64 has copy + drop)| u64 has copy + drop = run;
        let a = h(read_r);
        flip();
        let b = h(read_r);
        (a, b)
    }
    spec callback_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2;
    }
}
