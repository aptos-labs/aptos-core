//# publish
module 0xc0ffee::m {

    public struct W {
        inner: u64,
    }

}

//# publish
module 0xc0ffee::test_m {
    use 0xc0ffee::m::W;

    fun swap4(a: W, b: W): (W, W) {
        (a, b) = (b, a);
        (a, b)
    }

    fun swap5(): (u64, u64) {
        let x = 147;
        let y = 258;
        (W {inner: x}, W {inner: y}) = (W {inner: y}, W {inner: x});
        (x, y)
    }

    public fun test4(): (W, W) {
        swap4(W{inner: 111}, W{inner: 222})
    }

}

//# run 0xc0ffee::test_m::test4

//# run 0xc0ffee::test_m::swap5
