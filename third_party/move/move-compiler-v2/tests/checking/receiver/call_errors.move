module 0x42::m {

    struct S<T> { x: T }

    // Call styles

    fun receiver<T>(self: S<T>, y: T) {
        self.x = y;
    }

    fun receiver_ref_mut<T>(self: &mut S<T>, y: T) {
        self.x = y
    }

    fun receiver_needs_type_args<T, R>(self: S<T>, _y: T) {
        abort 1
    }

    fun test_call_styles(s: S<u64>, x: u64) {
        s.receiver_wrongly_spelled(x);
        s.receiver(x, x);
        s.receiver_ref_mut(x, x);
        s.receiver();
        s.receiver_ref_mut(1u8);
        s.receiver_ref_mut::<u8>(1);
        s.receiver_ref_mut::<u8, u8>(1);
        s.receiver_needs_type_args(x)
    }
}
