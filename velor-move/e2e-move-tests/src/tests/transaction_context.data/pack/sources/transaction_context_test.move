module admin::transaction_context_test {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use velor_std::from_bcs;
    use velor_std::type_info;
    use velor_framework::transaction_context;
    use velor_framework::multisig_account;

    /// Since tests in e2e-move-tests/ can only call entry functions which don't have return values, we must store
    /// the results we are interested in inside this (rather-artificial) resource, which we can read back in our
    /// e2e-move-tests/ test.
    struct TransactionContextStore has key {
        sender: address,
        secondary_signers: vector<address>,
        gas_payer: address,
        max_gas_amount: u64,
        gas_unit_price: u64,
        chain_id: u8,
        account_address: address,
        module_name: String,
        function_name: String,
        type_arg_names: vector<String>,
        args: vector<vector<u8>>,
        multisig_address: address,
        // Fields for monotonically increasing counter tests
        counter_values: vector<u128>,
        counter_timestamps: vector<u64>,
        counter_call_count: u64,
    }

    /// Called when the module is first deployed at address `signer`, which is supposed to be @admin (= 0x1).
    fun init_module(sender: &signer) {
        assert!(signer::address_of(sender) == @admin, 1);
        // Initialize the global resource with the default values.
        move_to(sender,
            TransactionContextStore {
                sender: @0x0,
                secondary_signers: vector[],
                gas_payer: @0x0,
                max_gas_amount: 0,
                gas_unit_price: 0,
                chain_id: 0,
                account_address: @0x0,
                module_name: string::utf8(x""),
                function_name: string::utf8(x""),
                args: vector[],
                type_arg_names: vector[],
                multisig_address: @0x0,
                counter_values: vector[],
                counter_timestamps: vector[],
                counter_call_count: 0,
            }
        );
    }

    public entry fun store_sender_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.sender = transaction_context::sender();
    }

    public entry fun store_secondary_signers_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.secondary_signers = transaction_context::secondary_signers();
    }

    public entry fun store_secondary_signers_from_native_txn_context_multi(
        _s: &signer,
        _s2: &signer
    ) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.secondary_signers = transaction_context::secondary_signers();
    }

    public entry fun store_gas_payer_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.gas_payer = transaction_context::gas_payer();
    }

    public entry fun store_max_gas_amount_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.max_gas_amount = transaction_context::max_gas_amount();
    }

    public entry fun store_gas_unit_price_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.gas_unit_price = transaction_context::gas_unit_price();
    }

    public entry fun store_chain_id_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.chain_id = transaction_context::chain_id();
    }

    entry fun store_entry_function_payload_from_native_txn_context<T1, T2, T3>(
        _s: &signer,
        arg0: u64,
        arg1: bool
    ) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        let payload_opt = transaction_context::entry_function_payload();
        if (option::is_some(&payload_opt)) {
            let payload = option::borrow(&payload_opt);
            store.account_address = transaction_context::account_address(payload);
            store.module_name = transaction_context::module_name(payload);
            store.function_name = transaction_context::function_name(payload);
            store.type_arg_names = transaction_context::type_arg_names(payload);
            store.args = transaction_context::args(payload);

            // Check that the arguments are correct and can be parsed using `from_bcs`.
            assert!(arg0 == from_bcs::to_u64(*vector::borrow(&store.args, 0)), 11);
            assert!(arg1 == from_bcs::to_bool(*vector::borrow(&store.args, 1)), 12);
            // Check that the type argument names are correct and matched to `type_info::type_name`.
            assert!(
                store.type_arg_names == vector[type_info::type_name<T1>(), type_info::type_name<T2>(
                ), type_info::type_name<T3>()],
                13
            );

            assert!(option::some(option::destroy_some(payload_opt)) == transaction_context::entry_function_payload(), 13);
        } else {
            assert!(option::none() == payload_opt, 14);
        }
    }

    entry fun store_multisig_payload_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        let multisig_opt = transaction_context::multisig_payload();
        if (option::is_some(&multisig_opt)) {
            let multisig = option::borrow(&multisig_opt);
            store.multisig_address = transaction_context::multisig_address(multisig);

            let entry_opt = transaction_context::inner_entry_function_payload(multisig);
            if (option::is_some(&entry_opt)) {
                let entry = option::borrow(&entry_opt);
                store.account_address = transaction_context::account_address(entry);
                store.module_name = transaction_context::module_name(entry);
                store.function_name = transaction_context::function_name(entry);
                store.type_arg_names = transaction_context::type_arg_names(entry);
                store.args = transaction_context::args(entry);
            };
            assert!(option::some(option::destroy_some(multisig_opt)) == transaction_context::multisig_payload(), 1);
        } else {
            assert!(option::none() == multisig_opt, 2);
        }
    }

    entry fun prepare_multisig_payload_test(s: &signer, payload: vector<u8>) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);

        let multisig_account = multisig_account::get_next_multisig_account_address(signer::address_of(s));
        multisig_account::create(s, 1, vector[], vector[]);
        multisig_account::create_transaction(s, multisig_account, payload);

        store.multisig_address = multisig_account;
    }

    // ===== Monotonically Increasing Counter Tests =====

    /// Test that stores a single counter value and verifies it's non-zero
    public entry fun store_monotonically_increasing_counter_single(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        let counter = transaction_context::monotonically_increasing_counter();

        // Store the counter value
        vector::push_back(&mut store.counter_values, counter);
        store.counter_call_count = store.counter_call_count + 1;

        // Verify counter is non-zero
        assert!(counter > 0, 100);
    }

    /// Test that stores multiple counter values in a single transaction and verifies they increase
    public entry fun store_monotonically_increasing_counter_multiple(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);

        // Get multiple counter values in the same transaction
        let counter1 = transaction_context::monotonically_increasing_counter();
        let counter2 = transaction_context::monotonically_increasing_counter();
        let counter3 = transaction_context::monotonically_increasing_counter();

        // Store all values
        vector::push_back(&mut store.counter_values, counter1);
        vector::push_back(&mut store.counter_values, counter2);
        vector::push_back(&mut store.counter_values, counter3);
        store.counter_call_count = store.counter_call_count + 3;

        // Verify they increase monotonically
        assert!(counter2 > counter1, 101);
        assert!(counter3 > counter2, 102);
        assert!(counter3 > counter1, 103);
    }

    /// Test that extracts the components of the counter to verify format
    public entry fun test_monotonically_increasing_counter_format(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);

        let counter_1 = transaction_context::monotonically_increasing_counter();

        // Extract components according to format:
        // `<reserved_byte (8 bits)> || timestamp_us (64 bits) || transaction_index (32 bits) || session_counter (8 bits) || local_counter (16 bits)`
        let local_counter_1 = (counter_1 & 0xFFFF) as u16;
        let session_counter_1 = ((counter_1 >> 16) & 0xFF) as u8;
        let transaction_index_1 = ((counter_1 >> 24) & 0xFFFFFFFF) as u32;
        let timestamp_us_1 = ((counter_1 >> 56) & 0xFFFFFFFFFFFFFFFF) as u64;
        let reserved_byte_1 = (counter_1 >> 120) as u8;

        vector::push_back(&mut store.counter_values, counter_1);
        vector::push_back(&mut store.counter_timestamps, timestamp_us_1);
        store.counter_call_count = store.counter_call_count + 1;

        let counter_2 = transaction_context::monotonically_increasing_counter();
        let local_counter_2 = (counter_2 & 0xFFFF) as u16;
        let session_counter_2 = ((counter_2 >> 16) & 0xFF) as u8;
        let transaction_index_2 = ((counter_2 >> 24) & 0xFFFFFFFF) as u32;
        let timestamp_us_2 = ((counter_2 >> 56) & 0xFFFFFFFFFFFFFFFF) as u64;
        let reserved_byte_2 = (counter_2 >> 120) as u8;

        vector::push_back(&mut store.counter_values, counter_2);
        vector::push_back(&mut store.counter_timestamps, timestamp_us_2);
        store.counter_call_count = store.counter_call_count + 1;

        // Verify format constraints
        assert!(reserved_byte_1 == reserved_byte_2, 110); // Reserved byte should be the same
        assert!(local_counter_2 > 0, 111); // Local counter should be > 0 after first call
        assert!(session_counter_1 == session_counter_2, 112); // Session counter should be the same
        assert!(transaction_index_1 == transaction_index_2, 113); // Transaction index should be the same after two calls
        assert!(timestamp_us_1 == timestamp_us_2, 114); // Timestamp should be the same within transaction
        assert!(local_counter_1 < local_counter_2, 115); // Local counter should increase monotonically
    }
}
