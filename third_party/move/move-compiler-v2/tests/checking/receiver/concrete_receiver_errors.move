module 0x42::m {
    struct G<T, R> has drop, copy { x: T, y: R }

    // Concrete receiver defined for G<u64, bool>
    fun process(self: G<u64, bool>) {}

    // === Negative: type mismatch at call site ===
    fun test_wrong_first_arg() {
        let g = G<bool, bool> { x: true, y: false };
        g.process();
    }

    fun test_wrong_second_arg() {
        let g = G<u64, u64> { x: 1, y: 2 };
        g.process();
    }

    fun test_both_wrong() {
        let g = G<bool, u64> { x: true, y: 1 };
        g.process();
    }

    // === Negative: partial instantiation still rejected at declaration ===
    fun partial_bad<T>(self: G<u64, T>) {}
}
