//# publish
module 0xc0ffee::m {
    struct S {
        f: u32,
        g: u32,
    }

    fun swap1(x: u32, y: u32): (u32, u32) {
        let S { f: x, g: y } = S { f: y, g: x };
        (x, y)
    }

    fun swap2(): (u32, u32) {
        let x = 44;
        let y = 55;
        let S { f: _x, g: _y } = S { f: y, g: x };
        (_x, _y)
    }

    fun test1(): (u32, u32) {
        swap1(1, 2)
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::swap2
