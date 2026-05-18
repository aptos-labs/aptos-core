//# publish
module 0xAA::theirs {
    struct Inner has drop, copy {
        val: u64,
    }

    struct Outer has drop, copy {
        inner: Inner,
    }

    public fun make_outer(v: u64): Outer {
        Outer { inner: Inner { val: v } }
    }
}

//# publish
module 0xBB::mine {
    use 0xAA::theirs::Outer;
    use 0xAA::theirs::Inner;

    enum Wrapper has drop {
        Wrapped { field: Outer },
    }

    public fun make_wrapper(v: u64): Wrapper {
        Wrapper::Wrapped { field: 0xAA::theirs::make_outer(v) }
    }

    // Nested match arm pattern unpacks both Outer and Inner from theirs
    public fun steal_via_nested_match(x: Wrapper): u64 {
        match (x) {
            Wrapper::Wrapped { field: Outer { inner: Inner { val } } } => val,
        }
    }
}
