module 0xc0ffee::m {
    struct S {
        x: u64,
        y: u64
    }

    public fun test() {
        let s = S {x: 3, y: 4};
        let S {x: _, y: _} = s;
    }
}
