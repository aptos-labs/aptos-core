module admin::transaction_context_test {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_std::from_bcs;
    use aptos_std::type_info;
    use aptos_framework::transaction_context;
    use aptos_framework::multisig_account;

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
    }

    /// Called when the module is first deployed at address `signer`, which is supposed to be @admin.
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
}
