//# publish
module 0xc0ffee::m {
    fun consume(_a: u32) {}

    public fun test(a: u32) {
        let b = copy a;
        consume(b);
        consume(a);
    }

    struct W has copy, drop {
        a: u32,
    }

    fun consume_(_a: W) {}

    public fun test_(a: W) {
        let b = copy a;
        consume_(b);
        consume_(a);
    }

    public fun main() {
        test(55);
        test_(W{a: 56});
    }
}

//# run 0xc0ffee::m::main
