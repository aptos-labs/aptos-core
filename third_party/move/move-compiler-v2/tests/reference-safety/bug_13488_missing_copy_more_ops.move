module 0xc0ffee::m {

    fun test() {
        let x = 0;
        let r1 = &x;
        let r2 = &x;
        x + copy x;
        r1;
        r2;
        assert!(*r1 == 0, 1);
        assert!(*r2 == 0, 1);
    }

    fun test2() {
        let x = 2;
        let _r1 = &x;
        let _r2 = &x;
        assert!(x + copy x == 4, 1);
        _r1;
        _r2;
    }

    fun test3() {
        use std::vector;
        let x = 1;
        let _r1 = &x;
        let v = vector[];
        vector::push_back(&mut v, x);
        _r1;
        assert!(*vector::borrow(&v, 0) == 1, 0);
    }

    fun t(x: u64): u64 {
        x
    }

    fun test5() {
        let x = 0;
        let _r1 = &x;
        let y = t(x);
        _r1;
        assert!(y == 0, 0);
    }
}
