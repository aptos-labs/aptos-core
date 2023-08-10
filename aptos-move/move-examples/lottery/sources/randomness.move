module aptos_std_extra::randomness {
    use std::vector;

    /// Generates a byte uniformly at random.
    public fun byte(): u8 { 0u8 }

    /// Generates `n` bytes uniformly at random.
    public fun bytes(n: u64): vector<u8> {
        let v = vector::empty<u8>();
        let i = 0;

        while (i < n) {
            vector::push_back(&mut v, byte());
            i = i + 1;
        };

        v
    }

    /// Generates a number uniformly at random.
    public fun u64_integer(): u64 { 0 }
    public fun u256_integer(): u256 { 0 }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    public fun u64_range(min_incl: u64, max_excl: u64): u64 { max_excl; min_incl }
    public fun u256_range(min_incl: u256, max_excl: u256): u256 { max_excl; min_incl }

    /* Similar methods for u8, u16, u32, u64, and u128. */

    /// Generate a permutation of `[0, 1, ..., n-1]` uniformly at random.
    public fun permutation(n: u64): vector<u64> {
        let i = 0;
        let v = vector::empty<u64>();

        while (i < n) {
            vector::push_back(&mut v, i);
            i = i + 1;
        };

        // TODO: Shuffle the vector based on r

        assert!(vector::length(&v) == n, 1);

        v
    }

    #[test_only]
    /// Test-only function to set the entropy in the random number generator
    /// to a specific value, which is useful for testing.
    public fun set_seed(seed: vector<u8>) {
        seed;
    }

    //
    // More functions can be added here to support other randomness generations operations
    //
}
