// Script hash: a9f09ee9
// Modifying on-chain feature flags:
// Enabled Features:
//   NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE = 64,
//   DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE = 68,
// Disabled Features:
//   None
script {
    use aptos_framework::aptos_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        let enabled_blob: vector<u64> = vector[
            64, 68,
        ];

        let disabled_blob: vector<u64> = vector[
        ];

        features::change_feature_flags_for_next_epoch(framework_signer, enabled_blob, disabled_blob);
        aptos_governance::reconfigure(framework_signer);
    }
}
