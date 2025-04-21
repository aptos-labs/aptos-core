//# publish
module 0xc0ffee::m {
    public fun test0() {
        let v = 1;
        v += {v += {v += 2; v}; v};
        assert!(v == 12);
    }

	public fun test1() {
        let v = 1;
        v += {v += 2; v};
        assert!(v == 6);
    }

	fun mod1(r: &mut u64) {
        *r += 2;
    }

    public fun test2() {
        let v = 1;
        v += {mod1(&mut v); v};
        assert!(v == 6);
    }

	fun mod2(r: &mut u64): u64 {
        *r += 2;
        *r
    }

    public fun test3() {
        let v = 1;
        v += mod2(&mut v);
        assert!(v == 6);
    }

    public fun test4() {
        let i = 0;
        let xs = vector<u64>[1, 2, 3];
        xs[{ i += 1; i }] += xs[{ i += 1; i }];
        assert!(xs == vector<u64>[1, 2, 5]);
    }
}

//# run --verbose -- 0xc0ffee::m::test0

//# run --verbose -- 0xc0ffee::m::test1

//# run --verbose -- 0xc0ffee::m::test2

//# run --verbose -- 0xc0ffee::m::test3

//# run --verbose -- 0xc0ffee::m::test4
