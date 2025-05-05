spec aptos_framework::genesis {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: All the core resources and modules should be created during genesis and owned by the Aptos framework
    /// account.
    /// Criticality: Critical
    /// Implementation: Resources created during genesis initialization: GovernanceResponsbility, ConsensusConfig,
    /// ExecutionConfig, Version, SetVersionCapability, ValidatorSet, ValidatorPerformance, StakingConfig,
    /// StorageGasConfig, StorageGas, GasScheduleV2, AggregatorFactory, SupplyConfig, ChainId, Configuration,
    /// BlockResource, StateStorageUsage, CurrentTimeMicroseconds. If some of the resources were to be owned by a
    /// malicious account, it could lead to the compromise of the chain, as these are core resources. It should be
    /// formally verified by a post condition to ensure that all the critical resources are owned by the Aptos framework.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: Addresses ranging from 0x0 - 0xa should be reserved for the framework and part of aptos governance.
    /// Criticality: Critical
    /// Implementation: The function genesis::initialize calls account::create_framework_reserved_account for addresses
    /// 0x0, 0x2, 0x3, 0x4, ..., 0xa which creates an account and authentication_key for them. This should be formally
    /// verified by ensuring that at the beginning of the genesis::initialize function no Account resource exists for
    /// the reserved addresses, and at the end of the function, an Account resource exists.
    /// Enforcement: Formally verified via [high-level-req-2](initialize).
    ///
    /// No.: 3
    /// Requirement: The Aptos coin should be initialized during genesis and only the Aptos framework account should own
    /// the mint and burn capabilities for the APT token.
    /// Criticality: Critical
    /// Implementation: Both mint and burn capabilities are wrapped inside the stake::AptosCoinCapabilities and
    /// transaction_fee::AptosCoinCapabilities resources which are stored under the aptos framework account.
    /// Enforcement: Formally verified via [high-level-req-3](initialize_aptos_coin).
    ///
    /// No.: 4
    /// Requirement: An initial set of validators should exist before the end of genesis.
    /// Criticality: Low
    /// Implementation: To ensure that there will be a set of validators available to validate the genesis block, the
    /// length of the ValidatorSet.active_validators vector should be > 0.
    /// Enforcement: Formally verified via [high-level-req-4](set_genesis_end).
    ///
    /// No.: 5
    /// Requirement: The end of genesis should be marked on chain.
    /// Criticality: Low
    /// Implementation: The end of genesis is marked, on chain, via the chain_status::GenesisEndMarker resource. The
    /// ownership of this resource marks the operating state of the chain.
    /// Enforcement: Formally verified via [high-level-req-5](set_genesis_end).
    /// </high-level-req>
    spec module {
        pragma verify = true;
    }

    spec initialize {
        pragma aborts_if_is_partial;
        include InitalizeRequires;

        // property 2: Addresses ranging from 0x0 - 0xa should be reserved for the framework and part of aptos governance.
        // 0x1's pre and post conditions are written in requires schema and the following group of ensures.
        /// [high-level-req-2]
        aborts_if exists<account::Account>(@0x0);
        aborts_if exists<account::Account>(@0x2);
        aborts_if exists<account::Account>(@0x3);
        aborts_if exists<account::Account>(@0x4);
        aborts_if exists<account::Account>(@0x5);
        aborts_if exists<account::Account>(@0x6);
        aborts_if exists<account::Account>(@0x7);
        aborts_if exists<account::Account>(@0x8);
        aborts_if exists<account::Account>(@0x9);
        aborts_if exists<account::Account>(@0xa);
        ensures exists<account::Account>(@0x0);
        ensures exists<account::Account>(@0x2);
        ensures exists<account::Account>(@0x3);
        ensures exists<account::Account>(@0x4);
        ensures exists<account::Account>(@0x5);
        ensures exists<account::Account>(@0x6);
        ensures exists<account::Account>(@0x7);
        ensures exists<account::Account>(@0x8);
        ensures exists<account::Account>(@0x9);
        ensures exists<account::Account>(@0xa);

        // property 1: All the core resources and modules should be created during genesis and owned by the Aptos framework account.
        /// [high-level-req-1]
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
        ensures exists<version::SetVersionCapability>(@aptos_framework);
        ensures exists<staking_config::StakingConfig>(@aptos_framework);
    }

    spec initialize_aptos_coin {
        // property 3: The Aptos coin should be initialized during genesis and only the Aptos framework account should
        // own the mint and burn capabilities for the APT token.
        /// [high-level-req-3]
        requires !exists<stake::AptosCoinCapabilities>(@aptos_framework);
        ensures exists<stake::AptosCoinCapabilities>(@aptos_framework);
        requires exists<transaction_fee::AptosCoinCapabilities>(@aptos_framework);
        ensures exists<transaction_fee::AptosCoinCapabilities>(@aptos_framework);
    }

    spec initialize_validator {
        pragma verify_duration_estimate = 120;
    }

    spec create_initialize_validators_with_commission {
        pragma verify_duration_estimate = 120;

        include stake::ResourceRequirement;
        include stake::GetReconfigStartTimeRequirement;
        include CompareTimeRequires;
        include aptos_coin::ExistsAptosCoin;
    }

    spec create_initialize_validators {
        pragma verify_duration_estimate = 120;

        include stake::ResourceRequirement;
        include stake::GetReconfigStartTimeRequirement;
        include CompareTimeRequires;
        include aptos_coin::ExistsAptosCoin;
    }

    spec create_initialize_validator {
        pragma verify_duration_estimate = 120;
        include stake::ResourceRequirement;
    }

    spec initialize_for_verification {
        // This function cause timeout (property proved)
        pragma verify_duration_estimate = 120;
        // We construct `initialize_for_verification` which is a "#[verify_only]" function that
        // simulates the genesis encoding process in `vm-genesis` (written in Rust).
        include InitalizeRequires;
    }

    spec set_genesis_end {
        pragma delegate_invariants_to_caller;
        // property 4: An initial set of validators should exist before the end of genesis.
        /// [high-level-req-4]
        requires len(global<stake::ValidatorSet>(@aptos_framework).active_validators) >= 1;
        // property 5: The end of genesis should be marked on chain.
        /// [high-level-req-5]
        let addr = std::signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if exists<chain_status::GenesisEndMarker>(@aptos_framework);
        ensures global<chain_status::GenesisEndMarker>(@aptos_framework) == chain_status::GenesisEndMarker {};
    }

    spec schema InitalizeRequires {
        execution_config: vector<u8>;
        requires !exists<account::Account>(@aptos_framework);
        requires chain_status::is_operating();
        requires len(execution_config) > 0;
        requires exists<staking_config::StakingRewardsConfig>(@aptos_framework);
        requires exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
        include CompareTimeRequires;
    }

    spec schema CompareTimeRequires {
        let staking_rewards_config = global<staking_config::StakingRewardsConfig>(@aptos_framework);
        requires staking_rewards_config.last_rewards_rate_period_start_in_secs <= timestamp::spec_now_seconds();
    }
}
