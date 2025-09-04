module 0xCAFE::scheduled_txns_usage {
    use std::signer;
    use aptos_std::debug;
    use std::string;
    use std::vector;
    use aptos_framework::scheduled_txns::{ScheduledTxnAuthToken, ScheduleMapKey, new_scheduled_transaction_no_signer,
        cancel_with_key, insert, new_scheduled_transaction_gen_auth_token, new_scheduled_transaction_reuse_auth_token};
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::timestamp;

    struct State has copy, store, drop {
        value: u64
    }

    /// Resource to store the scheduled transaction keys
    struct StoredScheduledTxns has key {
        txns: BigOrderedMap<ScheduleMapKey, ScheduledTransactionInfo>
    }

    struct ScheduledTransactionInfo has copy, drop, store {
        sender_addr: address,
        max_gas_amount: u64,
        gas_unit_price: u64
    }

    struct ScheduledTransactionInfoWithKey has copy, drop, store {
        sender_addr: address,
        max_gas_amount: u64,
        gas_unit_price: u64,
        block_timestamp_ms: u64,
        key: ScheduleMapKey
    }

    #[persistent]
    fun step(state: State) {
        debug::print(&string::utf8(b"Move: in the func step with val"));
        debug::print(&state.value);
        if (state.value < 10) {
            state.value = state.value + 1;
        }
    }

    #[persistent]
    fun user_func_abort() {
        debug::print(&string::utf8(b"Move: in the user_func_abort"));
        assert!(false, 0x1);
    }

    #[persistent]
    fun user_func_mod_publish(s: &signer, _auth_token: ScheduledTxnAuthToken) {
        use aptos_framework::code;

        debug::print(&string::utf8(b"Move: in the user_func_mod_publish"));

        // First call to publish_package_txn - this should succeed
        let metadata: vector<u8> = vector[
            7, 80, 97, 99, 107, 97, 103, 101, 1, 0, 0, 0, 0, 0, 0, 0, 0, 64, 68, 56, 49,
            69, 55, 68, 70, 69, 70, 54, 51, 52, 66, 50, 56, 56, 49, 69, 48, 48, 51, 69, 67,
            70, 49, 54, 66, 54, 66, 69, 53, 53, 66, 69, 57, 49, 54, 54, 55, 53, 65, 65, 66,
            66, 50, 67, 57, 52, 70, 55, 56, 52, 54, 67, 56, 70, 57, 55, 68, 49, 50, 57, 54,
            65, 107, 31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 37, 138, 203, 9, 128, 48, 16, 68,
            239, 91, 133, 164, 0, 177, 1, 123, 240, 30, 68, 214, 236, 32, 193, 124, 150,
            68, 5, 187, 55, 65, 230, 52, 239, 61, 171, 236, 78, 62, 176, 82, 226, 136, 97,
            30, 204, 242, 3, 67, 15, 74, 245, 57, 117, 54, 141, 109, 134, 110, 61, 10, 11,
            54, 205, 193, 187, 183, 11, 151, 163, 242, 229, 247, 208, 122, 203, 34, 5, 181,
            162, 174, 68, 86, 160, 72, 130, 228, 124, 255, 31, 84, 65, 171, 55, 103, 0, 0,
            0, 1, 1, 109, 0, 0, 0, 0, 0
        ];
        let code: vector<vector<u8>> = vector[
            vector[
                161, 28, 235, 11, 7, 0, 0, 10, 6, 1, 0, 2, 3, 2, 6, 5, 8, 1, 7, 9, 4, 8,
                13, 32, 12, 45, 7, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 109, 1, 102, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 202, 254, 0, 1, 0, 0, 0, 1, 2, 0
            ]
        ];

        code::publish_package_txn(s, metadata, code);

        debug::print(
            &string::utf8(
                b"Move: publish_package_txn succeeded - this should not be reached"
            )
        );
    }

    #[persistent]
    fun user_func_to_reschedule(
        sender: &signer,
        auth_token: ScheduledTxnAuthToken
    ) {
        debug::print(&string::utf8(b"Trying to reschedule txn"));
        let next_schedule_time = timestamp::now_microseconds() / 1000 + 1000; // schedule 1 second later

        let foo = |signer: &signer, auth_token: ScheduledTxnAuthToken| user_func_to_reschedule(signer, auth_token);

        let txn = new_scheduled_transaction_reuse_auth_token(
            sender,
            auth_token,
            next_schedule_time,
            1000,
            200,
            foo
        );

        debug::print(&string::utf8(b"Trying to reschedule txn 2"));

        insert(sender, txn);

        debug::print(&string::utf8(b"Trying to reschedule txn 3"));
    }

    public entry fun create_and_add_transactions(
        user: &signer,
        current_time_ms: u64,
        values: vector<u64>,
        gas_amounts: vector<u64>,
        gas_prices: vector<u64>
    ) {
        let user_addr = signer::address_of(user);
        let num_txns = values.length();

        assert!(gas_amounts.length() == num_txns, 0x1);
        assert!(gas_prices.length() == num_txns, 0x2);

        let txn_map =
            big_ordered_map::new<ScheduleMapKey, ScheduledTransactionInfo>();

        let i = 0;
        while (i < num_txns) {
            let state = State { value: values[i] };
            let foo = || step(state);
            let txn_time = current_time_ms + 1500 * (i + 1);
            let gas_unit_price = gas_prices[i];
            let txn =
                new_scheduled_transaction_no_signer(
                    user_addr,
                    txn_time,
                    gas_amounts[i],
                    gas_unit_price,
                    foo
                );

            let key = insert(user, txn);

            let txn_info = ScheduledTransactionInfo {
                sender_addr: user_addr,
                max_gas_amount: gas_amounts[i],
                gas_unit_price: gas_prices[i]
            };
            txn_map.add(key, txn_info);

            i = i + 1;
        };

        // Store the transaction infos in StoredScheduledTxns
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }

    public entry fun add_txn_with_user_func_abort(
        user: &signer, current_time_ms: u64
    ) {
        let gas_amount = 10000;
        let gas_unit_price = 200;
        let user_addr = signer::address_of(user);

        let foo_abort = || user_func_abort();
        let txn_time = current_time_ms + 1500;

        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                txn_time,
                gas_amount,
                gas_unit_price,
                foo_abort
            );

        let key = insert(user, txn);

        // Create a ScheduledTransactionInfo and store it
        let txn_info = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: gas_amount,
            gas_unit_price
        };

        // Store the transaction info in StoredScheduledTxns
        let txn_map =
            big_ordered_map::new<ScheduleMapKey, ScheduledTransactionInfo>();
        txn_map.add(key, txn_info);
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }

    public entry fun cancel_txn(user: &signer) acquires StoredScheduledTxns {
        // Get the first key from the BigOrderedMap
        let user_addr = signer::address_of(user);
        let stored_txns = borrow_global<StoredScheduledTxns>(user_addr);
        let (first_key, _value) = stored_txns.txns.borrow_front();

        // Cancel the scheduled transaction using the first key
        cancel_with_key(user, first_key);
    }

    public entry fun create_and_add_module_pub_txn(
        user: &signer, current_time_ms: u64
    ) {
        // Create a scheduled transaction for the double publish test
        let user_addr = signer::address_of(user);
        let schedule_time = current_time_ms + 1000; // Schedule 1 second later
        let gas_amount = 1000;
        let gas_unit_price = 100;
        let foo_module_pub = |s: &signer, auth_token: ScheduledTxnAuthToken| user_func_mod_publish(s, auth_token);

        let txn =
            new_scheduled_transaction_gen_auth_token(
                user,
                schedule_time,
                gas_amount,
                gas_unit_price,
                foo_module_pub
            );
        let key = insert(user, txn);

        let txn_info = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: gas_amount,
            gas_unit_price: gas_unit_price
        };
        let txn_map =
            big_ordered_map::new<ScheduleMapKey, ScheduledTransactionInfo>();
        big_ordered_map::add(&mut txn_map, key, txn_info);
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }

    /// Generic function to create and add scheduled transactions with custom scheduling functions
    /// Uses idx to select from predefined array of functions
    ///
    /// Function index mapping:
    /// - 0: user_func_mod_publish (module publish)
    /// - 1: user_func_xyz (placeholder - define this later)
    /// - 2: custom_function_2 (placeholder - define this later)
    /// - 3: custom_function_3 (placeholder - define this later)
    /// - default: user_func_mod_publish
    public entry fun create_and_add_custom_txn_template(
        user: &signer,
        current_time_ms: u64,
        delay_ms: u64,
        gas_amount: u64,
        gas_unit_price: u64,
        idx: u64  // Index to select which function to use
    ) {
        let user_addr = signer::address_of(user);
        let schedule_time = current_time_ms + delay_ms;

        // Create the appropriate lambda based on index
        let txn = if (idx == 1) {
            // Placeholder for user_func_xyz - you can define this later
            let user_function = |s: &signer, auth_token: ScheduledTxnAuthToken| user_func_to_reschedule(s, auth_token);  // Replace with actual function
            new_scheduled_transaction_gen_auth_token(
                user,
                schedule_time,
                gas_amount,
                gas_unit_price,
                user_function
            )
        } else {
            // Placeholder for another custom function
            let user_function = |s: &signer, auth_token: ScheduledTxnAuthToken| user_func_mod_publish(s, auth_token);  // Replace with actual function
            new_scheduled_transaction_gen_auth_token(
                user,
                schedule_time,
                gas_amount,
                gas_unit_price,
                user_function
            )
        };

        let key = insert(user, txn);

        let txn_info = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: gas_amount,
            gas_unit_price
        };
        let txn_map =
            big_ordered_map::new<ScheduleMapKey, ScheduledTransactionInfo>();
        txn_map.add(key, txn_info);
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }


    #[view]
    public fun get_stored_sched_txns(
        addr: address, block_timestamp_ms: u64
    ): vector<ScheduledTransactionInfoWithKey> acquires StoredScheduledTxns {
        let stored_txns = borrow_global<StoredScheduledTxns>(addr);
        let result = vector::empty<ScheduledTransactionInfoWithKey>();

        stored_txns.txns.for_each_ref(
            |key, value| {
                let txn_info_with_key =
                    ScheduledTransactionInfoWithKey {
                        sender_addr: value.sender_addr,
                        max_gas_amount: value.max_gas_amount,
                        gas_unit_price: value.gas_unit_price,
                        block_timestamp_ms,
                        key: *key
                    };
                result.push_back(txn_info_with_key);
            }
        );

        result
    }
}
