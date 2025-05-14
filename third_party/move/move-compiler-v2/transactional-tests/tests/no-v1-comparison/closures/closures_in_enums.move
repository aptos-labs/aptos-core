//# publish
module 0xc0ffee::m {
    enum Function has copy, drop {
        NoParam(||u64),
        OneParam(|u64|u64),
        TwoParam(|u64, u64|u64),
    }

    fun runner_1(f: Function): u64 {
        match (f) {
            Function::NoParam(func) => func(),
            Function::OneParam(func) => func(0),
            Function::TwoParam(func) => func(0, 0),
        }
    }

    public fun test_1() {
        let f1 = Function::NoParam(|| 0);
        assert!(runner_1(f1) == 0);

        let f2 = Function::OneParam(|x| x);
        assert!(runner_1(f2) == 0);

        let f3 = Function::TwoParam(|x, y| x + y);
        assert!(runner_1(f3) == 0);

        let x = 1;
        let f4 = Function::NoParam(|| x);
        assert!(runner_1(f4) == 1);

        let x = 1;
        let f5 = Function::OneParam(|y| x + y);
        assert!(runner_1(f5) == 1);
    }

    fun runner_2(f: Function): u64 {
        let to_call = match (f) {
            Function::NoParam(func) => func,
            Function::OneParam(func) => || func(0),
            Function::TwoParam(func) => || func(0, 0),
        };
        to_call()
    }

    public fun test_2() {
        let f1 = Function::NoParam(|| 0);
        assert!(runner_2(f1) == 0);

        let f2 = Function::OneParam(|x| x);
        assert!(runner_2(f2) == 0);

        let f3 = Function::TwoParam(|x, y| x + y);
        assert!(runner_2(f3) == 0);

        let x = 1;
        let f4 = Function::NoParam(|| x);
        assert!(runner_2(f4) == 1);

        let x = 1;
        let f5 = Function::OneParam(|y| x + y);
        assert!(runner_2(f5) == 1);
    }

    fun get_func(f: &Function): &||u64 has copy {
        match (f) {
            Function::NoParam(func) => func,
            _ => abort 42
        }
    }

    fun runner_3(f: &Function): u64 {
        (*get_func(f))()
    }

    fun test_3() {
        let f1 = Function::NoParam(|| 0);
        assert!(runner_3(&f1) == 0);
    }

    fun get_func_recursive(f: Function): ||u64 {
        match (f) {
            Function::NoParam(func) => func,
            Function::OneParam(func) => get_func_recursive(Function::NoParam(||func(0))),
            Function::TwoParam(func) => get_func_recursive(Function::OneParam(|x|func(0, x))),
        }
    }

    fun runner_4(f: Function): u64 {
        get_func_recursive(f)()
    }

    public fun test_4() {
        let f1 = Function::NoParam(|| 0);
        assert!(runner_4(f1) == 0);

        let f2 = Function::OneParam(|x| x);
        assert!(runner_4(f2) == 0);

        let f3 = Function::TwoParam(|x, y| x + y);
        assert!(runner_4(f3) == 0);

        let x = 1;
        let f4 = Function::NoParam(|| x);
        assert!(runner_4(f4) == 1);

        let x = 1;
        let f5 = Function::OneParam(|y| x + y);
        assert!(runner_4(f5) == 1);
    }
}

//# run 0xc0ffee::m::test_1

//# run 0xc0ffee::m::test_2

//# run 0xc0ffee::m::test_3

//# run 0xc0ffee::m::test_4
