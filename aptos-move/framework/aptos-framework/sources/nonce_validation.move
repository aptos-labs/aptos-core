module aptos_framework::nonce_validation {
    use aptos_framework::account;
    use aptos_std::table::{Self, Table};
    use aptos_std::timestamp;
    use aptos_std::vector;
    use aptos_std::aptos_hash::sip_hash_from_value;
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    struct NonceKey has copy, drop, store {
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    }

    struct NonceHistory has key {
        // Key = hash(sender address, nonce, txn expiration), value = (lowest expiration time of the nonce keys, vector of nonces).
        nonce_table: Table<u64, (u64, vector<NonceKey>)>,
    }

    struct NonceHistorySignerCap has key {
        signer_cap: account::SignerCapability,
    }


    public(friend) fun initialize(aptos_framework: &signer) {
        let table = table::new();
        let nonce_history = NonceHistory {
            nonce_table: table,
        };
        move_to<NonceHistory>(aptos_framework, nonce_history);
    }

    public(friend) fun check_and_insert_nonce(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        let nonce_key = NonceKey {
            sender_address,
            nonce,
            txn_expiration_time,
        };
        let hash = sip_hash_from_value(&nonce_key);
        let index = hash % 200000;
        if (!table::contains(&nonce_history.nonce_table, index)) {
            table::add(&mut nonce_history.nonce_table, index, vector::empty());
        };
        let (lowest_expiration_time, bucket) = table::borrow_mut(&mut nonce_history.nonce_table, index);
        let current_time = timestamp::now_seconds();
        if lowest_expiration_time > current_time {
            if vector::contains(bucket, nonce_key) {
                return false;
            }
            vector::push_back(bucket, nonce_key);
            *lowest_expiration_time = min(lowest_expiration_time, txn_expiration_time);
            return true;
        } else {
            let new_bucket = vector::empty();
            let len = vector::length(bucket);
            let i = 0;
            let lowest_expiration_time = 
            while (i < len) {
                let nonce_key = vector::borrow(bucket, i);
                if nonce_key.txn_expiration_time > current_time {
                    vector::push_back(new_bucket, key);
                }
                i = i + 1;
            }
        }
    }


    public(friend) fun insert_nonce(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ) acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        let nonce_key = NonceKey {
            sender_address,
            nonce,
            txn_expiration_time,
        };
        let hash = sip_hash_from_value(&nonce_key);
        let index = hash % 200000;
        if (!table::contains(&nonce_history.nonce_table, index)) {
            table::add(&mut nonce_history.nonce_table, index, vector::empty());
        };
        vector::push_back(table::borrow_mut(&mut nonce_history.nonce_table, index), nonce_key);
    }

    public(friend) fun nonce_exists(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_key = NonceKey {
            sender_address,
            nonce,
            txn_expiration_time,
        };
        let hash = sip_hash_from_value(&nonce_key);
        let index = hash % 200000;
        let nonce_history = borrow_global<NonceHistory>(@aptos_framework);
        if (table::contains(&nonce_history.nonce_table, index)) {
            if (vector::contains(table::borrow(&nonce_history.nonce_table, index), &nonce_key)) {
                return true;
            }
        };
        false
    }
}
