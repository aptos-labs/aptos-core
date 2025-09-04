script {
    use velor_framework::velor_governance;
    use velor_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve(proposal_id, @velor_framework);
        // Update voting power increase limit to 10%.
        staking_config::update_voting_power_increase_limit(&framework_signer, 10);
    }
}
