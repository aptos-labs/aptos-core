// Inlining `m::helper` into `m::locked` preserves the caller's module lock.
// The helper invokes a callback that reenters `m` through `n`; the lock rejects this
// reentry with or without inlining. All test configurations must abort.
//# publish
module 0x42::m {
    fun helper(f: |u64|u64): u64 { f(1) }
    #[module_lock]
    public fun locked(f: |u64|u64): u64 { helper(f) }
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
    public fun go(): u64 { m::locked(|x| n::reenter(x)) }
}

//# run 0x42::e::go
