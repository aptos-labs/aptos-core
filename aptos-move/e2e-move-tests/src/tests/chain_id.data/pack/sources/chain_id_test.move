module 0x1::chain_id_test {
    //use aptos_std::type_info;
    use aptos_framework::chain_id;
    use std::option;
    use aptos_std::aptos_hash;

    /// Since tests in e2e-move-tests/ can only call entry functions which don't have return values, we must store
    /// the results we are interested in (i.e., the chain ID) inside this (rather-artificial) resource, which we can
    /// read back in our e2e-move-tests/ test.
    struct ChainIdStore has key {
        id: u8,
    }

    fun init_module(sender: &signer) {
        move_to(sender,
            ChainIdStore {
                id: 0u8
            }
        )
    }

    /// Fetches the chain ID (via aptos_framework::chain_id::get()) and stores it in the ChainIdStore resource.
    public entry fun store_chain_id_from_aptos_framework(_s: &signer) acquires ChainIdStore {
        let store = borrow_global_mut<ChainIdStore>(@0x1);
        store.id = chain_id::get();
    }

    /// Fetches the chain ID (via the NativeTransactionContext) and stores it in the ChainIdStore resource.
    public entry fun store_chain_id_from_native_txn_context(_s: &signer) acquires ChainIdStore {
        let store = borrow_global_mut<ChainIdStore>(@0x1);

        aptos_hash::testing_smth();

        let opt = option::none<u8>(); // type_info::chain_id();

        assert!(option::is_some(&opt), 1);

        store.id = option::extract(&mut opt);
    }
}
