// A closure over a target encoded from its contract -- a native, or an opaque function under
// `pragma verify = false` -- does what the contract's `modifies` frame says, whatever the
// target's body (if any) writes. Each closure below writes `S` by its target's contract and
// reaches the empty frame of `apply`, and must be rejected.
module 0x42::fun_param_frame_contract_target {
    struct S has key { v: u64 }
    struct G<phantom T> has key { v: u64 }

    native fun native_writer(x: u64);
    spec native_writer {
        pragma opaque;
        modifies global<S>(@0x42);
    }

    fun unverified_writer(_x: u64) {}
    spec unverified_writer {
        pragma opaque;
        pragma verify = false;
        modifies global<S>(@0x42);
    }

    native fun generic_writer<T>(x: u64);
    spec generic_writer {
        pragma opaque;
        modifies global<G<T>>(@0x42);
    }

    fun apply(f: |u64| has drop) { f(1) }
    spec apply {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun over_native() { apply(native_writer) }
    spec over_native {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun over_unverified() { apply(unverified_writer) }
    spec over_unverified {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun over_generic_native() { apply(generic_writer<u64>) }

    // Returned by a transparent factory.
    fun make_native(): |u64| has drop { native_writer }
    fun make_unverified(): |u64| has drop { unverified_writer }

    fun native_from_factory() { apply(make_native()) }
    spec native_from_factory {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun unverified_from_factory() { apply(make_unverified()) }
    spec unverified_from_factory {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A lambda whose body calls such a target: the lambda's own usage summary omits the frame.
    fun native_in_body() { apply(|x| { native_writer(x); native_writer(x + 0) }) }
    spec native_in_body {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun unverified_in_body() { apply(|x| { unverified_writer(x); unverified_writer(x + 0) }) }
    spec unverified_in_body {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }
}
