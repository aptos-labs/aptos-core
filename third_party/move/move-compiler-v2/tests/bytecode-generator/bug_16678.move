module 0xCAFE::Module0 {
    public inline fun function1() {
        (||{})();
    }

    public fun function2() {
        (function1)();
    }

    public fun function3() {
        function1();
    }

    public inline fun foo(x: u64): u64 {
        x + 1
    }

    public fun negative(): u64 {
        let f = foo;
        f(42)
    }

    public fun positive_1(): u64 {
        let f = || foo(42);
        f()
    }

    public fun positive_2(): u64 {
        let f = |x| foo(x);
        f(42)
    }
}
