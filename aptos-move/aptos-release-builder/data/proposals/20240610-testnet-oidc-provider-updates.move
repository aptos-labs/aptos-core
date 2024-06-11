// Enable Apple JWK consensus, disable Facebook JWK consensus and clean up Facebook JWKs.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwks;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        jwks::upsert_oidc_provider_for_next_epoch(
            &framework,
            b"https://appleid.apple.com",
            b"https://appleid.apple.com/.well-known/openid-configuration"
        );

        jwks::remove_oidc_provider_for_next_epoch(&framework, b"https://www.facebook.com");
        jwks::remove_issuer_from_observed_jwks(&framework, b"https://www.facebook.com");
        aptos_governance::reconfigure(&framework);
    }
}
