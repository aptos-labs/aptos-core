module 0xc0ffee::m {
    enum Wrapper has drop {
        V1(u64),
        V2(u64),
    }

    public fun make(x: u64): Wrapper {
        Wrapper::V1(x)
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m;

    fun test1() {
        let x = m::make(22);
        match (x) {
            m::Wrapper::V1(v) => assert!(v == 22, 1),
            _ => abort(0),
        };
    }

    fun test2() {
        let x = m::make(22);
        match (&x) {
            m::Wrapper::V2(v) => assert!(*v == 22, 1),
            _ => abort(0),
        };
    }

    fun test3() {
        let x = m::make(22);
        match (&mut x) {
            m::Wrapper::V1(v) => assert!(*v == 22, 1),
            m::Wrapper::V2(v) => {
                *v = 0;
            }
        };
    }
}
