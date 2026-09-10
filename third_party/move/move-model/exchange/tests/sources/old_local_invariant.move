module 0x42::old_local_invariant {
    fun f(x: u64): u64 {
        while (0 < x) {
            x = x - 1;
        } spec {
            invariant old(x) >= x;
        };
        x
    }
}
