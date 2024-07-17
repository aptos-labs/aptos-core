script {
    use supra_framework::supra_governance;

    fun main(proposal_id: u64) {
        let framework_signer = supra_governance::resolve(proposal_id, @supra_framework);
        // Update voting duration of Aptos governance proposals to 1 day. Other params don't change.
        let updated_voting_duration_secs = 24 * 60 * 60;
        let unchanged_min_voting_threshold = supra_governance::get_min_voting_threshold();
        let unchanged_required_proposer_stake = supra_governance::get_required_proposer_stake();
        supra_governance::update_governance_config(
            &framework_signer,
            unchanged_min_voting_threshold,
            unchanged_required_proposer_stake,
            updated_voting_duration_secs,
        );
    }
}
