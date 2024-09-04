script {
    use aptos_framework::aptos_governance;
    use aptos_framework::atomic_bridge;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        atomic_bridge::initialize(&framework_signer);
    }
}
