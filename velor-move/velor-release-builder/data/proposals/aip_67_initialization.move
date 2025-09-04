// Initialize AIP-67 parital governance voting.
script {
    use velor_framework::velor_governance;
    use velor_framework::jwks;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );
        jwks::initialize(&framework_signer);
    }
}
