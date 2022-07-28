/// This module defines structs and methods to initialize VM configurations,
/// including different costs of running the VM.
module aptos_framework::gas_schedule {
    use std::error;
    use std::string::String;
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::util::from_bytes;

    /// Error with config
    const ECONFIG: u64 = 0;
    /// The provided gas constants were inconsistent.
    const EGAS_CONSTANT_INCONSISTENCY: u64 = 1;

    struct GasEntry has store, copy, drop {
        key: String,
        val: u64,
    }

    public fun new_entry(key: String, val: u64): GasEntry {
        GasEntry { key, val }
    }

    struct GasSchedule has key, copy, drop {
        entries: vector<GasEntry>
    }

    public fun new_gas_schedule(): GasSchedule {
        GasSchedule {
            entries: vector::empty(),
        }
    }

    public fun index_of(gas_schedule: &GasSchedule, key: &String): Option<u64> {
        let len = vector::length(&gas_schedule.entries);

        let i = 0;
        while (i < len) {
            let entry = vector::borrow(&gas_schedule.entries, i);
            if (&entry.key == key) {
                return option::some(i)
            };
            i = i + 1;
        };

        option::none()
    }

    public fun get(gas_schedule: &GasSchedule, key: &String): Option<u64> {
        let idx_opt = index_of(gas_schedule, key);

        if (option::is_none(&idx_opt)) {
            return option::none()
        };

        let idx = option::destroy_some(idx_opt);
        option::some(vector::borrow(&gas_schedule.entries, idx).val)
    }

    public fun set(gas_schedule: &mut GasSchedule, key: String, val: u64): Option<u64> {
        let idx_opt = index_of(gas_schedule, &key);

        if (option::is_none(&idx_opt)) {
            vector::push_back(&mut gas_schedule.entries, GasEntry { key, val });
            option::none()
        }
        else {
            let idx = option::destroy_some(idx_opt);
            let entry = vector::borrow_mut(&mut gas_schedule.entries, idx);
            let old = entry.val;
            entry.val = val;
            option::some(old)
        }
    }

    public fun remove(gas_schedule: &mut GasSchedule, key: &String): Option<u64> {
        let idx_opt = index_of(gas_schedule, key);

        if (option::is_none(&idx_opt)) {
            option::none()
        }
        else {
            let idx = option::destroy_some(idx_opt);
            let len = vector::length(&gas_schedule.entries);

            if (idx + 1 < len) {
                vector::swap(&mut gas_schedule.entries, idx, len - 1);
            };

            option::some(vector::pop_back(&mut gas_schedule.entries).val)
        }
    }

    public fun initialize(account: &signer, gas_schedule_blob: vector<u8>) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<GasSchedule>(@aptos_framework),
            error::already_exists(ECONFIG)
        );

        let gas_schedule: GasSchedule = from_bytes(gas_schedule_blob);

        move_to(account, gas_schedule);
    }

    public entry fun set_gas_schedule(account: &signer, gas_schedule_blob: vector<u8>) acquires GasSchedule {
        timestamp::assert_operating();
        system_addresses::assert_core_resource(account);

        assert!(exists<GasSchedule>(@aptos_framework), error::not_found(ECONFIG));

        let gas_schedule = borrow_global_mut<GasSchedule>(@aptos_framework);
        *gas_schedule = from_bytes(gas_schedule_blob);

        reconfiguration::reconfigure();
    }

    /*
    /// Initialize the table under the root account
    public fun initialize(
        account: &signer,
        gas_schedule: GasSchedule,
    ) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<GasSchedule>(@aptos_framework),
            error::already_exists(ECONFIG)
        );

        let gas_constants = GasConstants {
            global_memory_per_byte_cost: 4,
            global_memory_per_byte_write_cost: 9,
            min_transaction_gas_units: 600,
            large_transaction_cutoff: 600,
            intrinsic_gas_per_byte: 8,
            maximum_number_of_gas_units: 4000000,
            min_price_per_gas_unit,
            max_price_per_gas_unit: 10000,
            max_transaction_size_in_bytes: 262144,
            gas_unit_scaling_factor: 1000,
            default_account_size: 800,
        };

        move_to(
            account,
            VMConfig {
                gas_schedule: GasSchedule {
                    instruction_schedule,
                    native_schedule,
                    gas_constants,
                }
            },
        );
    }
    */

    /*
    public entry fun set_gas_constants(
        account: signer,
        global_memory_per_byte_cost: u64,
        global_memory_per_byte_write_cost: u64,
        min_transaction_gas_units: u64,
        large_transaction_cutoff: u64,
        intrinsic_gas_per_byte: u64,
        maximum_number_of_gas_units: u64,
        min_price_per_gas_unit: u64,
        max_price_per_gas_unit: u64,
        max_transaction_size_in_bytes: u64,
        gas_unit_scaling_factor: u64,
        default_account_size: u64,
    ) acquires VMConfig {
        timestamp::assert_operating();
        system_addresses::assert_core_resource(&account);

        assert!(
            min_price_per_gas_unit <= max_price_per_gas_unit,
            error::invalid_argument(EGAS_CONSTANT_INCONSISTENCY)
        );
        assert!(
            min_transaction_gas_units <= maximum_number_of_gas_units,
            error::invalid_argument(EGAS_CONSTANT_INCONSISTENCY)
        );

        assert!(exists<VMConfig>(@aptos_framework), error::not_found(ECONFIG));

        let gas_constants = &mut borrow_global_mut<VMConfig>(@aptos_framework).gas_schedule.gas_constants;

        gas_constants.global_memory_per_byte_cost       = global_memory_per_byte_cost;
        gas_constants.global_memory_per_byte_write_cost = global_memory_per_byte_write_cost;
        gas_constants.min_transaction_gas_units         = min_transaction_gas_units;
        gas_constants.large_transaction_cutoff          = large_transaction_cutoff;
        gas_constants.intrinsic_gas_per_byte            = intrinsic_gas_per_byte;
        gas_constants.maximum_number_of_gas_units       = maximum_number_of_gas_units;
        gas_constants.min_price_per_gas_unit            = min_price_per_gas_unit;
        gas_constants.max_price_per_gas_unit            = max_price_per_gas_unit;
        gas_constants.max_transaction_size_in_bytes     = max_transaction_size_in_bytes;
        gas_constants.gas_unit_scaling_factor           = gas_unit_scaling_factor;
        gas_constants.default_account_size              = default_account_size;

        reconfiguration::reconfigure();
    }
    */
}
