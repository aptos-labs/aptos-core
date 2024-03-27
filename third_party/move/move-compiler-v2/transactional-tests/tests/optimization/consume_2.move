//# publish
module 0xc0ffee::m {
    fun consume(_x: u64) {}

    public fun test1(x: u64) {
        let y = move x;
        consume(y);
        consume(y);
    }

    struct W has copy, drop {
        x: u64,
    }

    fun consume_(_x: W) {}

    public fun test2(x: W) {
        let y = move x;
        consume_(y);
        consume_(y);
    }

    public fun main() {
        test1(42);
        test2(W{ x: 42 });
    }
}

//# run 0xc0ffee::m::main
