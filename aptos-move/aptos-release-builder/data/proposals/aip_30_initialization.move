// Initialize AIP-28 parital governance voting.
// This script MUST be run before enabling the feature flag, otherwise no new proposal can be passed anymore.
script {
    use aptos_framework::staking_config;
    use aptos_framework::timestamp;
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0000000000000000000000000000000000000000000000000000000000000001,
            {{ script_hash }},
        );

        let is_mainnet = false;
        if (is_mainnet) {
            // for mainnet
            staking_config::initialize_rewards(
                &framework_signer,
                fixed_point64::create_from_rational(70, 1000*12*365), // rewards_rate: 7% // 2hour
                fixed_point64::create_from_rational(35, 1000*12*365), // min_rewards_rate: 3.5%
                365 * 24 * 60 * 60,                            // rewards_rate_period_in_secs: 1 year
                0,                                             // last_rewards_rate_period_start_in_secs: the timestamp at genesis
                fixed_point64::create_from_rational(15, 1000)  // rewards_rate_decrease_rate: 1.5%
            );
        }
        else {
            // for testing
            staking_config::initialize_rewards(
                &framework_signer,
                fixed_point64::create_from_rational(70, 1000*24*365), // rewards_rate: 7% / (# epochs in a year) <- TODO: check the ephoch period = 1 hours    24 * 365;
                fixed_point64::create_from_rational(35, 1000*24*365), // min_rewards_rate: 3.5% / (# epochs in a year)
                60 * 60,                                       // rewards_rate_period_in_secs: 1 hour:  too short >> the epoch period
                timestamp::now_seconds(),                      // last_rewards_rate_period_start_in_secs: the current timestamp
                fixed_point64::create_from_rational(500, 1000) // rewards_rate_decrease_rate: 25%
            );
        }
    }
}
