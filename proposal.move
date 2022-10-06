script {
    use aptos_framework::aptos_governance;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = &aptos_governance::resolve(proposal_id, @aptos_framework);
        // Reduce min stake by 1 APT unit. This is meant to be a very trivial change.
        let (curr_min_stake_required, curr_max_stake_required) =
            staking_config::get_required_stake(&staking_config::get());
        let updated_min_stake_required = curr_min_stake_required - 1;
        staking_config::update_required_stake(framework_signer, updated_min_stake_required, curr_max_stake_required);

        // Always trigger a reconfig event at the end of a proposal execution.
        aptos_governance::reconfigure(framework_signer);
    }
}
