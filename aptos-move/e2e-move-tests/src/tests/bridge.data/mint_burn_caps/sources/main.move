script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;
    use aptos_framework::atomic_bridge;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        let (mint, burn) = transaction_fee::copy_capabilities_for_bridge(&framework_signer);

        atomic_bridge::store_aptos_coin_mint_cap(&framework_signer, mint);
        atomic_bridge::store_aptos_coin_burn_cap(&framework_signer, burn);
    }
}
