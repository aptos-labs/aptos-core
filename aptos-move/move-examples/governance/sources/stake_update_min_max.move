script {
    use aptos_framework::aptos_governance;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve(proposal_id, @aptos_framework);
        let one_aptos_coin_with_decimals = 10 ** (coin::decimals<AptosCoin>() as u64);
        // Change min to 1000 and max to 1M Aptos coins.
        let new_min_stake = 1000 * one_aptos_coin_with_decimals;
        let new_max_stake = 1000000 * one_aptos_coin_with_decimals;
        staking_config::update_required_stake(&framework_signer, new_min_stake, new_max_stake);
    }
}
