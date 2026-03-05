// Governance proposal to set staking reward rate to 2.6% per year.
//
// This script directly sets the per-epoch rewards rate to a value equivalent
// to 2.6% APY (annual rate = per_epoch_rate * num_epochs_per_year).
//
// The existing yearly reduction mechanism (rewards_rate_decrease_rate = 1.5%)
// is preserved and will continue to apply automatically each year after this
// proposal is executed. The decrease is multiplicative:
//   new_rate = current_rate * (1 - 0.015)
// So from 2.6%: year 1 → 2.561%, year 2 → 2.522%, etc.
//
// IMPORTANT: Verify `min_rewards_rate` and `rewards_rate_decrease_rate` match
// current on-chain values before deploying to mainnet. The values below reflect
// the parameters set by the AIP-6 governance proposal (min=1.5%/yr, decrease=1.5%/yr).
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

        // Compute how many epochs fit in one year based on current epoch interval.
        let one_year_in_secs: u64 = 365 * 24 * 60 * 60;
        let epoch_duration_secs = block::get_epoch_interval_secs();
        let num_epochs_in_a_year = (one_year_in_secs as u128) / (epoch_duration_secs as u128);

        // Target annual reward rate: 2.6% per year.
        // Per-epoch rate = 2.6% / num_epochs_per_year = 26 / (1000 * num_epochs_per_year).
        let new_rewards_rate = fixed_point64::create_from_rational(
            26,
            1000 * num_epochs_in_a_year,
        );

        // Minimum reward rate floor: 1.5% per year expressed as per-epoch rate.
        // This ensures the rate never drops below 1.5% annually via the automatic
        // yearly reduction. Adjust this value if a different floor is desired.
        let min_rewards_rate = fixed_point64::create_from_rational(
            15,
            1000 * num_epochs_in_a_year,
        );

        // Yearly decrease rate of 1.5% (multiplicative).
        // Each year: new_annual_rate = current_annual_rate * (1 - 0.015).
        // This preserves the existing annual reduction schedule.
        let rewards_rate_decrease_rate = fixed_point64::create_from_rational(15, 1000);

        // rewards_rate_period_in_secs must equal the stored value (ONE_YEAR_IN_SECS).
        // This field cannot be changed once configured; it is validated on-chain.
        let rewards_rate_period_in_secs: u64 = 365 * 24 * 60 * 60;

        staking_config::update_rewards_config(
            &framework_signer,
            new_rewards_rate,
            min_rewards_rate,
            rewards_rate_period_in_secs,
            rewards_rate_decrease_rate,
        );

        aptos_governance::reconfigure(&framework_signer);
    }
}
