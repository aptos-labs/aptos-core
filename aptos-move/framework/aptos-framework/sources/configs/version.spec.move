spec aptos_framework::version {
    spec set_version {
        use aptos_framework::chain_status;
        use aptos_framework::timestamp;
        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
    }
}
