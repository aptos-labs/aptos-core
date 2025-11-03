module 0xc0ffee::m {
    const TRUE: bool = true;

    fun foo(x: u64): bool {
        x > 10
    }

    fun take(_x: bool) {}

    fun bar() {}

    #[lint::skip(cyclomatic_complexity)]
    public fun test1(x: u64) {
        if (foo(x) == true) { bar() };
        if (foo(x) == false) { bar() };
        if (foo(x) != true) { bar() };
        if (foo(x) != false) { bar() };
        if (true == foo(x)) { bar() };
        if (false == foo(x)) { bar() };
        if (true != foo(x)) { bar() };
        if (false != foo(x)) { bar() };
        if ((x + 1 > 0) == false) { bar() };
        let _y = foo(x) == true;
        assert!(true == !foo(x), 42);
        take(foo(x) == true);
        let _z = foo(x) == TRUE;
    }

    public fun test2(x: &bool, y: &bool) {
        if ((*x && *y) == true) { bar() };
    }
}

module 0xc0ffee::no_warn {
    #[lint::skip(unnecessary_boolean_identity_comparison)]
    public fun test(x: bool) {
        if (x == true) abort 1;
    }
}
