module aptos_framework::nonce_validation_bi_big_ordered_map {
    use aptos_std::table::{Self, Table};
    use aptos_std::timestamp;
    use aptos_std::math64::min;
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
        nonces: vector<BigOrderedMap<NonceKey, u64>>,
        last_stored_times: vector<u64>,
    }

    struct NonceKey has copy, drop, store {
        sender_address: address,
        nonce: u64,
    }

    struct NonceHistory has key {
        // Key = sip_hash(NonceKey) % 200000
        // Value = Bucket { lowest_expiration_time: u64, nonces: vector<NonceEntry> }
        nonce_table: Table<u64, Bucket>,
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

    // Adds a new nonce bucket to the nonce history
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

    // returns true if the nonce is valid and inserted into nonce table successfully
    // returns false if the nonce is duplicate
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
        let map_index = (txn_expiration_time /MAP_SWITCH_TIME) % 2;
        if (!table::contains(&nonce_history.nonce_table, index)) {
            let nonces = vector::empty();
            let last_stored_times = vector::empty();
            vector::push_back(&mut nonces, big_ordered_map::new());
            vector::push_back(&mut nonces, big_ordered_map::new());
            vector::push_back(&mut last_stored_times, timestamp::now_seconds());
            vector::push_back(&mut last_stored_times, timestamp::now_seconds());
            big_ordered_map::add(&mut nonces[map_index], nonce_key, txn_expiration_time);
            table::add(&mut nonce_history.nonce_table, index, Bucket {
                nonces: nonces,
                last_stored_times: last_stored_times,
            });
            return true;
        };
        let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
        let current_time = timestamp::now_seconds();
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

    // returns true if the nonce is valid (not present in the table)
    // returns false if the nonce is duplicate
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
