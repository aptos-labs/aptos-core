script {
    use aptos_framework::aptos_governance;
    use aptos_framework::governed_gas_pool;

    fun initialize_governed_gas_pool_extension(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        governed_gas_pool::initialize_governed_gas_pool_extension(&core_signer);
    }
}
