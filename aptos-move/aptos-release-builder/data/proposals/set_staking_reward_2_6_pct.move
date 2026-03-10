// Governance proposal to set staking reward rate to 2.6% per year
// and stop further yearly reductions.
//
// This script directly computes the per-epoch rate for 2.6% APY,
// then sets min_rewards_rate = rewards_rate to permanently freeze
// the rate at 2.6%. The periodic decrease mechanism remains enabled
// but is effectively neutralized since the rate is clamped to the floor.
//
// This avoids disabling the periodical_reward_rate_reduction feature flag,
// which would cause get_reward_rate() to fall back to stale legacy
// StakingConfig fields (see staking_config.move:222-224).
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::block;
    use aptos_framework::staking_config;
    use aptos_std::fixed_point64;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        let one_year_in_secs: u64 = 365 * 24 * 60 * 60;
        let epoch_duration_secs = block::get_epoch_interval_secs();
        let num_epochs_in_a_year = (one_year_in_secs as u128) / (epoch_duration_secs as u128);

        // Target annual reward rate: 2.6% per year.
        // Per-epoch rate = 26 / (1000 * num_epochs_per_year).
        let new_rewards_rate = fixed_point64::create_from_rational(
            26,
            1000 * num_epochs_in_a_year,
        );

        // Set min_rewards_rate = new_rewards_rate to stop the yearly reduction.
        // Each year the periodic decrease computes:
        //   new_rate = current_rate * 0.985
        // Then clamps: new_rate = max(new_rate, min_rewards_rate)
        // Since min == current, the rate stays at 2.6% permanently.
        let min_rewards_rate = new_rewards_rate;

        // Preserve the existing decrease rate (unchanged from on-chain value).
        // raw value 276701161105643274 = 1.5% as FixedPoint64
        let rewards_rate_decrease_rate = fixed_point64::create_from_raw_value(276701161105643274);

        staking_config::update_rewards_config(
            &framework_signer,
            new_rewards_rate,
            min_rewards_rate,
            one_year_in_secs,
            rewards_rate_decrease_rate,
        );

        aptos_governance::reconfigure(&framework_signer);
    }
}
