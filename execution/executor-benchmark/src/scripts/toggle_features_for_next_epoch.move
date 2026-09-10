script {
    use aptos_framework::aptos_governance;
    use std::features;

    // `enable`/`disable` are feature-flag indices (FeatureFlag as u64).
    fun main(core_resources: &signer, enable: vector<u64>, disable: vector<u64>) {
        let framework_signer =
            aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        features::change_feature_flags_for_next_epoch(
            &framework_signer, enable, disable,
        );
        // force_end_epoch (not reconfigure) so the epoch bump is synchronous.
        aptos_governance::force_end_epoch(&framework_signer);
    }
}
