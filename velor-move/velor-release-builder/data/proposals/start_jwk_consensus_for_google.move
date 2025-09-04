// Start JWK Consensus for Google.
script {
    use velor_framework::velor_governance;
    use velor_framework::jwks;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        jwks::upsert_oidc_provider_for_next_epoch(
            &framework_signer,
            b"https://accounts.google.com",
            b"https://accounts.google.com/.well-known/openid-configuration"
        );
        velor_governance::reconfigure(&framework_signer);
    }
}
