script {
    use aptos_framework::aptos_governance;
    use aptos_framework::atomic_bridge_configuration;

    fun main(core_resources: &signer, new_operator: address) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        atomic_bridge_configuration::update_bridge_operator(&framework_signer, new_operator);
    }
}
