script {
    use supra_framework::supra_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let framework_signer = supra_governance::get_signer_testnet_only(core_resources, @supra_framework);
        let feature = features::get_partial_governance_voting();
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[feature], vector[]);
        supra_governance::force_end_epoch(&framework_signer);
    }
}
