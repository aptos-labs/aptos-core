module 0xCAFE::Module0 {
    const ADDR: address = @0xBEEF;
    struct S has copy, drop, store, key { }

    public fun function1() acquires S {
        return;
        function2();
    }
    public fun function2() acquires S {
        borrow_global<S>(ADDR);
    }
}
