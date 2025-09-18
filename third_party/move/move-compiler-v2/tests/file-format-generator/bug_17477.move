module 0xc0ffee::m {
    struct S has drop {
        x: u64
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m::S;

    public fun test1() {
        spec {};
        let _ = S { x: 42 };
    }

    public fun test2(s: S) {
        spec { assert s.x > 0; };
    }
}
