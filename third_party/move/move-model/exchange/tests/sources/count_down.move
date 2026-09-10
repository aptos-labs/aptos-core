module 0x42::count_down {
    fun count_down(x: u64): u64 {
        while (0 < x) {
            x = x - 1;
        } spec {
            invariant x <= 18446744073709551615;
        };
        x
    }
    spec count_down {
        ensures result == 0;
    }
}
