module 0xc0ffee::m {
    fun consume(_x: u32) {}

    public fun test(p: bool, a: u32) {
        let b = copy a;
        if (p) {
            consume(b);
        } else {
            consume(a);
        };
        if (!p) {
            consume(a);
        } else {
            consume(b);
        }
    }

    struct W has copy, drop { x: u32 }

    fun consume_(_x: W) {}

    public fun test_struct(p: bool, a: W) {
        let b = copy a;
        if (p) {
            consume_(b);
        } else {
            consume_(a);
        };
        if (!p) {
            consume_(a);
        } else {
            consume_(b);
        }
    }
}
