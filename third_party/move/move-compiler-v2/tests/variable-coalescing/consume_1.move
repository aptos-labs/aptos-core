module 0xc0ffee::m {
    fun consume(_x: u64) {}

    public fun test1(x: u64) {
        let y = x;
        consume(x);
        consume(y);
    }

    public fun test2(x: u64) {
        let y = x;
        consume(y);
        consume(x);
    }

    struct W has copy, drop{
        x: u64,
    }

    fun consume_(_x: W) {}

    public fun test3(x: W) {
        let y = x;
        consume_(x);
        consume_(y);
    }

    public fun test4(x: W) {
        let y = x;
        consume_(y);
        consume_(x);
    }
}
