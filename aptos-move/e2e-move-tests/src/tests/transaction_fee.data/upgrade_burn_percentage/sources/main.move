script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;

    fun main(core_resources: &signer, burn_percentage: u8) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        transaction_fee::upgrade_burn_percentage(&framework_signer, burn_percentage);

        // Make sure to trigger a reconfiguration!
        aptos_governance::force_end_epoch(&framework_signer);
    }
}
