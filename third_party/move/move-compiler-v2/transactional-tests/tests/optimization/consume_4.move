//# publish
module 0xc0ffee::m {
    fun consume(_x: u32) {}

    public fun test(a: u32) {
        let b = copy a;
        let c = move b;
        consume(c);
        consume(a);
    }

    struct W has copy, drop {
        m: u32
    }

    fun consume_(_x: W) {}

    public fun test_struct(a: W) {
        let b = copy a;
        let c = move b;
        consume_(c);
        consume_(a);
    }

    public fun main() {
        test(42);
        test_struct(W {m: 45});
    }
}

//# run 0xc0ffee::m::main
