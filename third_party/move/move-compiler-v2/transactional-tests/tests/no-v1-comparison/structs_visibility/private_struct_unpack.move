//# publish
module 0xAA::theirs {
    struct Inner has drop, copy {
        val: u64,
    }

    public fun make_inner(v: u64): Inner {
        Inner { val: v }
    }
}

//# publish
module 0xBB::mine {
    use 0xAA::theirs::Inner;

    enum Wrapper has drop {
        Wrapped { field: Inner },
    }

    public fun make_wrapper(v: u64): Wrapper {
        Wrapper::Wrapped { field: 0xAA::theirs::make_inner(v) }
    }

    public fun steal_via_match(x: Wrapper): u64 {
        match (x) {
            Wrapper::Wrapped { field: Inner { val } } => val,
        }
    }
}
