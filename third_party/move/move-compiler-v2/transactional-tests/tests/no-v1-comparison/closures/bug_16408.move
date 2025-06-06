//# publish
module 0xc0ffee::m {
    fun make_func_1(): |u64|(|u64|u64) has copy + drop {
        |x: u64| {
            return (|y| x + y)
        }
    }

    fun make_func_2(): |u64, u64|(||u64) has copy + drop {
        return (|x, y| {
            return (|| x * y)
        })
    }

    fun make_func_3(): |u64, u64|(||(u64, u64)) has copy + drop {
        return (|x, y| {
            return (|| (x, y))
        })
    }

    fun test() {
        assert!(make_func_1()(10)(20) == 30);
        assert!(make_func_2()(10, 20)() == 200);
        let (a, b) = make_func_3()(10, 20)();
        assert!(a == 10);
        assert!(b == 20);
    }
}

//# run 0xc0ffee::m::test
