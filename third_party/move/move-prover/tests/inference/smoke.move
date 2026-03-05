/// Smoke test for the inference end-to-end test harness.
module 0x1::smoke {

    fun add(a: u64, b: u64): u64 {
        a + b
    }

    fun id(x: u64): u64 {
        x
    }
}
