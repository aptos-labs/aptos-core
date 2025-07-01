//# publish
module 0x8675309::M {
    struct S<T> { f: T }

    fun f<T>() {
        let ff = || f<S<T>>();
        ff();
    }
}
