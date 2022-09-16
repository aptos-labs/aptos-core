spec aptos_framework::storage_gas {
    spec fun spec_calculate_gas(max_usage: u64, current_usage: u64, curve: GasCurve): u64;
    spec calculate_gas {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_calculate_gas(max_usage, current_usage, curve);
    }

    spec on_reconfig {
        use aptos_std::chain_status;
        requires chain_status::is_operating();
        aborts_if false;
    }

    spec module {
        use aptos_std::chain_status;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGasConfig>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGas>(@aptos_framework);
    }
}
