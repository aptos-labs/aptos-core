script {
    use aptos_framework::jwks;
    use aptos_framework::aptos_governance;
    use std::option;
    use std::vector;
    use std::string::utf8;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        jwks::upsert_oidc_provider(&framework_signer, b"https://accounts.google.com", b"https://accounts.google.com/.well-known/openid-configuration");
    }
}
