module 0xc0ffee::m {
    #[lint::skip(while_true, blocks_in_conditions)]
    public fun test1(x: u64) {
        if ({let y = x + 1; y < 5}) {
            // do nothing
        };
        while (true) {
            // do nothing
        }
    }

    public fun test2(x: u64) {
        if ({let y = x + 1; y < 5}) {
            // do nothing
        };
        while (true) {
            // do nothing
        }
    }
}
