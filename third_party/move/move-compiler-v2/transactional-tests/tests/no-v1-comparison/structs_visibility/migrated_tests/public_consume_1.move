//# publish
module 0xc0ffee::m {

    public struct W has copy, drop {
        x: u64,
    }
}

//# publish
module 0xc0ffee::test_m {
    use 0xc0ffee::m::W;

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

    public fun test5() {
        let x = W { x: 42 };
        let y = copy x;
        test3(x);
        test4(y);
    }
}

//# run 0xc0ffee::test_m::test1 --args 42

//# run 0xc0ffee::test_m::test2 --args 42

//# run 0xc0ffee::test_m::test5
