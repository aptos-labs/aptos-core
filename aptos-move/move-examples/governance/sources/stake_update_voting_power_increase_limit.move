script {
    use supra_framework::supra_governance;
    use supra_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = supra_governance::resolve(proposal_id, @supra_framework);
        // Update voting power increase limit to 10%.
        staking_config::update_voting_power_increase_limit(&framework_signer, 10);
    }
}
