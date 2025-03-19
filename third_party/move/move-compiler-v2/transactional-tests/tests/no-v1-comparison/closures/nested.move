//# publish
module 0x42::test {

    public fun nested(x: u64): u64 {
        let f = |y| x + y;
        let h = |g, x| g(x);
        h(f, 1)
    }

    public fun nested_capture(x: u64): u64 {
        let f = |y| x + y;
        let h = |x| f(x);
        h(1)
    }
}

//# run 0x42::test::nested --args 3

//# run 0x42::test::nested_capture --args 4
