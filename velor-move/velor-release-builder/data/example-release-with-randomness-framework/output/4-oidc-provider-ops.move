// Script hash: 268052ee
script {
    use velor_framework::velor_governance;
    use velor_framework::jwks;

    fun main(core_resources: &signer) {
        let core_signer = velor_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        jwks::upsert_oidc_provider_for_next_epoch(framework_signer, b"https://accounts.google.com", b"https://accounts.google.com/.well-known/openid-configuration");
        jwks::remove_oidc_provider_for_next_epoch(framework_signer, b"https://www.facebook.com");
        velor_governance::reconfigure(framework_signer);
    }
}
