/// This module defines structs and methods to initialize the gas schedule, which dictates how much
/// it costs to execute Move on the network.
module aptos_framework::gas_schedule {
    use std::error;
    use std::string::String;
    use std::vector;
    use std::signer;

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use aptos_framework::util::from_bytes;

    friend aptos_framework::genesis;

    /// The provided gas schedule bytes are empty or invalid
    const EINVALID_GAS_SCHEDULE: u64 = 1;
    const EINVALID_GAS_FEATURE_VERSION: u64 = 2;

    const GENESIS_GAS_FEATURE_VERSION: u64 = 0;

    struct GasFeatureVersion has key, copy, drop {
        major: u64,
    }

    struct GasEntry has store, copy, drop {
        key: String,
        val: u64,
    }

    struct GasSchedule has key, copy, drop {
        entries: vector<GasEntry>
    }

    /// Called during a module upgrade.
    /// Creates and publishes the resources that are added at a later time.
    public fun upgrade_module(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);

        if (!exists<GasFeatureVersion>(signer::address_of(aptos_framework))) {
            move_to<GasFeatureVersion>(aptos_framework, GasFeatureVersion { major: GENESIS_GAS_FEATURE_VERSION });
        }
    }

    /// Only called during genesis.
    public(friend) fun initialize(aptos_framework: &signer, gas_schedule_blob: vector<u8>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(vector::length(&gas_schedule_blob) > 0, error::invalid_argument(EINVALID_GAS_SCHEDULE));

        // TODO(Gas): check if gas schedule is consistent
        move_to<GasSchedule>(aptos_framework, from_bytes(gas_schedule_blob));
        move_to<GasFeatureVersion>(aptos_framework, GasFeatureVersion { major: GENESIS_GAS_FEATURE_VERSION });
    }

    /// This can be called by on-chain governance to update the gas schedule.
    public fun set_gas_schedule(aptos_framework: &signer, gas_schedule_blob: vector<u8>) acquires GasSchedule {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(vector::length(&gas_schedule_blob) > 0, error::invalid_argument(EINVALID_GAS_SCHEDULE));

        // TODO(Gas): check if gas schedule is consistent
        let gas_schedule = borrow_global_mut<GasSchedule>(@aptos_framework);
        *gas_schedule = from_bytes(gas_schedule_blob);

        // Need to trigger reconfiguration so validator nodes can sync on the updated gas schedule.
        reconfiguration::reconfigure();
    }

    /// This can be called by on-chain governance to update the gas feature version.
    public fun set_gas_feature_version(aptos_framework: &signer, new_version: u64) acquires GasFeatureVersion {
        system_addresses::assert_aptos_framework(aptos_framework);

        let version = borrow_global_mut<GasFeatureVersion>(signer::address_of(aptos_framework));
        assert!(new_version > version.major, error::invalid_argument(EINVALID_GAS_FEATURE_VERSION));
        version.major = new_version;
    }
}
