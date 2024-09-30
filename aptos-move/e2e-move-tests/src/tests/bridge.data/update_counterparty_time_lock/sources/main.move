script {
    use aptos_framework::aptos_governance;
    use aptos_framework::atomic_bridge_configuration;

    fun main(core_resources: &signer, new_time_lock: u64) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        atomic_bridge_configuration::set_counterparty_time_lock_duration(&framework_signer, new_time_lock);
    }
}
