spec supra_framework::version {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During genesis, the Version resource should be initialized with the initial version and stored along
    /// with its capability under the supra framework account.
    /// Criticality: Medium
    /// Implementation: The initialize function ensures that the signer is the supra framework account and stores the
    /// Version and SetVersionCapability resources in it.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: The version should be updateable after initialization, but only by the Supra framework account and
    /// with an increasing version number.
    /// Criticality: Medium
    /// Implementation: The version number for the blockchain should be updatable whenever necessary. This functionality
    /// is provided by the set_version function which ensures that the new version is greater than the previous one.
    /// Enforcement: Formally verified via [high-level-req-2](set_version).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec set_version(account: &signer, major: u64) {
        use std::signer;
        use supra_framework::chain_status;
        use supra_framework::timestamp;
        use supra_framework::stake;
        use supra_framework::coin::CoinInfo;
        use supra_framework::supra_coin::SupraCoin;
        use supra_framework::transaction_fee;
        use supra_framework::staking_config;
        use supra_framework::reconfiguration;

        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 120;
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;
        requires chain_status::is_genesis();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
        requires exists<stake::ValidatorFees>(@supra_framework);
        requires exists<CoinInfo<SupraCoin>>(@supra_framework);

        aborts_if !exists<SetVersionCapability>(signer::address_of(account));
        aborts_if !exists<Version>(@supra_framework);

        let old_major = global<Version>(@supra_framework).major;
        /// [high-level-req-2]
        aborts_if !(old_major < major);

        ensures global<Version>(@supra_framework).major == major;
    }

    /// Abort if resource already exists in `@aptos_framwork` when initializing.
    spec initialize(supra_framework: &signer, initial_version: u64) {
        use std::signer;

        /// [high-level-req-1]
        aborts_if signer::address_of(supra_framework) != @supra_framework;
        aborts_if exists<Version>(@supra_framework);
        aborts_if exists<SetVersionCapability>(@supra_framework);
        ensures exists<Version>(@supra_framework);
        ensures exists<SetVersionCapability>(@supra_framework);
        ensures global<Version>(@supra_framework) == Version { major: initial_version };
        ensures global<SetVersionCapability>(@supra_framework) == SetVersionCapability {};
    }

    spec set_for_next_epoch(account: &signer, major: u64) {
        aborts_if !exists<SetVersionCapability>(signer::address_of(account));
        aborts_if !exists<Version>(@supra_framework);
        aborts_if global<Version>(@supra_framework).major >= major;
        aborts_if !exists<config_buffer::PendingConfigs>(@supra_framework);
    }

    spec on_new_epoch(framework: &signer) {
        requires @supra_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<Version>;
        aborts_if false;
    }

    /// This module turns on `aborts_if_is_strict`, so need to add spec for test function `initialize_for_test`.
    spec initialize_for_test {
        // Don't verify test functions.
        pragma verify = false;
    }
}
