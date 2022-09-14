spec aptos_framework::gas_schedule {
    spec set_gas_schedule {
        use aptos_framework::chain_status;
        use aptos_framework::timestamp;
        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
    }
}
