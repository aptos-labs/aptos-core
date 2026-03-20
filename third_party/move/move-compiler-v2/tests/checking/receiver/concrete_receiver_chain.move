module 0x42::m {
    struct Pair has drop, copy { a: u64, b: u64 }
    struct Wrapper<T> has drop, copy { inner: T }

    // Concrete receiver on Wrapper<Pair>
    fun get_pair(self: &Wrapper<Pair>): Pair { self.inner }

    // Regular receiver on Pair (non-generic struct)
    fun sum(self: &Pair): u64 { self.a + self.b }

    // === Chaining: concrete receiver result feeds into another receiver ===
    fun test_chain() {
        let w = Wrapper<Pair> { inner: Pair { a: 3, b: 4 } };
        // Chain: w.get_pair() returns Pair, then .sum() on Pair
        assert!(w.get_pair().sum() == 7, 0);
    }

    // === Chaining through vector index ===
    fun test_chain_vector() {
        let w = Wrapper<Pair> { inner: Pair { a: 10, b: 20 } };
        let v = vector[w];
        assert!(v[0].get_pair().sum() == 30, 0);
    }
}
