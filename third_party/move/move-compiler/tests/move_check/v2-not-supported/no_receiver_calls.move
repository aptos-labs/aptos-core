module 0x42::m {
    struct T {}

    fun receiver(self: &T, x: u64) { abort 1 }

    fun call_receiver(t: T) {
        t.receiver(1);
        receiver(&t, 1)
    }
}
