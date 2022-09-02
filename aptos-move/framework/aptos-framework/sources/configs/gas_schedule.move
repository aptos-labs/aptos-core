/// This module defines structs and methods to initialize the gas schedule, which dictates how much
/// it costs to execute Move on the network.
module aptos_framework::gas_schedule {
    use std::error;
    use std::string::String;
    use std::vector;

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use aptos_framework::util::from_bytes;
    use aptos_framework::storage_gas::StorageGasConfig;
    use aptos_framework::storage_gas;

    friend aptos_framework::genesis;

    /// The provided gas schedule bytes are empty or invalid
    const EINVALID_GAS_SCHEDULE: u64 = 1;

    struct GasEntry has store, copy, drop {
        key: String,
        val: u64,
    }

    struct GasSchedule has key, copy, drop {
        entries: vector<GasEntry>
    }

    /// Only called during genesis.
    public(friend) fun initialize(aptos_framework: &signer, gas_schedule_blob: vector<u8>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(!vector::is_empty(&gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));

        // TODO(Gas): check if gas schedule is consistent
        move_to<GasSchedule>(aptos_framework, from_bytes(gas_schedule_blob));
    }

    /// This can be called by on-chain governance to update gas schedule.
    public fun set_gas_schedule(aptos_framework: &signer, gas_schedule_blob: vector<u8>) acquires GasSchedule {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(!vector::is_empty(&gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));

        // TODO(Gas): check if gas schedule is consistent
        let gas_schedule = borrow_global_mut<GasSchedule>(@aptos_framework);
        *gas_schedule = from_bytes(gas_schedule_blob);

        // Need to trigger reconfiguration so validator nodes can sync on the updated gas schedule.
        reconfiguration::reconfigure();
    }

    public fun set_storage_gas_config(aptos_framework: &signer, config: StorageGasConfig) {
        storage_gas::set_config(aptos_framework, config);
        // Need to trigger reconfiguration so the VM is guaranteed to load the new gas fee starting from the next
        // transaction.
        reconfiguration::reconfigure();
    }
}
