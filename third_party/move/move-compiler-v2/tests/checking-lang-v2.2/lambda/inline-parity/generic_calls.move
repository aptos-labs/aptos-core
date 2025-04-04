module 0x42::m {

    struct S<T> has drop { x: T }


    fun inlined<T:drop>(f: |S<T>|S<T>, s: S<T>) {
        f(s);
    }

    fun id<T>(self: S<T>): S<T> {
        self
    }

    fun test_receiver_inference(s: S<u64>) {
        // In the lambda the type of `s` is not known when the expression is checked,
        // and the receiver function `id` is resolved later when the parameter type is unified
        // with the lambda expression
        inlined(|s| s.id(), s)
    }
}
