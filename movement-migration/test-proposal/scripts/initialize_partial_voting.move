script {
    use aptos_framework::aptos_governance;
    use aptos_framework::governed_gas_pool;
    use aptos_framework::staking_config;
    use std::features;
    use aptos_std::fixed_point64;

    fun init_partial_voting(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        aptos_governance::initialize_partial_voting(&core_signer);
        aptos_governance::force_end_epoch(&core_signer);
    }
}
