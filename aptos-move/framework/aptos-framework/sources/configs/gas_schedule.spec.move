spec aptos_framework::gas_schedule {
    spec set_gas_schedule {
        use aptos_framework::chain_status;
        requires chain_status::is_operating();
    }

    spec set_storage_gas_config {
        use aptos_framework::chain_status;
        requires chain_status::is_operating();
    }
}
