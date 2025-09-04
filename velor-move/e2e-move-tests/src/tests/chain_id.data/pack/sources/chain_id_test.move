module 0x1::chain_id_test {
    use velor_std::type_info;
    use velor_framework::velor_governance;
    use velor_framework::chain_id;
    use std::features;

    /// Since tests in e2e-move-tests/ can only call entry functions which don't have return values, we must store
    /// the results we are interested in (i.e., the chain ID) inside this (rather-artificial) resource, which we can
    /// read back in our e2e-move-tests/ test.
    struct ChainIdStore has key {
        id: u8,
    }

    /// Called when the module is first deployed at address `signer`, which is set to 0x1 (according to the `module 0x1::chain_id_test` line above).
    fun init_module(sender: &signer) {
        move_to(sender,
            ChainIdStore {
                id: 0u8
            }
        );

        features::change_feature_flags_for_next_epoch(sender, vector[features::get_velor_stdlib_chain_id_feature()], vector[]);
        velor_governance::force_end_epoch(sender);
    }

    /// Fetches the chain ID (via velor_framework::chain_id::get()) and stores it in the ChainIdStore resource.
    public entry fun store_chain_id_from_velor_framework(_s: &signer) acquires ChainIdStore {
        let store = borrow_global_mut<ChainIdStore>(@0x1);
        store.id = chain_id::get();
    }

    /// Fetches the chain ID (via the NativeTransactionContext) and stores it in the ChainIdStore resource.
    public entry fun store_chain_id_from_native_txn_context(_s: &signer) acquires ChainIdStore {
        let store = borrow_global_mut<ChainIdStore>(@0x1);

        store.id =  type_info::chain_id();
    }
}
