spec aptos_framework::nonce_validation {
    /// Creates the empty nonce history unless it exists.
    spec initialize_nonce_table(aptos_framework: &signer) {
        use std::signer;
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        ensures exists<NonceHistory>(@aptos_framework);
        ensures old(exists<NonceHistory>(@aptos_framework)) ==>
            global<NonceHistory>(@aptos_framework) == old(global<NonceHistory>(@aptos_framework));
        ensures !old(exists<NonceHistory>(@aptos_framework)) ==>
            global<NonceHistory>(@aptos_framework).next_key == 0;
    }

    spec empty_bucket(pre_allocate_slots: bool): Bucket {
        // Partial: aborts of the slot pre-allocation are not modelled.
        pragma aborts_if_is_partial;
        ensures big_ordered_map::spec_len(result.nonces_ordered_by_exp_time) == 0;
        ensures big_ordered_map::spec_len(result.nonce_to_exp_time_map) == 0;
    }

    spec add_nonce_buckets(count: u64) {
        // Partial: aborts of the slot pre-allocation are not modelled.
        pragma aborts_if_is_partial;
        aborts_if !exists<NonceHistory>(@aptos_framework);
        modifies global<NonceHistory>(@aptos_framework);
        ensures exists<NonceHistory>(@aptos_framework);
    }

    /// Returns false iff `(sender_address, nonce)` is in use; otherwise records it with `txn_expiration_time`.
    spec check_and_insert_nonce(sender_address: address, nonce: u64, txn_expiration_time: u64): bool {
        // Partial: the aborts of the bucket map operations are not specified.
        pragma aborts_if_is_partial;
        aborts_if !exists<NonceHistory>(@aptos_framework);
        aborts_if txn_expiration_time
            > timestamp::spec_now_seconds() + NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECONDS;
        ensures result == !old(spec_is_nonce_in_use(sender_address, nonce, txn_expiration_time));
        ensures result ==> spec_has_nonce(sender_address, nonce)
            && spec_nonce_expiration_time(sender_address, nonce) == txn_expiration_time;
    }

    /// The bucket map of the nonce history that `(sender_address, nonce)` is kept in.
    spec fun spec_bucket_map(sender_address: address, nonce: u64): BigOrderedMap<NonceKey, u64> {
        use std::bcs;
        use aptos_std::aptos_hash;
        let index = aptos_hash::spec_sip_hash(bcs::serialize(NonceKey { sender_address, nonce })) % NUM_BUCKETS;
        table::spec_get(global<NonceHistory>(@aptos_framework).nonce_table, index).nonce_to_exp_time_map
    }

    spec fun spec_has_nonce(sender_address: address, nonce: u64): bool {
        use std::bcs;
        use aptos_std::aptos_hash;
        let index = aptos_hash::spec_sip_hash(bcs::serialize(NonceKey { sender_address, nonce })) % NUM_BUCKETS;
        table::spec_contains(global<NonceHistory>(@aptos_framework).nonce_table, index)
            && big_ordered_map::spec_contains_key(
                spec_bucket_map(sender_address, nonce), NonceKey { sender_address, nonce }
            )
    }

    spec fun spec_nonce_expiration_time(sender_address: address, nonce: u64): u64 {
        big_ordered_map::spec_get(spec_bucket_map(sender_address, nonce), NonceKey { sender_address, nonce })
    }

    /// The pair is kept with an expiration time that is not over, or that is within the overlap interval of
    /// `txn_expiration_time`.
    spec fun spec_is_nonce_in_use(sender_address: address, nonce: u64, txn_expiration_time: u64): bool {
        let exp = spec_nonce_expiration_time(sender_address, nonce);
        spec_has_nonce(sender_address, nonce) && (exp >= timestamp::spec_now_seconds()
            || txn_expiration_time <= exp + NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECONDS)
    }
}
