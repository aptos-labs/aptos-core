script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;

    const MAX_NUM_BATCH_PROPOSERS: u16 = 100;

    fun main(core_resources: &signer, block_distribution_percentage: u8, batch_distribution_percentage: u8) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        transaction_fee::initialize_fee_collection_and_distributions(&framework_signer, MAX_NUM_BATCH_PROPOSERS, block_distribution_percentage, batch_distribution_percentage);
        // Since this only places resources on the core account, no reconfiguration is needed.
    }
}
