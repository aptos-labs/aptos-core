// Script hash: a9f09ee9
// Modifying on-chain feature flags:
// Enabled Features: [Bls12381Structures]
// Disabled Features: [Bn254Structures]
//
script {
    use velor_framework::velor_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let core_signer = velor_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        let enabled_blob: vector<u64> = vector[
            13,
        ];

        let disabled_blob: vector<u64> = vector[
            43,
        ];

        features::change_feature_flags_for_next_epoch(framework_signer, enabled_blob, disabled_blob);
        velor_governance::reconfigure(framework_signer);
    }
}
