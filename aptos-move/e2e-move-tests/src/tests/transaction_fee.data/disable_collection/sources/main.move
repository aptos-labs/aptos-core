script {
    use supra_framework::supra_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let framework_signer = supra_governance::get_signer_testnet_only(core_resources, @supra_framework);
        let feature = features::get_collect_and_distribute_gas_fees_feature();

        // Trigger reconfiguration first, to also sync all the fees to validators.
        supra_governance::force_end_epoch(&framework_signer);

        // Then, disable the feature.
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[], vector[feature]);
        supra_governance::force_end_epoch(&framework_signer);
    }
}
