script {
    use velor_framework::velor_governance;
    use velor_framework::coin;
    use velor_framework::velor_coin::VelorCoin;
    use velor_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve(proposal_id, @velor_framework);
        let one_velor_coin_with_decimals = 10 ** (coin::decimals<VelorCoin>() as u64);
        // Change min to 1000 and max to 1M Velor coins.
        let new_min_stake = 1000 * one_velor_coin_with_decimals;
        let new_max_stake = 1000000 * one_velor_coin_with_decimals;
        staking_config::update_required_stake(&framework_signer, new_min_stake, new_max_stake);
    }
}
