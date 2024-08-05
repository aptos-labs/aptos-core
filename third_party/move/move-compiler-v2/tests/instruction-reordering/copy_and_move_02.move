module 0xc0ffee::m {
    struct S has copy {x: u64}

    fun consume(x: S, y: S) {
        let S{x: _} = x;
        let S{x: _} = y;
    }

    public fun test(x: S) {
        let y = copy x;
        let z = move x;
        consume(z, y);
    }
}
