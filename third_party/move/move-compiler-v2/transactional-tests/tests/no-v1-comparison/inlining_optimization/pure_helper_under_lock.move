// `b::dispatch` holds a module lock while its callback calls `a::hook`, which reenters
// `b::pure`. Inlining `b::pure` into `a::hook` would remove the reentry check and allow
// the call. All test configurations must abort.
//# publish
module 0x42::b {
    public fun pure(x: u64, y: u64): u64 { x + y }
    #[module_lock]
    public fun dispatch(f: |u64|u64): u64 { f(1) }
}

//# publish
module 0x42::a {
    use 0x42::b;
    public fun hook(x: u64): u64 { b::pure(x, 2) }
}

//# publish
module 0x42::e {
    use 0x42::a;
    use 0x42::b;
    public fun go(): u64 { b::dispatch(|x| a::hook(x)) }
}

//# run 0x42::e::go
