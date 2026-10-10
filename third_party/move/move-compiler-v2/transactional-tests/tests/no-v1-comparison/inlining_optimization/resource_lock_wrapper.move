// `b::f` reads `b`'s state, invokes a callback that reenters `b::set`, then writes back
// using the earlier value. Inlining `b::f` into `a::go` would reduce `b`'s active count
// to one in `b::set`, allowing the callback's write to be overwritten by the stale update.
// All test configurations must reject the reentrant write without `#[module_lock]`.
//# publish
module 0x42::b {
    struct S has key { v: u64 }
    fun init(s: &signer) { move_to(s, S { v: 0 }) }
    public fun get(): u64 acquires S { S[@0x42].v }
    public fun set(v: u64) acquires S { S[@0x42].v = v }
    public fun f(cb: |u64|) {
        let before = get();
        cb(100);
        set(before + 1)
    }
}

//# publish
module 0x42::c {
    use 0x42::b;
    public fun reenter(x: u64) { b::set(x) }
}

//# publish
module 0x42::a {
    use 0x42::b;
    use 0x42::c;
    public fun go() { b::f(|x| c::reenter(x)) }
    public fun check(): u64 { b::get() }
}

//# run 0x42::b::init --signers 0x42

//# run 0x42::a::go

//# run 0x42::a::check
