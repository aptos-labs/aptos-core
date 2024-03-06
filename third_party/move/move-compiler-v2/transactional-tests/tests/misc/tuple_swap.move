//# publish
module 0xc0ffee::m {
    fun swap1(x: u32, y: u32): (u32, u32) {
        (x, y) = (y, x);
        (x, y)
    }

    fun swap2(a: u64, b: u64, c: u64, d: u64): (u64, u64, u64, u64) {
        (a, b, c, d) = (c, d, b, a);
        (a, b, c, d)
    }

    fun swap3(a: u64, b: u64): (u64, u64) {
        let (a, b) = (b, a);
        (a, b)
    }

    struct W {
        inner: u64,
    }

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

    fun swap6(x: u64, y: u64): (u64, u64) {
        (x, y) = ({y = y + 1; y}, {x = x + 1; x});
        (x, y)
    }

    fun swap7(x: u64, y: u64): (u64, u64) {
        let a = &x;
        let b = &y;
        (a, b) = (b, a);
        (*a, *b)
    }

    public fun test1(): (u32, u32) {
        swap1(1, 2)
    }

    public fun test2(): (u64, u64, u64, u64) {
        swap2(10, 20, 30, 40)
    }

    public fun test3(): (u64, u64) {
        swap3(11, 22)
    }

    public fun test4(): (W, W) {
        swap4(W{inner: 111}, W{inner: 222})
    }

    public fun test6(): (u64, u64) {
        swap6(4, 40)
    }

    public fun test7(): (u64, u64) {
        swap7(44, 440)
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4

//# run 0xc0ffee::m::swap5

//# run 0xc0ffee::m::test6

//# run 0xc0ffee::m::test7
