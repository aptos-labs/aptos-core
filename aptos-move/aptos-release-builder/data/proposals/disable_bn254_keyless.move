script {
    use aptos_framework::aptos_governance;
    use std::features;
    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        features::change_feature_flags_for_next_epoch(
            &framework,
            vector[],
            vector[features::get_bn254_strutures_feature(), features::get_keyless_accounts_feature()]
        );
        aptos_governance::reconfigure(&framework);
    }
}
