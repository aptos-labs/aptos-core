/// On-chain randomness utils.
module aptos_framework::randomness {
    use std::hash;
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_framework::system_addresses;
    use aptos_framework::transaction_context;
    #[test_only]
    use aptos_std::debug;

    friend aptos_framework::block;

    const DST: vector<u8> = b"APTOS_RANDOMNESS";

    /// Randomness APIs calls must originate from a private entry function. Otherwise, test-and-abort attacks are possible.
    const E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT: u64 = 1;

    /// 32-byte randomness seed unique to every block.
    /// This resource is updated in every block prologue.
    struct PerBlockRandomness has drop, key {
        epoch: u64,
        round: u64,
        seed: Option<vector<u8>>,
    }

    /// Called in genesis.move.
    /// Must be called in tests to initialize the `PerBlockRandomness` resource.
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, PerBlockRandomness {
            epoch: 0,
            round: 0,
            seed: option::none(),
        });
    }

    #[test_only]
    public fun initialize_for_testing(framework: &signer) acquires  PerBlockRandomness {
        initialize(framework);
        set_seed(x"0000000000000000000000000000000000000000000000000000000000000000");
    }

    /// Invoked in block prologues to update the block-level randomness seed.
    public(friend) fun on_new_block(vm: &signer, epoch: u64, round: u64, seed_for_new_block: Option<vector<u8>>) acquires PerBlockRandomness {
        system_addresses::assert_vm(vm);
        if (exists<PerBlockRandomness>(@aptos_framework)) {
            let randomness = borrow_global_mut<PerBlockRandomness>(@aptos_framework);
            randomness.epoch = epoch;
            randomness.round = round;
            randomness.seed = seed_for_new_block;
        }
    }

    /// Generate 32 random bytes.
    fun next_blob(): vector<u8> acquires PerBlockRandomness {
        assert!(is_safe_call(), E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT);

        let input = DST;
        let randomness = borrow_global<PerBlockRandomness>(@aptos_framework);
        let seed = *option::borrow(&randomness.seed);
        vector::append(&mut input, seed);
        vector::append(&mut input, transaction_context::get_transaction_hash());
        vector::append(&mut input, fetch_and_increment_txn_counter());
        hash::sha3_256(input)
    }

    /// Generates an u8 uniformly at random.
    public fun u8_integer(): u8 acquires PerBlockRandomness {
        let raw = next_blob();
        let ret: u8 = vector::pop_back(&mut raw);
        ret
    }

    /// Generates an u16 uniformly at random.
    public fun u16_integer(): u16 acquires PerBlockRandomness {
        let raw = next_blob();
        let i = 0;
        let ret: u16 = 0;
        while (i < 2) {
            ret = ret * 256 + (vector::pop_back(&mut raw) as u16);
            i = i + 1;
        };
        ret
    }

    /// Generates an u32 uniformly at random.
    public fun u32_integer(): u32 acquires PerBlockRandomness {
        let raw = next_blob();
        let i = 0;
        let ret: u32 = 0;
        while (i < 4) {
            ret = ret * 256 + (vector::pop_back(&mut raw) as u32);
            i = i + 1;
        };
        ret
    }

    /// Generates an u64 uniformly at random.
    public fun u64_integer(): u64 acquires PerBlockRandomness {
        let raw = next_blob();
        let i = 0;
        let ret: u64 = 0;
        while (i < 8) {
            ret = ret * 256 + (vector::pop_back(&mut raw) as u64);
            i = i + 1;
        };
        ret
    }

    /// Generates an u128 uniformly at random.
    public fun u128_integer(): u128 acquires PerBlockRandomness {
        let raw = next_blob();
        let i = 0;
        let ret: u128 = 0;
        while (i < 16) {
            ret = ret * 256 + (vector::pop_back(&mut raw) as u128);
            i = i + 1;
        };
        ret
    }

    /// Generates a u256 uniformly at random.
    public fun u256_integer(): u256 acquires PerBlockRandomness {
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
    /// NOTE: The uniformity is not perfect, but it can be proved that the bias is negligible.
    /// If you need perfect uniformity, consider implement your own via rejection sampling.
    public fun u8_range(min_incl: u8, max_excl: u8): u8 acquires PerBlockRandomness {
        let range = ((max_excl - min_incl) as u256);
        let sample = ((u256_integer() % range) as u8);
        min_incl + sample
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: The uniformity is not perfect, but it can be proved that the bias is negligible.
    /// If you need perfect uniformity, consider implement your own via rejection sampling.
    public fun u16_range(min_incl: u16, max_excl: u16): u16 acquires PerBlockRandomness {
        let range = ((max_excl - min_incl) as u256);
        let sample = ((u256_integer() % range) as u16);
        min_incl + sample
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: The uniformity is not perfect, but it can be proved that the bias is negligible.
    /// If you need perfect uniformity, consider implement your own via rejection sampling.
    public fun u32_range(min_incl: u32, max_excl: u32): u32 acquires PerBlockRandomness {
        let range = ((max_excl - min_incl) as u256);
        let sample = ((u256_integer() % range) as u32);
        min_incl + sample
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: The uniformity is not perfect, but it can be proved that the bias is negligible.
    /// If you need perfect uniformity, consider implement your own via rejection sampling.
    public fun u64_range(min_incl: u64, max_excl: u64): u64 acquires PerBlockRandomness {
        let range = ((max_excl - min_incl) as u256);
        let sample = ((u256_integer() % range) as u64);
        min_incl + sample
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: The uniformity is not perfect, but it can be proved that the bias is negligible.
    /// If you need perfect uniformity, consider implement your own via rejection sampling.
    public fun u128_range(min_incl: u128, max_excl: u128): u128 acquires PerBlockRandomness {
        let range = ((max_excl - min_incl) as u256);
        let sample = ((u256_integer() % range) as u128);
        min_incl + sample
    }

    /// Generates a number $n \in [min_incl, max_excl)$ uniformly at random.
    ///
    /// NOTE: The uniformity is not perfect, but it can be proved that the bias is negligible.
    /// If you need perfect uniformity, consider implement your own with `u256_integer()` + rejection sampling.
    public fun u256_range(min_incl: u256, max_excl: u256): u256 acquires PerBlockRandomness {
        let range = max_excl - min_incl;
        let r0 = u256_integer();
        let r1 = u256_integer();

        // Will compute sample := (r0 + r1*2^256) % range.

        let sample = r1 % range;
        let i = 0;
        while ({
            spec {
                invariant sample >= 0 && sample < max_excl - min_incl;
            };
            i < 256
        }) {
            sample = safe_add_mod(sample, sample, range);
            i = i + 1;
        };

        let sample = safe_add_mod(sample, r0 % range, range);
        spec {
            assert sample >= 0 && sample < max_excl - min_incl;
        };

        min_incl + sample
    }

    /// Generate a permutation of `[0, 1, ..., n-1]` uniformly at random.
    public fun permutation(n: u64): vector<u64> acquires PerBlockRandomness {
        let values = vector[];

        // Initialize into [0, 1, ..., n-1].
        let i = 0;
        while ({
            spec {
                invariant i <= n;
                invariant len(values) == i;
            };
            i < n
        }) {
            std::vector::push_back(&mut values, i);
            i = i + 1;
        };
        spec {
            assert len(values) == n;
        };

        // Shuffle.
        let tail = n - 1;
        while ({
            spec {
                invariant tail >= 0 && tail < len(values);
            };
            tail > 0
        }) {
            let pop_position = u64_range(0, tail + 1);
            spec {
                assert pop_position < len(values);
            };
            std::vector::swap(&mut values, pop_position, tail);
            tail = tail - 1;
        };

        values
    }

    #[test_only]
    public fun set_seed(seed: vector<u8>) acquires PerBlockRandomness {
        assert!(vector::length(&seed) == 32, 0);
        let randomness = borrow_global_mut<PerBlockRandomness>(@aptos_framework);
        randomness.seed = option::some(seed);
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

    #[verify_only]
    fun safe_add_mod_for_verification(a: u256, b: u256, m: u256): u256 {
        let neg_b = m - b;
        if (a < neg_b) {
            a + b
        } else {
            a - neg_b
        }
    }

    /// Fetches and increments a transaction-specific 32-byte randomness-related counter.
    native fun fetch_and_increment_txn_counter(): vector<u8>;

    /// Called in each randomness generation function to ensure certain safety invariants.
    ///  1. Ensure that the TXN that led to the call of this function had a private (or friend) entry function as its TXN payload.
    ///  2. TBA
    native fun is_safe_call(): bool;

    #[test]
    fun test_safe_add_mod() {
        assert!(2 == safe_add_mod(3, 4, 5), 1);
        assert!(2 == safe_add_mod(4, 3, 5), 1);
        assert!(7 == safe_add_mod(3, 4, 9), 1);
        assert!(7 == safe_add_mod(4, 3, 9), 1);
        assert!(0xfffffffffffffffffffffffffffffffffffffffffffffffe == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0x000000000000000000000000000000000000000000000001, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
        assert!(0xfffffffffffffffffffffffffffffffffffffffffffffffe == safe_add_mod(0x000000000000000000000000000000000000000000000001, 0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
        assert!(0x000000000000000000000000000000000000000000000000 == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0x000000000000000000000000000000000000000000000002, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
        assert!(0x000000000000000000000000000000000000000000000000 == safe_add_mod(0x000000000000000000000000000000000000000000000002, 0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
        assert!(0x000000000000000000000000000000000000000000000001 == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0x000000000000000000000000000000000000000000000003, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
        assert!(0x000000000000000000000000000000000000000000000001 == safe_add_mod(0x000000000000000000000000000000000000000000000003, 0xfffffffffffffffffffffffffffffffffffffffffffffffd, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
        assert!(0xfffffffffffffffffffffffffffffffffffffffffffffffd == safe_add_mod(0xfffffffffffffffffffffffffffffffffffffffffffffffe, 0xfffffffffffffffffffffffffffffffffffffffffffffffe, 0xffffffffffffffffffffffffffffffffffffffffffffffff), 1);
    }

    #[test(fx = @aptos_framework)]
    fun randomness_smoke_test(fx: signer) acquires PerBlockRandomness {
        initialize(&fx);
        set_seed(x"0000000000000000000000000000000000000000000000000000000000000000");
        // Test cases should always be a safe place to do a randomness call from.
        assert!(is_safe_call(), 0);
        let num = u64_integer();
        debug::print(&num);
    }
}
