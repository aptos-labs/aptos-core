script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwks;

    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let issuer = b"test.oidc.provider";
        let config_url = b"https://storage.googleapis.com/aptos-keyless-jwks/keys.json";
        jwks::upsert_oidc_provider(&framework, issuer, config_url);
        aptos_governance::reconfigure(&framework);
    }
}
