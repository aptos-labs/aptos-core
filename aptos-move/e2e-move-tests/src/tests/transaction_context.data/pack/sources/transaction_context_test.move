module admin::transaction_context_test {
    use std::signer;
    use aptos_framework::transaction_context;

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
            }
        );
    }

    public entry fun store_sender_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.sender =  transaction_context::sender();
    }

    public entry fun store_secondary_signers_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.secondary_signers =  transaction_context::secondary_signers();
    }

    public entry fun store_secondary_signers_from_native_txn_context_multi(_s: &signer, _s2: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.secondary_signers =  transaction_context::secondary_signers();
    }

    public entry fun store_gas_payer_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.gas_payer =  transaction_context::gas_payer();
    }

    public entry fun store_max_gas_amount_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.max_gas_amount =  transaction_context::max_gas_amount();
    }

    public entry fun store_gas_unit_price_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.gas_unit_price =  transaction_context::gas_unit_price();
    }

    public entry fun store_chain_id_from_native_txn_context(_s: &signer) acquires TransactionContextStore {
        let store = borrow_global_mut<TransactionContextStore>(@admin);
        store.chain_id =  transaction_context::chain_id();
    }
}
