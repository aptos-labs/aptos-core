script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwk_consensus_config;
    use std::string::utf8;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let provider_google = jwk_consensus_config::new_oidc_provider(
            utf8(b"https://accounts.google.com"),
            utf8(b"https://accounts.google.com/.well-known/openid-configuration"),
        );
        let provider_test = jwk_consensus_config::new_oidc_provider(
            utf8(b"test.oidc.provider"),
            utf8(b"https://storage.googleapis.com/aptos-keyless-jwks/keys.json"),
        );
        let config = jwk_consensus_config::new_v1(vector[provider_google, provider_test]);
        jwk_consensus_config::set_for_next_epoch(&framework, config);
        aptos_governance::reconfigure(&framework);
    }
}
