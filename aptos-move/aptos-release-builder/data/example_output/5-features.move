// Modifying on-chain feature flags:
// Enabled Features: [CodeDependencyCheck]
// Disabled Features: []
//
script {
    use aptos_framework::aptos_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let framework_signer = &core_signer;

        let enabled_blob: vector<u64> = vector[
            1,
        ];

        let disabled_blob: vector<u64> = vector[

        ];

        features::change_feature_flags(framework_signer, enabled_blob, disabled_blob);
    }
}
