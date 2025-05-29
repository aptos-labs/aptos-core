module 0xCAFE::Module1 {
    public inline fun function1() {}

    public fun function2() {
        (function1)();
    }

    public fun function3() {
        f3(function1);
    }

    public fun f3(x: ||) {
        x();
    }

    struct S {
        x: ||,
    }

    public fun f4() {
        let s = S { x: function1 };
        let S { x } = s;
        x();
    }
}
