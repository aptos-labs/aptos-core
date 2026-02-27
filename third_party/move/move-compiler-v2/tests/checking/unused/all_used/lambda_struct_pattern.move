// Test that structs used in lambda patterns are tracked.
module 0x42::m {
    struct Pair has drop, copy { x: u64, y: u64 }

    inline fun apply(p: Pair, f: |Pair| u64): u64 {
        f(p)
    }

    public fun test(): u64 {
        let p = Pair { x: 10, y: 20 };
        apply(p, |Pair { x, y }| x + y)
    }
}
