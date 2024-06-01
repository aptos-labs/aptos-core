//# publish
module 0xdeadbeef::test {
    fun add(a: u64, b: u64, c: u64): u64 {
        a + b + c
    }

    fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    fun test1(): u64 {
        let x = 1;
        add(x, inc(&mut x), inc(&mut x))
    }

    fun test2(): u64 {
        let x = 1;
        add({x = x + 1; x}, {x = x + 1; x}, x)
    }

    fun test3(): u64 {
        let x = 1;
        add({x = x + 1; x}, x, {x = x + 1; x})
    }

    fun test4(): u64 {
        let x = 1;
        add({x = x + 1; x}, {x = x + 1; x}, {x = x + 1; x})
    }
}

//# run 0xdeadbeef::test::test1

//# run 0xdeadbeef::test::test2

//# run 0xdeadbeef::test::test3

//# run 0xdeadbeef::test::test4
