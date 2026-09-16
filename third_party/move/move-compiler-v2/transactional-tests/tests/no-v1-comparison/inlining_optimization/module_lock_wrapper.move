// `b::run` invokes a callback that enters `c::locked_cb` and reenters `b` under a module
// lock. Inlining `b::run` into `a::go` would remove `b` from the active modules and allow
// the reentry. All test configurations must abort.
//# publish
module 0x42::b {
    public fun run(f: |u64|) { f(1) }
    public fun touch(): u64 { 7 }
}

//# publish
module 0x42::c {
    use 0x42::b;
    #[module_lock]
    public fun locked_cb(_x: u64) { b::touch(); }
}

//# publish
module 0x42::a {
    use 0x42::b;
    use 0x42::c;
    public fun go() { b::run(|x| c::locked_cb(x)) }
}

//# run 0x42::a::go
