script {
    use velor_framework::velor_governance;
    use std::features;

    fun main(core_resources: &signer, enable_partial_governance_voting: bool) {
        let framework_signer = velor_governance::get_signer_testnet_only(core_resources, @velor_framework);
        let feature = features::get_partial_governance_voting();
        if (enable_partial_governance_voting) {
            features::change_feature_flags_for_next_epoch(&framework_signer, vector[feature], vector[]);
        } else {
            features::change_feature_flags_for_next_epoch(&framework_signer, vector[], vector[feature]);
        };
        velor_governance::force_end_epoch(&framework_signer);
    }
}
