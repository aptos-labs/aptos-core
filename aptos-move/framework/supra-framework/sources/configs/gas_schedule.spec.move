spec supra_framework::gas_schedule {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During genesis, the Aptos framework account should be assigned the gas schedule resource.
    /// Criticality: Medium
    /// Implementation: The gas_schedule::initialize function calls the assert_supra_framework function to ensure that
    /// the signer is the supra_framework and then assigns the GasScheduleV2 resource to it.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: Only the Aptos framework account should be allowed to update the gas schedule resource.
    /// Criticality: Critical
    /// Implementation: The gas_schedule::set_gas_schedule function calls the assert_supra_framework function to ensure
    /// that the signer is the aptos framework account.
    /// Enforcement: Formally verified via [high-level-req-2](set_gas_schedule).
    ///
    /// No.: 3
    /// Requirement: Only valid gas schedule should be allowed for initialization and update.
    /// Criticality: Medium
    /// Implementation: The initialize and set_gas_schedule functions ensures that the gas_schedule_blob is not empty.
    /// Enforcement: Formally verified via [high-level-req-3.3](initialize) and [high-level-req-3.2](set_gas_schedule).
    ///
    /// No.: 4
    /// Requirement: Only a gas schedule with the feature version greater or equal than the current feature version is
    /// allowed to be provided when performing an update operation.
    /// Criticality: Medium
    /// Implementation: The set_gas_schedule function validates the feature_version of the new_gas_schedule by ensuring
    /// that it is greater or equal than the current gas_schedule.feature_version.
    /// Enforcement: Formally verified via [high-level-req-4](set_gas_schedule).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec initialize(supra_framework: &signer, gas_schedule_blob: vector<u8>) {
        use std::signer;

        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1]
        include system_addresses::AbortsIfNotAptosFramework{ account: supra_framework };
        /// [high-level-req-3.3]
        aborts_if len(gas_schedule_blob) == 0;
        aborts_if exists<GasScheduleV2>(addr);
        ensures exists<GasScheduleV2>(addr);
    }

    spec set_gas_schedule(supra_framework: &signer, gas_schedule_blob: vector<u8>) {
        use std::signer;
        use supra_framework::util;
        use supra_framework::stake;
        use supra_framework::coin::CoinInfo;
        use supra_framework::supra_coin::SupraCoin;
        use supra_framework::transaction_fee;
        use supra_framework::staking_config;

        // TODO: set because of timeout (property proved)
        pragma verify_duration_estimate = 120;
        requires exists<stake::ValidatorFees>(@supra_framework);
        requires exists<CoinInfo<SupraCoin>>(@supra_framework);
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;

        /// [high-level-req-2]
        include system_addresses::AbortsIfNotAptosFramework{ account: supra_framework };
        /// [high-level-req-3.2]
        aborts_if len(gas_schedule_blob) == 0;
        let new_gas_schedule = util::spec_from_bytes<GasScheduleV2>(gas_schedule_blob);
        let gas_schedule = global<GasScheduleV2>(@supra_framework);
        /// [high-level-req-4]
        aborts_if exists<GasScheduleV2>(@supra_framework) && new_gas_schedule.feature_version < gas_schedule.feature_version;
        ensures exists<GasScheduleV2>(signer::address_of(supra_framework));
        ensures global<GasScheduleV2>(@supra_framework) == new_gas_schedule;
    }

    spec set_storage_gas_config(supra_framework: &signer, config: StorageGasConfig) {
        use supra_framework::stake;
        use supra_framework::coin::CoinInfo;
        use supra_framework::supra_coin::SupraCoin;
        use supra_framework::transaction_fee;
        use supra_framework::staking_config;

        // TODO: set because of timeout (property proved).
        pragma verify_duration_estimate = 120;
        requires exists<stake::ValidatorFees>(@supra_framework);
        requires exists<CoinInfo<SupraCoin>>(@supra_framework);
        include system_addresses::AbortsIfNotAptosFramework{ account: supra_framework };
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;
        aborts_if !exists<StorageGasConfig>(@supra_framework);
        ensures global<StorageGasConfig>(@supra_framework) == config;
    }
}
