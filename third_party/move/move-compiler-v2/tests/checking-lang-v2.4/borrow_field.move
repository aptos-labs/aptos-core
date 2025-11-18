module 0xc0ffee::n {
    enum Wrapper has drop {
        V1(u64, u64),
        V2(u8),
    }

    enum Wrapper2 has drop {
        V1 {x: u64, y: u64},
        V2 {z: u8, x: u8},
    }

    fun make(x: u64): Wrapper2 {
        Wrapper2::V1 {x: x, y: x + 1}
    }

    fun test_1() {
        let x = make(22);
        let x_0 = &mut x.x;
        let x_1 = &mut x.y;
        foo(x_0, x_1);
    }

    fun foo(_x: &mut u64, _y: &mut u64) {

    }

}
