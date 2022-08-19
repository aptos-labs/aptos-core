script {
    use aptos_framework::aptos_governance;
    use aptos_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve(proposal_id, @aptos_framework);
        // Update voting power increase limit to 10%.
        staking_config::update_voting_power_increase_limit(&framework_signer, 10);
    }
}
