module aptos_framework::nonce_validation {
    use aptos_framework::account;
    use aptos_std::smart_table::{Self, SmartTable};
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_validation;

    struct NonceKey has copy, drop, store {
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    }

    struct NonceHistory has key {
        // Key = (sender address, nonce), Value = bool (always set to true).
        table_1: SmartTable<NonceKey, bool>,
        // table_2: SmartTable<NonceKey, bool>,
        // Either 1 or 2
        current_table: u64,
    }

    struct NonceHistorySignerCap has key {
        signer_cap: account::SignerCapability,
    }


    public(friend) fun initialize(aptos_framework: &signer) {
        // let table_1 = smart_table::new();
        // let table_1 = smart_table::new_with_config(2000000, 75, 50);
        let table_1 = smart_table::new_with_config(100000, 75, 5);
        let nonce_history = NonceHistory {
            table_1,
            // table_2,
            current_table: 1,
        };

        move_to<NonceHistory>(aptos_framework, nonce_history);
    }

    public(friend) fun switch_table() acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        nonce_history.current_table = 3 - nonce_history.current_table;
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
        // if (nonce_history.current_table == 1) {
            smart_table::upsert(&mut nonce_history.table_1, nonce_key, true);
        // } else {
        //     smart_table::upsert(&mut nonce_history.table_2, nonce_key, true);
        // };
    }

    public(friend) fun nonce_exists(
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ): bool acquires NonceHistory {
        let nonce_history = borrow_global<NonceHistory>(@aptos_framework);
        let nonce_key = NonceKey {
            sender_address,
            nonce,
            txn_expiration_time,
        };
        if (smart_table::contains(&nonce_history.table_1, nonce_key)) {
            return true
        };
        // if (smart_table::contains(&nonce_history.table_2, nonce_key)) {
        //     return true
        // };
        false
    }
}
