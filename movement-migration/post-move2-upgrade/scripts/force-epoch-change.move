script {
    use aptos_framework::aptos_governance;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        aptos_governance::force_end_epoch(&core_signer);
    }
}
