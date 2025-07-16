//# publish
module 0xc0ffee::m {
    struct Wrapper(u64);

    public fun apply(f: |Wrapper|u64): u64 {
        let w = Wrapper(42);
        f(w)
    }

    public fun unwrap_maker(): |Wrapper|u64 {
        |Wrapper(x)| x
    }
}

//# publish
module 0xc0ffee::n {
    use 0xc0ffee::m::apply;

    public fun test(): u64 {
        let unwrap = 0xc0ffee::m::unwrap_maker();
        apply(unwrap)
    }
}

//# run 0xc0ffee::n::test
