module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun two(): u64 {
        2
    }

    fun id(x: u64): u64 {
        x
    }

    public fun test1() {
        let x = one();
        let y = two();
        id(x);
        id(y);
    }

    struct Foo {
        x: u64
    }

    public fun test2() {
        let x = 1;
        let y = 2;
        id(x);
        id(y);
    }

    public fun test3(x: u64): u64 {
        x + 1 + x + x
    }

    fun bar(_x: u64, _y: u64) {}

    fun baz(_x: u64) {}

    public fun test4() {
        let a = one();
        let b = two();
        let c = one() + two();
        bar(a, b);
        baz(c);
    }

}
