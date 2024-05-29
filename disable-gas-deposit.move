// Disable gas deposit requirement.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::randomness_api_v0_config;
    use std::options;
    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        randomness_api_v0_config::set_for_next_epoch(&framework, option::none());
        aptos_governance::reconfigure(&framework);
    }
}
