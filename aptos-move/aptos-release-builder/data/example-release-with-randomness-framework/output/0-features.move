// Script hash: a9f09ee9
// Modifying on-chain feature flags:
// Enabled Features: [Bls12381Structures]
// Disabled Features: [Bn254Structures]
//
script {
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;
        aptos_framework::coin::create_coin_conversion_map(framework_signer);
        aptos_framework::coin::create_pairing<aptos_framework::aptos_coin::AptosCoin>(framework_signer);
        std::features::change_feature_flags_for_next_epoch(
            framework_signer,
            vector[std::features::get_module_event_migration_feature()],
            vector[]
        );
    }
}
