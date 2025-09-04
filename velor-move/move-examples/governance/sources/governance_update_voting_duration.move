script {
    use velor_framework::velor_governance;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve(proposal_id, @velor_framework);
        // Update voting duration of Velor governance proposals to 1 day. Other params don't change.
        let updated_voting_duration_secs = 24 * 60 * 60;
        let unchanged_min_voting_threshold = velor_governance::get_min_voting_threshold();
        let unchanged_required_proposer_stake = velor_governance::get_required_proposer_stake();
        velor_governance::update_governance_config(
            &framework_signer,
            unchanged_min_voting_threshold,
            unchanged_required_proposer_stake,
            updated_voting_duration_secs,
        );
    }
}
