spec supra_framework::execution_config {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure the caller is admin
    /// When setting now time must be later than last_reconfiguration_time.
    spec set(account: &signer, config: vector<u8>) {
        use supra_framework::timestamp;
        use std::signer;
        use std::features;
        use supra_framework::transaction_fee;
        use supra_framework::chain_status;
        use supra_framework::stake;
        use supra_framework::staking_config;
        use supra_framework::supra_coin;

        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 120;
        let addr = signer::address_of(account);
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        requires chain_status::is_operating();
        requires exists<stake::ValidatorFees>(@supra_framework);
        requires exists<staking_config::StakingRewardsConfig>(@supra_framework);
        requires len(config) > 0;
        include features::spec_periodical_reward_rate_decrease_enabled() ==> staking_config::StakingRewardsConfigEnabledRequirement;
        include supra_coin::ExistsSupraCoin;
        requires system_addresses::is_supra_framework_address(addr);
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();

        ensures exists<ExecutionConfig>(@supra_framework);
    }
}
