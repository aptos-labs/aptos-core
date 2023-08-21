spec aptos_framework::gas_schedule {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec initialize(aptos_framework: &signer, gas_schedule_blob: vector<u8>) {
        use std::signer;

        include system_addresses::AbortsIfNotAptosFramework{ account: aptos_framework };
        aborts_if len(gas_schedule_blob) == 0;
        aborts_if exists<GasScheduleV2>(signer::address_of(aptos_framework));
        ensures exists<GasScheduleV2>(signer::address_of(aptos_framework));
    }

    spec set_gas_schedule(aptos_framework: &signer, gas_schedule_blob: vector<u8>) {
        use std::signer;
        use aptos_framework::util;
        use aptos_framework::stake;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::transaction_fee;
        use aptos_framework::staking_config;
        pragma verify_duration_estimate = 120;

        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;

        include system_addresses::AbortsIfNotAptosFramework{ account: aptos_framework };
        aborts_if len(gas_schedule_blob) == 0;
        let new_gas_schedule = util::spec_from_bytes<GasScheduleV2>(gas_schedule_blob);
        let gas_schedule = global<GasScheduleV2>(@aptos_framework);
        aborts_if exists<GasScheduleV2>(@aptos_framework) && new_gas_schedule.feature_version < gas_schedule.feature_version;
        ensures exists<GasScheduleV2>(signer::address_of(aptos_framework));
    }

    spec set_storage_gas_config(aptos_framework: &signer, config: StorageGasConfig) {
        use aptos_framework::stake;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::transaction_fee;
        use aptos_framework::staking_config;

        pragma verify_duration_estimate = 200;

        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include system_addresses::AbortsIfNotAptosFramework{ account: aptos_framework };
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include staking_config::StakingRewardsConfigRequirement;
        aborts_if !exists<StorageGasConfig>(@aptos_framework);
    }
}
