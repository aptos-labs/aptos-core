//# publish
module 0xc0ffee::m {
    use std::vector;

    fun test3() {
        let x = 1;
        let _r1 = &x;
        let v = vector[];
        vector::push_back(&mut v, x);
        _r1;
        assert!(*vector::borrow(&v, 0) == 1, 0);
    }

    fun test4() {
        let x = 0;
        let _r1 = &x;
        let y = 3;
        let z = &mut y;
        *z = x;
        _r1;
        assert!(*z == 0, 0);
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

    fun test6() {
        let x = 0;
        let _r1 = &x;
        let v = vector[x];
        _r1;
        assert!(*vector::borrow(&v, 0) == 0, 0);
    }

}

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6
