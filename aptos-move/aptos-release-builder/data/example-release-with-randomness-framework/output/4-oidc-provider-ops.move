// Script hash: 268052ee
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwks;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        jwks::upsert_oidc_provider_for_next_epoch(framework_signer, b"https://accounts.google.com", b"https://accounts.google.com/.well-known/openid-configuration");
        jwks::remove_oidc_provider_for_next_epoch(framework_signer, b"https://www.facebook.com");
        aptos_governance::reconfigure(framework_signer);
    }
}
