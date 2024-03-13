script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwks;

    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        jwks::upsert_oidc_provider_for_next_epoch(&framework, b"https://acounts.google.com", b"https://accounts.google.com/.well-known/openid-configuration");
        aptos_governance::reconfigure(&framework);
    }
}
