//# publish
module 0xc0ffee::m {
    public fun test1(): u8 {
        let x = 1;
        let f = |y: u8| {
            let x = x + y; // x: 8
            let f = |y: u8| {
                let x = y + x; // x: 11
                x
            };
            f(3) * {
                let f = |y: u8| {
                    let x = x - y; // x: 4
                    x
                };
                f(4)
            }
        };
        f(7)
    }

    public fun test2(): u64 {
        let x = 1;
        let f = || || x;
        f()()
    }

    public fun test3(): u64 {
        let x = 1;
        let f = |x| {
            (|x| x - 1)(x + 1)
        };
        f(x)
    }

    public fun test4(x: u64): u64 {
        let f = || || {
            let x = x + 1;
            x
        };
        f()()
    }

    public fun test5(): u8 {
        let x: u8 = 2;
        let f = |x: u8| {          // shadows outer `x`
            let g = || { x + 1 };  // captures the *parameter* `x`
            g()
        };
        f(7) + x
    }

    public fun test6(): u64 {
        let x: u64 = 5;
        let f = |y: u64| {
            let x = y + 1;
            let g = || {
                let y = x + 1;
                y
            };
            g()
        };
        f(10) + x
    }

    public fun test7(): u8 {
        let x: u8 = 3;
        let f = |x: u8| {          // shadows outer `x`
            let x = x + 2;         // shadows parameter `x`
            let g = |y: u8| x + y; // captures second‑shadow `x`
            g(4)                   // (3 + 2) + 4
        };
        f(x)
    }

    public fun test8(): u64 {
        let x: u64 = 1;
        let f = || {
            let x = 2;             // shadows outermost
            let g = || {
                let x = 3;         // shadows again
                x                  // 3
            };
            g() + x                // 3 + 2
        };
        f() + x                    // 5 + 1
    }

    public fun test9(): u8 {
        let x: u8 = 10;
        let f = |x: u8| {
            let h = |x: u8| x;     // yet another shadow
            h(x)                   // returns the parameter `x`
        };
        f(4) + (x - 10)            // (4) + 0
    }

    public fun test10(): u8 {
        let x: u8 = 2;
        let f = |y: u8| y + x;     // captures the first `x`
        let x: u8 = 5;             // new shadow of `x`
        f(3) + (x - 5)             //  (3 + 2) + 0
    }

    fun call(f: ||u64): u64 {
        f()
    }

    public fun test11(): u64 {
        let x = 1;
        x + call(|| x + 2) + call(|| x - 1)
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2

//# run 0xc0ffee::m::test3

//# run 0xc0ffee::m::test4 --args 42

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6

//# run 0xc0ffee::m::test7

//# run 0xc0ffee::m::test8

//# run 0xc0ffee::m::test9

//# run 0xc0ffee::m::test10

//# run 0xc0ffee::m::test11
