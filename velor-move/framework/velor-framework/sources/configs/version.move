/// Maintains the version number for the blockchain.
module velor_framework::version {
    use std::error;
    use std::signer;
    use velor_framework::chain_status;
    use velor_framework::config_buffer;

    use velor_framework::reconfiguration;
    use velor_framework::system_addresses;

    friend velor_framework::genesis;
    friend velor_framework::reconfiguration_with_dkg;

    struct Version has drop, key, store {
        major: u64,
    }

    struct SetVersionCapability has key {}

    /// Specified major version number must be greater than current version number.
    const EINVALID_MAJOR_VERSION_NUMBER: u64 = 1;
    /// Account is not authorized to make this change.
    const ENOT_AUTHORIZED: u64 = 2;

    /// Only called during genesis.
    /// Publishes the Version config.
    public(friend) fun initialize(velor_framework: &signer, initial_version: u64) {
        system_addresses::assert_velor_framework(velor_framework);

        move_to(velor_framework, Version { major: initial_version });
        // Give velor framework account capability to call set version. This allows on chain governance to do it through
        // control of the velor framework account.
        move_to(velor_framework, SetVersionCapability {});
    }

    /// Deprecated by `set_for_next_epoch()`.
    ///
    /// WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!
    ///
    /// TODO: update all the tests that reference this function, then disable this function.
    public entry fun set_version(account: &signer, major: u64) acquires Version {
        assert!(exists<SetVersionCapability>(signer::address_of(account)), error::permission_denied(ENOT_AUTHORIZED));
        chain_status::assert_genesis();

        let old_major = borrow_global<Version>(@velor_framework).major;
        assert!(old_major < major, error::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER));

        let config = borrow_global_mut<Version>(@velor_framework);
        config.major = major;

        // Need to trigger reconfiguration so validator nodes can sync on the updated version.
        reconfiguration::reconfigure();
    }

    /// Used in on-chain governances to update the major version for the next epoch.
    /// Example usage:
    /// - `velor_framework::version::set_for_next_epoch(&framework_signer, new_version);`
    /// - `velor_framework::velor_governance::reconfigure(&framework_signer);`
    public entry fun set_for_next_epoch(account: &signer, major: u64) acquires Version {
        assert!(exists<SetVersionCapability>(signer::address_of(account)), error::permission_denied(ENOT_AUTHORIZED));
        let old_major = borrow_global<Version>(@velor_framework).major;
        assert!(old_major < major, error::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER));
        config_buffer::upsert(Version {major});
    }

    /// Only used in reconfigurations to apply the pending `Version`, if there is any.
    public(friend) fun on_new_epoch(framework: &signer) acquires Version {
        system_addresses::assert_velor_framework(framework);
        if (config_buffer::does_exist<Version>()) {
            let new_value = config_buffer::extract_v2<Version>();
            if (exists<Version>(@velor_framework)) {
                *borrow_global_mut<Version>(@velor_framework) = new_value;
            } else {
                move_to(framework, new_value);
            }
        }
    }

    /// Only called in tests and testnets. This allows the core resources account, which only exists in tests/testnets,
    /// to update the version.
    fun initialize_for_test(core_resources: &signer) {
        system_addresses::assert_core_resource(core_resources);
        move_to(core_resources, SetVersionCapability {});
    }

    #[test(velor_framework = @velor_framework)]
    public entry fun test_set_version(velor_framework: signer) acquires Version {
        initialize(&velor_framework, 1);
        assert!(borrow_global<Version>(@velor_framework).major == 1, 0);
        set_version(&velor_framework, 2);
        assert!(borrow_global<Version>(@velor_framework).major == 2, 1);
    }

    #[test(velor_framework = @velor_framework, core_resources = @core_resources)]
    public entry fun test_set_version_core_resources(
        velor_framework: signer,
        core_resources: signer,
    ) acquires Version {
        initialize(&velor_framework, 1);
        assert!(borrow_global<Version>(@velor_framework).major == 1, 0);
        initialize_for_test(&core_resources);
        set_version(&core_resources, 2);
        assert!(borrow_global<Version>(@velor_framework).major == 2, 1);
    }

    #[test(velor_framework = @velor_framework, random_account = @0x123)]
    #[expected_failure(abort_code = 327682, location = Self)]
    public entry fun test_set_version_unauthorized_should_fail(
        velor_framework: signer,
        random_account: signer,
    ) acquires Version {
        initialize(&velor_framework, 1);
        set_version(&random_account, 2);
    }
}
