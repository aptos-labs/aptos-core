spec aptos_framework::consensus_config {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During genesis, the Aptos framework account should be assigned the consensus config resource.
    /// Criticality: Medium
    /// Implementation: The consensus_config::initialize function calls the assert_aptos_framework function to ensure
    /// that the signer is the aptos_framework and then assigns the ConsensusConfig resource to it.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: Only aptos framework account is allowed to update the consensus configuration.
    /// Criticality: Medium
    /// Implementation: The consensus_config::set function ensures that the signer is aptos_framework.
    /// Enforcement: Formally verified via [high-level-req-2](set).
    ///
    /// No.: 3
    /// Requirement: Only a valid configuration can be used during initialization and update.
    /// Criticality: Medium
    /// Implementation: Both the initialize and set functions validate the config by ensuring its length to be greater
    /// than 0.
    /// Enforcement: Formally verified via [high-level-req-3.1](initialize) and [high-level-req-3.2](set).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure caller is admin.
    /// Aborts if StateStorageUsage already exists.
    spec initialize(aptos_framework: &signer, config: vector<u8>) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        /// [high-level-req-1]
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<ConsensusConfig>(@aptos_framework);
        /// [high-level-req-3.1]
        aborts_if !(len(config) > 0);
        ensures global<ConsensusConfig>(addr) == ConsensusConfig { config };
    }

    /// Ensure the caller is admin and `ConsensusConfig` should be existed.
    /// When setting now time must be later than last_reconfiguration_time.
    spec set(account: &signer, config: vector<u8>) {
        use aptos_framework::chain_status;
        use aptos_framework::timestamp;
        use std::signer;
        use aptos_framework::stake;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::transaction_fee;
        use aptos_framework::staking_config;

        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 120;
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;
        let addr = signer::address_of(account);
        /// [high-level-req-2]
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if !exists<ConsensusConfig>(@aptos_framework);
        /// [high-level-req-3.2]
        aborts_if !(len(config) > 0);

        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        ensures global<ConsensusConfig>(@aptos_framework).config == config;
    }
}
