// Initialize AIP-28 parital governance voting.
// This script MUST be run before enabling the feature flag, otherwise no new proposal can be passed anymore.
script {
    use aptos_framework::supra_governance;

    fun main(proposal_id: u64) {
        let framework_signer = supra_governance::resolve_multi_step_proposal(
            proposal_id,
            @0000000000000000000000000000000000000000000000000000000000000001,
            {{ script_hash }},
        );
        supra_governance::initialize_partial_voting(&framework_signer);
    }
}
