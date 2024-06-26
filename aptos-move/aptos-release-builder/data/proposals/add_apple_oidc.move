// Enable Apple JWK consensus
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

        aptos_governance::reconfigure(&framework);
    }
}
