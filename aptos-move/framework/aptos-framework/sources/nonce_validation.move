module aptos_framework::nonce_validation {
    use std::bcs;
    use std::error;
    use std::features;
    use std::signer;
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::chain_id;
    use aptos_framework::coin;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::transaction_fee;
    use aptos_framework::smart_table;
    use aptos_framework::transaction_validation::NonceHistorySignerCap;
    friend aptos_framework::genesis;


    struct NonceHistory has key {
        // Key = (sender address, nonce), Value = bool (always set to true).
        table_1: SmartTable<(address, u64), bool>,
        table_2: SmartTable<(address, u64), bool>,
        // Either 1 or 2
        current_table: u64,
        last_switched_time: u64,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        let table_1 = smart_table::new_with_config(5000, 75, 5);
        let table_2 = smart_table::new_with_config(5000, 75, 5);
        let nonce_history = NonceHistory {
            table_1,
            table_2,
            current_table: 1,
        };

        let (resource_account_signer, signer_cap) = account::create_resource_account(main_account, seed);
        let signer_cap_resource = NonceHistorySignerCap {
            signer_cap,
        };
        move_to<NonceHistory>(resource_account_signer, nonce_history);
        move_to<NonceHistorySignerCap>(aptos_framwork, signer_cap_resource);
    }

    public(friend) fun switch_table(aptos_framework: &signer) {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        nonce_history.current_table = 3 - nonce_history.current_table;
    }

    public(friend) fun insert_nonce(
        aptos_framework: &signer,
        sender_address: address,
        nonce: u64,
    ) {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        let table = if nonce_history.current_table == 1 {
            &mut nonce_history.table_1
        } else {
            &mut nonce_history.table_2
        };
        table.insert((address, nonce), true);
    }

    public(friend) fun check_nonce(
        aptos_framework: &signer,
        sender_address: address,
        nonce: u64,
    ): bool {
        let nonce_history = borrow_global_mut<NonceHistory>(@aptos_framework);
        if nonce_history.table1.contains_key((address, nonce)) {
            return true;
        }
        if nonce_history.table2.contains_key((address, nonce)) {
            return true;
        }
        return false;
    }
}
