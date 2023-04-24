spec aptos_framework::consensus_config {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure caller is admin.
    /// Aborts if StateStorageUsage already exists.
    spec initialize(aptos_framework: &signer, config: vector<u8>) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<ConsensusConfig>(@aptos_framework);
        aborts_if !(len(config) > 0);
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

        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)

        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;
        let addr = signer::address_of(account);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if !exists<ConsensusConfig>(@aptos_framework);
        aborts_if !(len(config) > 0);

        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
    }
}
