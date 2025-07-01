//# publish
module 0xc0ffee::m {
    public struct Foo has copy, drop {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
        e: u64,
    }

}

//# publish
module 0xc0ffee::test_m {
    use 0xc0ffee::m::Foo;
    fun sequential(p: Foo): Foo {
        let a = p;
        let b = a;
        let c = b;
        let d = c;
        let e = d;
        e
    }

    public fun main() {
        assert!(
            sequential(
                Foo {a: 1, b: 2, c: 3, d: 4, e: 5}
            ) == Foo {a: 1, b: 2, c: 3, d: 4, e: 5},
            0
        );
    }
}

//# run 0xc0ffee::test_m::main
