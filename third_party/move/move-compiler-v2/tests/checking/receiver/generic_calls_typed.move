module 0x42::m {

    struct S<T> { x: T }

    // Call styles

    fun receiver<T>(self: S<T>, y: T) {
        self.x = y;
    }

    fun receiver_ref<T>(self: &S<T>, _y: T) {
    }

    fun receiver_ref_mut<T>(self: &mut S<T>, y: T) {
        self.x = y
    }

    fun receiver_more_generics<T, R>(self: S<T>, _y: R) {
    }

    fun receiver_needs_type_args<T, R>(self: S<T>, _y: T) {
        abort 1
    }

    fun test_call_styles(s: S<u64>, x: u64) {
        s.receiver(x);
        s.receiver_ref(x);
        s.receiver_ref_mut(x);
        s.receiver_more_generics(22);
        s.receiver_needs_type_args::<u64, u8>(x);
    }

    // Inference of receiver function

    inline fun inlined<T>(f: |S<T>|S<T>, s: S<T>) {
        f(s);
    }

    fun id<T>(self: S<T>): S<T> {
        self
    }

    fun test_receiver_inference(s: S<u64>) {
        // In the lambda the type of `s` is not known when the expression is checked,
        // and the receiver function `id` is resolved later when the parameter type is unified
        // with the lambda expression
        inlined(|s: S<u64>| s.id(), s)
    }
}
