script {
    use supra_framework::aptos_governance;
    use supra_framework::stake;

    fun main(core_resources: &signer, addr: address) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @supra_framework);
        stake::remove_validators(&framework_signer, &vector[addr]);

        // Make sure to trigger a reconfiguration!
        aptos_governance::reconfigure(&framework_signer);
    }
}
