// Cross-module calls retain their frames for VM reentrancy checks, even when the
// callee's body is trivial.
module 0xc0ffee::n {
    public fun impl(x: u64): u64 {
        x
    }

    public fun fwd(x: u64): u64 {
        impl(x)
    }

    public fun max(a: u64, b: u64): u64 {
        if (a > b) a else b
    }
}

module 0xc0ffee::m {
    use 0xc0ffee::n;

    public fun t_fwd(x: u64): u64 {
        n::fwd(x)
    }

    public fun t_max(x: u64): u64 {
        n::max(x, 7)
    }
}
