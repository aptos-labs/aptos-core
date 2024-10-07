spec aptos_framework::execution_config {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure the caller is admin
    /// When setting now time must be later than last_reconfiguration_time.
    spec set(account: &signer, config: vector<u8>) {
        use aptos_framework::timestamp;
        use std::signer;
        use std::features;
        use aptos_framework::transaction_fee;
        use aptos_framework::chain_status;
        use aptos_framework::stake;
        use aptos_framework::staking_config;
        use aptos_framework::aptos_coin;

        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 600;
        let addr = signer::address_of(account);
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        requires chain_status::is_genesis();
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<staking_config::StakingRewardsConfig>(@aptos_framework);
        requires len(config) > 0;
        include features::spec_periodical_reward_rate_decrease_enabled() ==> staking_config::StakingRewardsConfigEnabledRequirement;
        include aptos_coin::ExistsAptosCoin;
        requires system_addresses::is_aptos_framework_address(addr);
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();

        ensures exists<ExecutionConfig>(@aptos_framework);
    }

    spec set_for_next_epoch(account: &signer, config: vector<u8>) {
        include config_buffer::SetForNextEpochAbortsIf;
    }

    spec on_new_epoch(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<ExecutionConfig>;
        aborts_if false;
    }
}
