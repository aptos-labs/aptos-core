module 0xc0ffee::m {

    public enum E has copy, drop {
        V1 {
            o1: u8
        },
        V2 {
            o1: u8, o3: u16
        },
        V3 {
            o2: u8
        },
        V4 {
            o2: u8
        }
    }

}

module 0xc0ffee::n6 {
    use 0xc0ffee::m::E;

    fun test_match(w: E): bool {
        match (w) {
            V1 {
                o1: _
            } => true,
            V2 {
                o1: _, o3: _
            } => false,
            V3 {
                o2: _
            } => false,
            V4 {
                o2: _
            } => false,
        }
    }

    fun mut_borrow(w: &mut E): (&mut u8, u16) {
        let y = w.o3;
        (&mut w.o1, y)
    }

    fun test_mut_borrow() {
        let x = E::V2 { o1: 0, o3: 0 };
        let (a, _) = mut_borrow(&mut x);
        *a = 1;
        assert!(x.o1 == 1, 1);
        assert!(x.o3 == 0, 2);
    }


}
