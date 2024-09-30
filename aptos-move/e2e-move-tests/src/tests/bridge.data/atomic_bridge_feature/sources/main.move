script {
    use aptos_framework::aptos_governance;
    use aptos_framework::features;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        features::change_feature_flags_for_next_epoch(
            &framework_signer,
            vector[features::get_atomic_bridge_feature()],
            vector[]
        );

        features::on_new_epoch(&framework_signer);
    }
}
