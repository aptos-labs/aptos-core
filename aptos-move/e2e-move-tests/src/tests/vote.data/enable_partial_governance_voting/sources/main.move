script {
    use aptos_framework::aptos_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        aptos_governance::initialize_partial_voting(&framework_signer);
        let feature = features::get_partial_governance_voting();
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[feature], vector[]);
        aptos_governance::force_end_epoch(&framework_signer);
    }
}
