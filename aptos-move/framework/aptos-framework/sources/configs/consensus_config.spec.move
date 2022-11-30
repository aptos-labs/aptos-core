spec aptos_framework::consensus_config {
    spec set {
        use aptos_framework::chain_status;
        use aptos_framework::timestamp;
        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
    }
}
