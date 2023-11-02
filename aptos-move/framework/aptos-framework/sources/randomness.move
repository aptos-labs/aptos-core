/// On-chain randomness Move API as specified in [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md).
module aptos_std::randomness {

    use std::hash::sha3_256;
    use std::vector;
    use aptos_framework::system_addresses;
    use aptos_framework::transaction_context::get_transaction_hash;

    friend aptos_framework::block;

    const EUT_FAILURE: u64 = 1;
    const DST: vector<u8> = b"APTOS_RANDOMNESS";

    /// The block-level seed randomness, only stored under account `0x0`.
    /// It's updated at the beginning of every block.
    ///
    /// NOTE: This is a seed shared by every txn in the block. Your transaction should NOT consume it directly.
    /// Use the public functions in this module instead.
    struct BlockRandomness has drop, key {
        block_randomness: vector<u8>,
    }

    /// Invoked in `block_prologue_ext()` to update the block-level seed randomness.
    public(friend) fun on_new_block(vm: &signer, randomness_available: bool, block_randomness: vector<u8>) acquires BlockRandomness {
        system_addresses::assert_vm(vm);
        if (exists<BlockRandomness>(@vm)) {
            move_from<BlockRandomness>(@vm);
        };
        if (randomness_available) {
            move_to(vm, BlockRandomness { block_randomness })
        };
    }

    /// Generate 32 random bytes.
    public fun next_blob(): vector<u8> acquires BlockRandomness {
        let input = DST;
        vector::append(&mut input, borrow_global<BlockRandomness>(@vm).block_randomness);
        vector::append(&mut input, get_transaction_hash());
        vector::append(&mut input, get_and_add_txn_local_state());
        sha3_256(input)
    }

    /// Generates a u64 uniformly at random.
    public fun u64_integer(): u64 acquires BlockRandomness {
        let raw = next_blob();
        let i = 0;
        let ret: u64 = 0;
        while (i < 8) {
            ret = ret * 256 + (vector::pop_back(&mut raw) as u64);
            i = i + 1;
        };
        ret
    }

    /// Generates a u256 uniformly at random.
    public fun u256_integer(): u256 acquires BlockRandomness {
        let raw = next_blob();
        let i = 0;
        let ret: u256 = 0;
        while (i < 32) {
            ret = ret * 256 + (vector::pop_back(&mut raw) as u256);
            i = i + 1;
        };
        ret
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: the uniformity is not perfect, but it can be proved that the probability error is no more than 1/2^192.
    /// If you need perfect uniformty, consider implement your own with `u64_integer()` + rejection sampling.
    public fun u64_range(min_incl: u64, max_excl: u64): u64 acquires BlockRandomness {
        let range = ((max_excl - min_incl) as u256);
        let sample = ((u256_integer() % range) as u64);
        min_incl + sample
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: the uniformity is not perfect, but it can be proved that the probability error is no more than 1/2^256.
    /// If you need perfect uniformty, consider implement your own with `u256_integer()` + rejection sampling.
    public fun u256_range(min_incl: u256, max_excl: u256): u256 acquires BlockRandomness {
        let range = max_excl - min_incl;
        let r0 = u256_integer();
        let r1 = u256_integer();

        // Will compute sample := (r0 + r1*2^256) % range.

        let sample = r1 % range;
        let i = 0;
        while (i < 256) {
            sample = safe_add_mod(sample, sample, range);
            i = i + 1;
        };

        let sample = safe_add_mod(sample, r0 % range, range);

        min_incl + sample
    }

    /// Generate a permutation of `[0, 1, ..., n-1]` uniformly at random.
    public fun permutation(n: u64): vector<u64> acquires BlockRandomness {
        let values = vector[];

        // Initialize into [0, 1, ..., n-1].
        let i = 0;
        while (i < n) {
            std::vector::push_back(&mut values, i);
            i = i + 1;
        };

        // Shuffle.
        let tail = n - 1;
        while (tail > 0) {
            let pop_position = u64_range(0, tail);
            std::vector::swap(&mut values, pop_position, tail);
            tail = tail - 1;
        };

        values
    }

    /// Compute `(a + b) % m`, assuming `m >= 1, 0 <= a < m, 0<= b < m`.
    inline fun safe_add_mod(a: u256, b: u256, m: u256): u256 {
        let neg_b = m - b;
        if (a < neg_b) {
            a + b
        } else {
            a - neg_b
        }
    }

    native fun get_and_add_txn_local_state(): vector<u8>;

    #[test]
    fun test_safe_add_mod() {
        assert!(2 == safe_add_mod(3, 4, 5), std::error::aborted(EUT_FAILURE));
        assert!(2 == safe_add_mod(4, 3, 5), std::error::aborted(EUT_FAILURE));
        assert!(7 == safe_add_mod(3, 4, 9), std::error::aborted(EUT_FAILURE));
        assert!(7 == safe_add_mod(4, 3, 9), std::error::aborted(EUT_FAILURE));
        assert!(0xfffffffffffffffffffffffffffffffffffffffffffffffe == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0x000000000000000000000000000000000000000000000001, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
        assert!(0xfffffffffffffffffffffffffffffffffffffffffffffffe == safe_add_mod(0x000000000000000000000000000000000000000000000001, 0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
        assert!(0x000000000000000000000000000000000000000000000000 == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0x000000000000000000000000000000000000000000000002, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
        assert!(0x000000000000000000000000000000000000000000000000 == safe_add_mod(0x000000000000000000000000000000000000000000000002, 0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
        assert!(0x000000000000000000000000000000000000000000000001 == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0x000000000000000000000000000000000000000000000003, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
        assert!(0x000000000000000000000000000000000000000000000001 == safe_add_mod(0x000000000000000000000000000000000000000000000003, 0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
        assert!(0xfffffffffffffffffffffffffffffffffffffffffffffffd == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffe, 0xfffffffffffffffffffffffffffffffffffffffffffffffe, 0xffffffffffffffffffffffffffffffffffffffffffffffff), std::error::aborted(EUT_FAILURE));
    }
}
