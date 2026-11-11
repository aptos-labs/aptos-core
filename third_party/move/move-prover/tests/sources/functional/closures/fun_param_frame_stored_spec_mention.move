// flag: --language-version=2.4

// A function-typed parameter that the callee only stores is exempt from the call-site frame
// check unless its specification names it. Every spec position counts: here the parameter is
// named only in a `modifies` target or only in a proof, and the writer passed for it is
// rejected. Neither position lets a false claim through (both are single-state), so these are
// conservative rejections.
module 0x42::state_stored_spec_mention {
    struct S has key { v: u64 }

    public fun bump() acquires S { let s = &mut S[@0x42]; s.v = s.v + 1; }
    spec bump {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + 1;
    }
}

module 0x42::fun_param_frame_stored_spec_mention {
    use 0x42::state_stored_spec_mention::bump;

    struct R has key { v: u64 }

    struct H has drop { f: |u64| u64 has copy + drop }

    fun store_modifies(f: |u64| u64 has copy + drop): H acquires R {
        R[@0x42].v = 1;
        H { f }
    }
    spec store_modifies {
        modifies global<R>(if (result_of<f>(0) == 7) @0x42 else @0x42);
    }

    fun store_proof(f: |u64| u64 has copy + drop, x: u64): H {
        let _ = x;
        H { f }
    }
    spec store_proof {
        ensures result.f == result.f;
    } proof {
        assert result_of<f>(x) == result_of<f>(x);
    }

    fun via_modifies(): H acquires R { store_modifies(|_x| { bump(); 0 }) }

    fun via_proof(): H { store_proof(|_x| { bump(); 0 }, 1) }
}
