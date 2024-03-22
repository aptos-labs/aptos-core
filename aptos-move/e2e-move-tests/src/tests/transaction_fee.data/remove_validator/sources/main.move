script {
    use aptos_framework::aptos_governance;
    use aptos_framework::stake;

    fun main(core_resources: &signer, addr: address) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        stake::remove_validators(&framework_signer, &vector[addr]);

        // Make sure to trigger a reconfiguration!
        aptos_governance::force_end_epoch(&framework_signer);
    }
}
