//# publish
module 0xc0ffee::m {
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
