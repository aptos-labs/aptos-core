module 0xc0ffee::m {
    struct S<T> { f: T }

    fun f<T>() {
        f<|T|>();
    }
}

module 0xc0ffee::n {
    public fun foo<T>(): || {
        bar<|&T|>
    }

    fun bar<T>() {
        (foo<T>())();
    }

    fun test() {
        let f = foo<||>();
        f();
    }
}

module 0xc0ffee::o {
    public fun foo<T>(): ||(||) {
        bar<|&T|>
    }

    fun bar<T>(): || {
        maker<T>()()()
    }

    fun maker<T>(): ||(||(||)) {
        foo<T>
    }

    fun test() {
        let f = foo<||>();
        f()();
    }
}

module 0xc0ffee::p {
    public fun foo<T>(): || {
        bar<||(T, T)>
    }

    fun bar<T>() {
        (foo<||T>())();
    }

    fun test() {
        let f = foo<||>();
        f();
    }
}
