/// This module defines structs and methods to initialize VM configurations,
/// including different costs of running the VM.
module aptos_framework::gas_schedule {
    use std::error;
    use std::string::String;

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::util::from_bytes;

    /// Error with config
    const ECONFIG: u64 = 1;
    /// The provided gas constants were inconsistent.
    const EGAS_CONSTANT_INCONSISTENCY: u64 = 2;

    struct GasEntry has store, copy, drop {
        key: String,
        val: u64,
    }

    struct GasSchedule has key, copy, drop {
        entries: vector<GasEntry>
    }

    public fun initialize(account: &signer, gas_schedule_blob: vector<u8>) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<GasSchedule>(@aptos_framework),
            error::already_exists(ECONFIG)
        );

        // TODO(Gas): check if gas schedule is consistent

        move_to<GasSchedule>(account, from_bytes(gas_schedule_blob));
    }

    public entry fun set_gas_schedule(account: &signer, gas_schedule_blob: vector<u8>) acquires GasSchedule {
        timestamp::assert_operating();
        system_addresses::assert_core_resource(account);

        assert!(exists<GasSchedule>(@aptos_framework), error::not_found(ECONFIG));

        // TODO(Gas): check if gas schedule is consistent

        let gas_schedule = borrow_global_mut<GasSchedule>(@aptos_framework);
        *gas_schedule = from_bytes(gas_schedule_blob);

        reconfiguration::reconfigure();
    }
}
