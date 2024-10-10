module aptos_framework::nonce_validation {
    use aptos_framework::account;
    use aptos_std::table::{Self, Table};
    use aptos_std::vector;
    use aptos_std::aptos_hash::sip_hash_from_value;
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    struct NonceKey has copy, drop, store {
        sender: address,
        nonce: u64,
    }

    struct NonceHistory has key {
        // Bucket Index = hash(NonceKey) % 100k.
        // Bucket = OrderedMap<NonceKey, transaction_expiration_time>
        table_1: Table<u64, OrderedMap<NonceKey, u64>>,
    }

    struct NonceHistorySignerCap has key {
        signer_cap: account::SignerCapability,
    }


    public(friend) fun initialize(aptos_framework: &signer) {
        let table_1 = table::new();
        let nonce_history = NonceHistory {
            table_1,
        };

        move_to<NonceHistory>(aptos_framework, nonce_history);
    }

    public(friend) fun insert_nonce(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        let nonce_key = NonceKey {
            sender_address,
            nonce,
        };
        let hash = sip_hash_from_value(&nonce_key);
        let index = hash % 100000;
        if (!table::contains(&nonce_history.table_1, index)) {
            table::add(&mut nonce_history.table_1, index, ordered_map::new());
        };
        ordered_map::add(table::borrow_mut(&mut nonce_history.table_1, index), nonce_key, txn_expiration_time)
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
        let index = hash % 100000;
        let nonce_history = borrow_global<NonceHistory>(@aptos_framework);
        if (table::contains(&nonce_history.table_1, index)) {
            if (vector::contains(table::borrow(&nonce_history.table_1, index), &nonce_key)) {
                return true;
            }
        };
        false
    }
}
