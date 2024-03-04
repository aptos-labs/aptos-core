script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwks;
    use std::features;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        features::change_feature_flags(&framework_signer, vector[], vector[features::get_jwk_consensus_feature()]);
        jwks::remove_oidc_provider(&framework_signer, b"https://accounts.google.com");
        jwks::remove_issuer_from_observed_jwks(&framework_signer, b"https://accounts.google.com");
        aptos_governance::reconfigure(&framework_signer);
    }
}
