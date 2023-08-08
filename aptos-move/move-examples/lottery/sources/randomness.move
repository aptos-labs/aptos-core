module aptos_std_extra::randomness {
    use std::vector;

    /// A _random number generator (RNG)_ object that stores entropy from the on-chain randomness beacon.
    ///
    /// This RNG object can be used to produce one or more random numbers, random permutation, etc.
    struct RandomNumberGenerator has drop { /* ... */ }

    /// Returns a uniquely-seeded RNG.
    ///
    /// Repeated calls to this function will return an RNG with a different seed. This is to
    /// prevent developers from accidentally calling `rng` twice and generating the same randomness.
    ///
    /// Calls to this function **MUST** only be made from private entry functions in modules that have
    /// no other functions. This is to prevent _test-and-abort_ attacks.
    public fun rng(): RandomNumberGenerator { RandomNumberGenerator { /* ... */ } }

    /// Generates a number uniformly at random.
    public fun u64_integer(_rng: &mut RandomNumberGenerator): u64 { 42 }
    public fun u256_integer(_rng: &mut RandomNumberGenerator): u256 { 42 }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    public fun u64_range(_rng: &mut RandomNumberGenerator, _min_incl: u64, _max_excl: u64): u64 { _min_incl }
    public fun u256_range(_rng: &mut RandomNumberGenerator, _min_incl: u256, _max_excl: u256): u256 { _min_incl }

    /* Similar methods for u8, u16, u32, u64, and u128. */

    /// Generate a permutation of `[0, 1, ..., n-1]` uniformly at random.
    public fun permutation(_rng: &mut RandomNumberGenerator, n: u64): vector<u64> {
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
    /// Test-only function to set the entropy in the RNG to a specific value, which is useful for
    /// testing.
    public fun set_seed(_seed: vector<u8>) {

    }

    //
    // More functions can be added here to support other randomness generations operations
    //
}
