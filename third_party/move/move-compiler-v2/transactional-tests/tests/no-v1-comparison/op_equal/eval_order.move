//# publish
module 0xc0ffee::m {
    public fun test0(): u64 {
        let v = 1;
        v += {v += {v += 2; v}; v};
        v
    }

	public fun test1(): u64 {
        let v = 1;
        v += {v += 2; v};
        v
    }

	fun mod1(r: &mut u64) {
        *r += 2;
    }

    public fun test2(): u64 {
        let v = 1;
        v += {mod1(&mut v); v};
        v
    }

	fun mod2(r: &mut u64): u64 {
        *r += 2;
        *r
    }

    public fun test3(): u64 {
        let v = 1;
        v += mod2(&mut v);
        v
    }
}

//# run --verbose -- 0xc0ffee::m::test0

//# run --verbose -- 0xc0ffee::m::test1

//# run --verbose -- 0xc0ffee::m::test2

//# run --verbose -- 0xc0ffee::m::test3
