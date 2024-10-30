module aptos_framework::nonce_validation_big_ordered_map {
    use aptos_std::table::{Self, Table};
    use aptos_std::timestamp;
    use aptos_std::math64::min;
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};
    use aptos_std::aptos_hash::sip_hash_from_value;
    use aptos_std::option;
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    struct Bucket has store {
        // TODO[Orderless]: Caching the lowest_expiration_time of all the nonces in the bucket. If the lowest_expiration_time is
        // below the current time, then some nonces in the bucket are expired, and we need to remove them.
        // We could avoid caching the lowest_expiration_time and just iterate over all the nonces in the bucket as well.
        // Do some benchmarking to compare the time complexity of both approaches.
        lowest_expiration_time: u64,
        nonces: BigOrderedMap<NonceKey, u64>,
    }

    struct NonceKey has drop, copy, store {
        sender_address: address,
        nonce: u64,
    }

    struct NonceHistory has key {
        // Key = sip_hash(NonceKey) % 50000
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
            // Question[Orderless]: We need to prefill this table in the beginning, so that we pay for the intial storage cost
            // I'm not sure what's the best way to initialize. If we initialize the table here, will it be only executed
            // in genesis? If this function is executed only in genesis, then will it run on mainnet when we release this feature?
            move_to<NonceHistory>(aptos_framework, nonce_history);
        };
    }

    public entry fun add_nonce_bucket() acquires NonceHistory {
        if (exists<NonceHistory>(@aptos_framework)) {
            let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
            if (!table::contains(&nonce_history.nonce_table, nonce_history.next_key)) {
                // Question[Orderless]: Should we add some dummy entries as well?
                table::add(&mut nonce_history.nonce_table, nonce_history.next_key, Bucket {
                    lowest_expiration_time: timestamp::now_seconds(),
                    nonces: big_ordered_map::new(),
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
        let index = sip_hash_from_value(&nonce_key) % 50000;
        if (!table::contains(&nonce_history.nonce_table, index)) {
            let nonces = big_ordered_map::new();
            big_ordered_map::upsert(&mut nonces, nonce_key, txn_expiration_time);
            table::add(&mut nonce_history.nonce_table, index, Bucket {
                lowest_expiration_time: txn_expiration_time,
                nonces: nonces,
            });
            return true
        };
        let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
        if (option::is_some(&big_ordered_map::upsert(&mut bucket.nonces, nonce_key, txn_expiration_time))) {
            return false
        };
        bucket.lowest_expiration_time = min(bucket.lowest_expiration_time, txn_expiration_time);
        // let current_time = timestamp::now_seconds();
        // if (current_time > bucket.lowest_expiration_time && big_ordered_map::length(&bucket.nonces) > 10) {
        //     // There is an expired nonce. Remove the expired nonces.
        //     let new_bucket = Bucket {
        //         lowest_expiration_time: txn_expiration_time,
        //         nonces: big_ordered_map::new(),
        //     };
        //     let i = 0;
        //     let len = big_ordered_map::length(&bucket.nonces);
        //     while (i < len) {
        //         let (cur_nonce_key, cur_txn_expiration_time) = big_ordered_map::pop_front(&mut bucket.nonces);
        //         if (current_time <= cur_txn_expiration_time) {
        //             big_ordered_map::add(&mut new_bucket.nonces, cur_nonce_key, cur_txn_expiration_time);
        //             new_bucket.lowest_expiration_time = min(new_bucket.lowest_expiration_time, cur_txn_expiration_time);
        //         };
        //         i = i + 1;
        //     };
        //     *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
        // };
        return true
    }


    // public(friend) fun insert_nonce(
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
    //     let nonce_entry = NonceEntry {
    //         sender_address,
    //         nonce,
    //         txn_expiration_time,
    //     };
    //     let hash = sip_hash_from_value(&nonce_entry);
    //     let index = hash % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         table::add(&mut nonce_history.nonce_table, index, vector::empty());
    //     };
    //     vector::push_back(table::borrow_mut(&mut nonce_history.nonce_table, index), nonce_entry);
    // }

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
        let hash = sip_hash_from_value(&nonce_key);
        let index = hash % 50000;
        let nonce_history = borrow_global<NonceHistory>(@aptos_framework);
        if (table::contains(&nonce_history.nonce_table, index)) {
            if (big_ordered_map::contains(&table::borrow(&nonce_history.nonce_table, index).nonces, &nonce_key)) {
                return false
            }
        };
        true
    }
}
