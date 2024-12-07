module aptos_framework::nonce_validation {
    use aptos_std::table::{Self, Table};
    use aptos_std::timestamp;
    use aptos_std::math64::min;
    use aptos_std::vector;
    use aptos_std::aptos_hash::sip_hash_from_value;
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    struct NonceEntry has copy, drop, store {
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    }

    struct Bucket has copy, drop, store {
        lowest_expiration_time: u64,
        nonces: vector<NonceEntry>,
    }

    struct NonceKey has drop {
        sender_address: address,
        nonce: u64,
    }

    struct NonceHistory has key {
        // Key = sip_hash(NonceKey) % 200000
        // Value = Bucket { lowest_expiration_time: u64, nonces: vector<NonceEntry> }
        nonce_table: Table<u64, Bucket>,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        let table = table::new();
        let nonce_history = NonceHistory {
            nonce_table: table,
        };
        move_to<NonceHistory>(aptos_framework, nonce_history);
    }

    // returns true if the nonce is valid and inserted into nonce table successfully
    // returns false if the nonce is duplicate
    public(friend) fun check_and_insert_nonce(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        let nonce_entry = NonceEntry {
            sender_address,
            nonce,
            txn_expiration_time,
        };
        let nonce_key = NonceKey {
            sender_address,
            nonce,
        };
        let index = sip_hash_from_value(&nonce_key) % 200000;
        if (!table::contains(&nonce_history.nonce_table, index)) {
            let nonces = vector::empty();
            vector::push_back(&mut nonces, nonce_entry);
            table::add(&mut nonce_history.nonce_table, index, Bucket {
                lowest_expiration_time: txn_expiration_time,
                nonces: nonces,
            });
            return true
        };
        let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
        if (vector::contains(&bucket.nonces, &nonce_entry)) {
            return false
        };
        let current_time = timestamp::now_seconds();
        if (current_time <= bucket.lowest_expiration_time) {
            // None of the nonces are expired. Just insert the nonce.
            vector::push_back(&mut bucket.nonces, nonce_entry);

            // Question: Is there a better way to do this?
            table::borrow_mut(&mut nonce_history.nonce_table, index).lowest_expiration_time = min(bucket.lowest_expiration_time, txn_expiration_time);
        } else {
            // There is an expired nonce. Remove the expired nonces.
            let new_bucket = Bucket {
                lowest_expiration_time: txn_expiration_time,
                nonces: vector::empty(),
            };
            let len = vector::length(&bucket.nonces);
            let i = 0;
            while (i < len) {
                let nonce_entry = vector::borrow(&bucket.nonces, i);
                if (current_time <= nonce_entry.txn_expiration_time) {
                    vector::push_back(&mut new_bucket.nonces, *nonce_entry);
                    new_bucket.lowest_expiration_time = min(new_bucket.lowest_expiration_time, nonce_entry.txn_expiration_time);
                };
                i = i + 1;
            };
            *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
        };
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

    // public(friend) fun nonce_exists(
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ): bool acquires NonceHistory {
    //     let nonce_entry = NonceEntry {
    //         sender_address,
    //         nonce,
    //         txn_expiration_time,
    //     };
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let hash = sip_hash_from_value(&nonce_key);
    //     let index = hash % 200000;
    //     let nonce_history = borrow_global<NonceHistory>(@aptos_framework);
    //     if (table::contains(&nonce_history.nonce_table, index)) {
    //         if (vector::contains(&table::borrow(&nonce_history.nonce_table, index).nonces, &nonce_entry)) {
    //             return true
    //         }
    //     };
    //     false
    // }
}
