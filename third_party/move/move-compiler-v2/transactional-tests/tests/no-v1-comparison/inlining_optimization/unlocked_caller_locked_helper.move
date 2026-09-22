// `m::go` statically calls the `#[module_lock]` helper `m::locked_helper`, whose callback
// reenters `m` through `n`. Inlining `locked_helper` into `go` would drop the lock and allow
// the reentry. All test configurations must abort.
//# publish
module 0x42::m {
    #[module_lock]
    fun locked_helper(f: |u64|u64): u64 { f(1) }
    public fun go(f: |u64|u64): u64 { locked_helper(f) }
    public fun touch(x: u64): u64 { x }
}

//# publish
module 0x42::n {
    use 0x42::m;
    public fun reenter(x: u64): u64 { m::touch(x) }
}

//# publish
module 0x42::e {
    use 0x42::m;
    use 0x42::n;
    public fun go(): u64 { m::go(|x| n::reenter(x)) }
}

//# run 0x42::e::go
