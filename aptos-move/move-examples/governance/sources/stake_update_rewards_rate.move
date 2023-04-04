script {
    use aptos_framework::aptos_governance;
    use aptos_framework::block;
    use aptos_framework::staking_config;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve(proposal_id, @aptos_framework);
        let num_seconds_in_a_year = 365 * 24 * 60 * 60;
        let epoch_duration_secs = block::get_epoch_interval_secs();
        let num_epochs_in_a_year = num_seconds_in_a_year / epoch_duration_secs;
        // Change reward rate to 5% a year.
        let apy = 5;
        // Need to represent reward rate fraction as 2 numbers - numerator and denominator.
        let reward_rate_denominator = 1000000000;
        let reward_rate_numerator = apy * reward_rate_denominator / num_epochs_in_a_year / 100;
        staking_config::update_rewards_rate(
            &framework_signer, reward_rate_numerator, reward_rate_denominator);
    }
}
