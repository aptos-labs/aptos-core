spec aptos_framework::state_storage {
    spec module {
        use aptos_std::chain_status;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<StateStorageUsage>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<GasParameter>(@aptos_framework);
    }

    spec on_new_block {
        use aptos_std::chain_status;
        requires chain_status::is_operating();
        aborts_if false;
    }

    spec get_state_storage_usage_only_at_epoch_beginning {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
    }
}
