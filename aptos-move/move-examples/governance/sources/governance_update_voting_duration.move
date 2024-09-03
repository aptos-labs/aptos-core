script {
    use supra_framework::supra_governance;

    fun main(proposal_id: u64) {
        let framework_signer = supra_governance::supra_resolve(proposal_id, @supra_framework);
        // Update voting duration of Aptos governance proposals to 1 day. Other params don't change.
        let updated_voting_duration_secs = 24 * 60 * 60;
        let unchanged_min_voting_threshold = supra_governance::get_min_voting_threshold();
        let voters = supra_governance::get_voters_list();
        supra_governance::update_supra_governance_config(
            &framework_signer,
            unchanged_min_voting_threshold,
            updated_voting_duration_secs,
            voters,
        );
    }
}
