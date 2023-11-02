spec aptos_framework::genesis {
    spec module {
        pragma verify = true;
    }

    spec initialize {
        include InitalizeRequires;
        ensures exists<aptos_governance::GovernanceResponsbility>(@aptos_framework);
        ensures exists<consensus_config::ConsensusConfig>(@aptos_framework);
        ensures exists<execution_config::ExecutionConfig>(@aptos_framework);
        ensures exists<version::Version>(@aptos_framework);
        ensures exists<stake::ValidatorSet>(@aptos_framework);
        ensures exists<stake::ValidatorPerformance>(@aptos_framework);
        ensures exists<storage_gas::StorageGasConfig>(@aptos_framework);
        ensures exists<storage_gas::StorageGas>(@aptos_framework);
        ensures exists<gas_schedule::GasScheduleV2>(@aptos_framework);
        ensures exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        ensures exists<coin::SupplyConfig>(@aptos_framework);
        ensures exists<chain_id::ChainId>(@aptos_framework);
        ensures exists<reconfiguration::Configuration>(@aptos_framework);
        ensures exists<block::BlockResource>(@aptos_framework);
        ensures exists<state_storage::StateStorageUsage>(@aptos_framework);
        ensures exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        ensures exists<account::Account>(@aptos_framework);
    }

    spec initialize_aptos_coin {
        requires !exists<stake::AptosCoinCapabilities>(@aptos_framework);
        ensures exists<stake::AptosCoinCapabilities>(@aptos_framework);
        requires exists<transaction_fee::AptosCoinCapabilities>(@aptos_framework);
        ensures exists<transaction_fee::AptosCoinCapabilities>(@aptos_framework);
    }

    spec create_initialize_validators_with_commission {
        include stake::ResourceRequirement;
        include CompareTimeRequires;
        include aptos_coin::ExistsAptosCoin;
    }

    spec create_initialize_validators {
        include stake::ResourceRequirement;
        include CompareTimeRequires;
        include aptos_coin::ExistsAptosCoin;
    }

    spec create_initialize_validator {
        include stake::ResourceRequirement;
    }

    spec initialize_for_verification {
        // We construct `initialize_for_verification` which is a "#[verify_only]" function that
        // simulates the genesis encoding process in `vm-genesis` (written in Rust).
        include InitalizeRequires;
    }

    spec schema InitalizeRequires {
        execution_config: vector<u8>;
        requires !exists<account::Account>(@aptos_framework);
        requires chain_status::is_operating();
        requires len(execution_config) > 0;
        requires exists<staking_config::StakingRewardsConfig>(@aptos_framework);
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
        include CompareTimeRequires;
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
    }

    spec schema CompareTimeRequires {
        let staking_rewards_config = global<staking_config::StakingRewardsConfig>(@aptos_framework);
        requires staking_rewards_config.last_rewards_rate_period_start_in_secs <= timestamp::spec_now_seconds();
    }
}
