// TODO: #18199
module 0xc0ffee::m {
    public enum Wrapper has drop {
        V1(u64, u64),
        V2(u64),
    }

    public fun make(x: u64): Wrapper {
        Wrapper::V1(x, x + 1)
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m;
    use 0xc0ffee::m::Wrapper;

    fun test_1() {
        let x = m::make(22);
        let x_0 = &mut x.0;
        let x_1 = &mut x.1;
        foo(x_0, x_1);
    }

    fun foo(_x: &mut u64, _y: &mut u64) {

    }

    fun test_2() {
        let x = m::make(22);
        let x_0 = &mut x.0;
        let x_1 = &mut x.1;
        *x_0 = 23;
        *x_1 = 24;
    }

    fun test_no_err() {
        let x = m::make(22);
        x.0 = 23;
        x.1 = 24;
    }

    fun test_match() {
        let x = m::make(22);
        match (&mut x) {
            Wrapper::V2(_) => {

            }
            Wrapper::V1(x_0, x_1) => {
                foo(x_0, x_1);
            }
        }
    }

}
