script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;

    fun main(core_resources: &signer, block_distribution_percentage: u8, batch_distribution_percentage: u8) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        transaction_fee::upgrade_distribution_percentages(&framework_signer, block_distribution_percentage, batch_distribution_percentage);

        // Make sure to trigger a reconfiguration!
        aptos_governance::reconfigure(&framework_signer);
    }
}
