script {
    use aptos_framework::aptos_governance;
    use aptos_framework::staking_config;
    use std::vector;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @0000000000000000000000000000000000000000000000000000000000000001, vector::empty<u8>());

        // Update staking lockup duration from 30d to 14d.
        staking_config::update_recurring_lockup_duration_secs(
            &framework_signer,
            14 * 24 * 60 * 60, // 14 days * 24 hours * 60 mins * 60 secs
        );

        // Update governance voting duration from 7d to 3d
        aptos_governance::update_governance_config(
            &framework_signer,
            aptos_governance::get_min_voting_threshold(), // Unchanged
            aptos_governance::get_required_proposer_stake(), // Unchanged
            3 * 24 * 60 * 60, // 3 days * 24 hours * 60 mins * 60 secs
        );

        // Trigger reconfiguration for changes to take effect immediately
        aptos_governance::reconfigure(&framework_signer);
    }
}
