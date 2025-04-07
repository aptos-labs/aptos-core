module aptos_framework::nonce_validation {
    use aptos_std::table::{Self, Table};
    use aptos_std::timestamp;
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};
    use aptos_std::aptos_hash::sip_hash_from_value;
    use aptos_std::option;
    use aptos_std::vector;
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    const NUM_BUCKETS: u64 = 50000;
    const MAP_SWITCH_TIME: u64 = 75;
    const MAX_EXPIRATION_TIME: u64 = 60;

    struct Bucket has store {
        // Maps NonceKey = (address, nonce) to txn expiration time.
        // Using multiple maps to facilitate easier garbage collection.
        nonces: vector<BigOrderedMap<NonceKey, u64>>,
        // last_stored_times[i] stores the timestamp at which a nonce entry 
        // is last added to the nonces[i] map.
        last_stored_times: vector<u64>,
    }

    struct NonceKey has copy, drop, store {
        sender_address: address,
        nonce: u64,
    }

    struct NonceHistory has key {
        // Key = sip_hash(NonceKey) % NUM_BUCKETS
        // Value = Bucket
        nonce_table: Table<u64, Bucket>,
        // Used to facilitate prefill the nonce_table with empty buckets
        // one by one using `add_nonce_bucket` method.
        // This is the next_key to prefill with an empty bucket
        next_key: u64,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        initialize_nonce_table(aptos_framework);
    }

    public entry fun initialize_nonce_table(aptos_framework: &signer) {
        if (!exists<NonceHistory>(@aptos_framework)) {
            let table = table::new();
            let nonce_history = NonceHistory {
                nonce_table: table,
                next_key: 0,
            };
            move_to<NonceHistory>(aptos_framework, nonce_history);
        };
    }

    // This method is used to prefill the nonce_table with empty buckets one by one.
    public entry fun add_nonce_bucket(aptos_framework: &signer) acquires NonceHistory {
        if (exists<NonceHistory>(@aptos_framework)) {
            let current_time = timestamp::now_seconds();
            let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
            if (!table::contains(&nonce_history.nonce_table, nonce_history.next_key)) {
                let nonces = vector::empty();
                let last_stored_times = vector::empty();
                vector::push_back(&mut nonces, big_ordered_map::new());
                vector::push_back(&mut nonces, big_ordered_map::new());
                vector::push_back(&mut last_stored_times, 0);
                vector::push_back(&mut last_stored_times, 0);
                table::add(&mut nonce_history.nonce_table, nonce_history.next_key, Bucket {
                    nonces: nonces,
                    last_stored_times: last_stored_times,
                });
            };
            nonce_history.next_key = nonce_history.next_key + 1;
        };
    }

    // Returns true if the input (address, nonce) pair doesn't exist in the nonce history, and inserted into nonce history successfully.
    // Returns false if the input (address, nonce) pair already exists in the nonce history.
    public(friend) fun check_and_insert_nonce(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        let nonce_key = NonceKey {
            sender_address,
            nonce,
        };
        let index = sip_hash_from_value(&nonce_key) % NUM_BUCKETS;
        let map_index = (txn_expiration_time / MAP_SWITCH_TIME) % 2;
        let current_time = timestamp::now_seconds();
        if (!table::contains(&nonce_history.nonce_table, index)) {
            let nonces = vector::empty();
            let last_stored_times = vector::empty();
            vector::push_back(&mut nonces, big_ordered_map::new());
            vector::push_back(&mut nonces, big_ordered_map::new());
            vector::push_back(&mut last_stored_times, 0);
            vector::push_back(&mut last_stored_times, 0);
            big_ordered_map::add(&mut nonces[map_index], nonce_key, txn_expiration_time);
            last_stored_times[map_index] = current_time;
            table::add(&mut nonce_history.nonce_table, index, Bucket {
                nonces: nonces,
                last_stored_times: last_stored_times,
            });
            return true;
        };
        let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
        // Question[Orderless]: As this is an expensive operation, didn't do it for both buckets.
        // Should we do this for both buckets? 
        if (bucket.last_stored_times[1-map_index] + MAX_EXPIRATION_TIME < current_time) {
            while (!big_ordered_map::is_empty(&bucket.nonces[1-map_index])) {
                bucket.nonces[1-map_index].pop_back();
            }
        };
        let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
        if (big_ordered_map::contains(&bucket.nonces[1-map_index], &nonce_key)) {
            return false
        };
        if (option::is_some(&big_ordered_map::upsert(&mut bucket.nonces[map_index], nonce_key, txn_expiration_time))) {
            return false
        };
        bucket.last_stored_times[map_index] = current_time;
        true
    }

    // Returns true if the input (address, nonce) pair doesn't exist in the nonce history.
    // Returns false if the input (address, nonce) pair already exists in the nonce history.
    public(friend) fun check_nonce(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_key = NonceKey {
            sender_address,
            nonce,
        };
        let index = sip_hash_from_value(&nonce_key) % NUM_BUCKETS;
        let nonce_history = borrow_global<NonceHistory>(@aptos_framework);
        if (table::contains(&nonce_history.nonce_table, index)) {
            let bucket = table::borrow(&nonce_history.nonce_table, index);
            if (big_ordered_map::contains(&bucket.nonces[0], &nonce_key)) {
                return false
            };
            if (big_ordered_map::contains(&bucket.nonces[1], &nonce_key)) {
                return false
            };
        };
        true
    }
}
