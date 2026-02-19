//# publish
module 0xc0ffee::m {
    public fun test(x: i64, y: i64): i64 {
        if (x == -1) {
            -12 + y
        } else {
            8
        }
    }
}

//# run 0xc0ffee::m::test --args -1i64 -12i64

//# run 0xc0ffee::m::test --args 0i64 55i64
