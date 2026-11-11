// Function-typed parameters that the callee only packs, returns or drops: their frames are not
// load-bearing for the callee, so effectful closures are accepted.
// Expected: `pack_all`, `ret_one`, `drop_one` call sites accepted (no error);
// `push_one`: ERR (C: a vector is not one of the escapes);
// `pack_and_invoke`: ERR for `f1` only (line of the call), `f2` accepted;
// `via_forwarder`: ERR (forwarded to `apply`, which invokes it);
// `via_capture`: ERR (captured for `apply`'s invoked parameter);
// `pack_spec`: ERR (C: the parameter is named in the callee's spec).
module 0x42::state_stored {
    struct S has key { v: u64 }
    public fun init(s: &signer) { move_to(s, S { v: 0 }) }
    public fun get(): u64 acquires S { S[@0x42].v }
    public fun bump() acquires S { let s = &mut S[@0x42]; s.v = s.v + 1; }
    spec bump {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + 1;
    }
}

module 0x42::fun_param_frame_stored {
    use 0x42::state_stored as st;

    struct Cbs has drop { f1: |u64| has copy + drop, f2: |u64| has copy + drop }

    fun new_cbs(f1: |u64| has copy + drop, f2: |u64| has copy + drop): Cbs { Cbs { f1, f2 } }
    fun new_cbs_invoking(f1: |u64| has copy + drop, f2: |u64| has copy + drop): Cbs {
        f1(0);
        Cbs { f1, f2 }
    }
    fun push(f: |u64| has copy + drop): vector<|u64| has copy + drop> { vector[f] }
    fun ret(f: |u64| has copy + drop): |u64| has copy + drop { f }
    fun drop_it(_f: |u64| has copy + drop) {}
    fun apply(f: |u64| has copy + drop) { f(1) }
    fun fwd(f: |u64| has copy + drop) { apply(f) }
    fun cap(f: |u64| has copy + drop): |u64| has copy + drop { |x| apply2(f, x) }
    fun apply2(f: |u64| has copy + drop, x: u64) { f(x) }
    fun new_cbs_spec(f1: |u64| has copy + drop, f2: |u64| has copy + drop): Cbs { Cbs { f1, f2 } }
    spec new_cbs_spec { ensures result.f1 == f1; }

    fun pack_all(): Cbs { new_cbs(|_x| st::bump(), |_x| st::bump()) }
    fun push_one(): vector<|u64| has copy + drop> { push(|_x| st::bump()) }
    fun ret_one(): |u64| has copy + drop { ret(|_x| st::bump()) }
    fun drop_one() { drop_it(|_x| st::bump()) }
    fun pack_and_invoke(): Cbs {
        new_cbs_invoking(|_x| st::bump(), |_x| st::bump())
    }
    fun via_forwarder() { fwd(|_x| st::bump()) }
    fun via_capture(): |u64| has copy + drop { cap(|_x| st::bump()) }
    fun pack_spec(): Cbs { new_cbs_spec(|_x| st::bump(), |_x| ()) }
}
