spec velor_framework::randomness {

    spec module {
        use velor_framework::chain_status;
        pragma verify = true;
        invariant [suspendable] chain_status::is_operating() ==> exists<PerBlockRandomness>(@velor_framework);
        global var: vector<u8>;
    }

    spec fetch_and_increment_txn_counter(): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_fetch_and_increment_txn_counter();
    }

    spec fun spec_fetch_and_increment_txn_counter(): vector<u8>;

    spec is_unbiasable(): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_unbiasable();
    }

    spec fun spec_is_unbiasable(): bool;

    spec initialize(framework: &signer) {
        use std::signer;
        let framework_addr = signer::address_of(framework);
        aborts_if framework_addr != @velor_framework;
    }

    spec on_new_block(vm: &signer, epoch: u64, round: u64, seed_for_new_block: Option<vector<u8>>) {
        use std::signer;
        aborts_if signer::address_of(vm) != @vm;
        ensures exists<PerBlockRandomness>(@velor_framework) ==> global<PerBlockRandomness>(@velor_framework).seed == seed_for_new_block;
        ensures exists<PerBlockRandomness>(@velor_framework) ==> global<PerBlockRandomness>(@velor_framework).epoch == epoch;
        ensures exists<PerBlockRandomness>(@velor_framework) ==> global<PerBlockRandomness>(@velor_framework).round == round;
    }

    spec next_32_bytes(): vector<u8> {
        use std::hash;
        include NextBlobAbortsIf;
        let input = b"VELOR_RANDOMNESS";
        let randomness = global<PerBlockRandomness>(@velor_framework);
        let seed = option::spec_borrow(randomness.seed);
        let txn_hash = transaction_context::spec_get_txn_hash();
        let txn_counter = spec_fetch_and_increment_txn_counter();
        ensures len(result) == 32;
        ensures result == hash::sha3_256(concat(concat(concat(input, seed), txn_hash), txn_counter));
    }

    spec schema NextBlobAbortsIf {
        let randomness = global<PerBlockRandomness>(@velor_framework);
        aborts_if option::spec_is_none(randomness.seed);
        aborts_if !spec_is_unbiasable();
        aborts_if !exists<PerBlockRandomness>(@velor_framework);
    }

    spec u8_integer(): u8 {
        include NextBlobAbortsIf;
    }

    spec u16_integer(): u16 {
        pragma unroll = 2;
        include NextBlobAbortsIf;
    }

    spec u32_integer(): u32 {
        pragma unroll = 4;
        include NextBlobAbortsIf;
    }

    spec u64_integer(): u64 {
        pragma unroll = 8;
        include NextBlobAbortsIf;
    }

    spec u128_integer(): u128 {
        pragma unroll = 16;
        include NextBlobAbortsIf;
    }

    spec u256_integer(): u256 {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        pragma unroll = 32;
        include NextBlobAbortsIf;
        ensures [abstract] result == spec_u256_integer();
    }

    spec u256_integer_internal(): u256 {
        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 300;
        pragma unroll = 32;
        include NextBlobAbortsIf;
    }

    spec fun spec_u256_integer(): u256;

    spec u8_range(min_incl: u8, max_excl: u8): u8 {
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved).
        pragma opaque;
        include NextBlobAbortsIf;
        aborts_if min_incl >= max_excl;
        ensures result >= min_incl && result < max_excl;
    }


    spec u64_range(min_incl: u64, max_excl: u64): u64 {
        pragma verify_duration_estimate = 120;
        include NextBlobAbortsIf;
        aborts_if min_incl >= max_excl;
        ensures result >= min_incl && result < max_excl;
    }

    spec u256_range(min_incl: u256, max_excl: u256): u256 {
        pragma verify_duration_estimate = 120;
        include NextBlobAbortsIf;
        aborts_if min_incl >= max_excl;
        ensures result >= min_incl && result < max_excl;
    }

    spec permutation(n: u64): vector<u64> {
        pragma aborts_if_is_partial;
        // TODO(tengzhang): complete the aborts_if conditions
        // include n > 1 ==> NextBlobAbortsIf;
        // aborts_if n > 1 && !exists<PerBlockRandomness>(@velor_framework);
    }

    spec safe_add_mod_for_verification(a: u256, b: u256, m: u256): u256 {
        aborts_if m < b;
        aborts_if a < m - b && a + b > MAX_U256;
        ensures result == spec_safe_add_mod(a, b, m);
    }

    spec fun spec_safe_add_mod(a: u256, b: u256, m: u256): u256 {
        if (a < m - b) {
            a + b
        } else {
            a - (m - b)
        }
    }
}
