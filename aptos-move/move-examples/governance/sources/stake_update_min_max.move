script {
    use supra_framework::aptos_governance;
    use supra_framework::coin;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve(proposal_id, @supra_framework);
        let one_supra_coin_with_decimals = 10 ** (coin::decimals<SupraCoin>() as u64);
        // Change min to 1000 and max to 1M Aptos coins.
        let new_min_stake = 1000 * one_supra_coin_with_decimals;
        let new_max_stake = 1000000 * one_supra_coin_with_decimals;
        staking_config::update_required_stake(&framework_signer, new_min_stake, new_max_stake);
    }
}
