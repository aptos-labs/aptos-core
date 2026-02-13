/// Provides the config related to leader ban registry
module supra_framework::leader_ban_registry_config {
    use std::error;
    use std::option;
    use std::option::Option;
    use std::vector;
    use supra_std::decode_bcs;
    use supra_framework::config_buffer;
    use supra_framework::system_addresses;
    #[test_only]
    use std::signer;

    friend supra_framework::genesis;
    friend supra_framework::reconfiguration_with_dkg;

    #[test_only]
    friend supra_framework::test_leader_ban_registry_config;
    #[test_only]
    friend supra_framework::test_leader_ban_registry;

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;
    /// The provided on chain config version should be equal or greater than existing
    const EINVALID_VERSION: u64 = 2;
    /// Decoding from version bytes failed
    const EINVALID_VERSION_BYTES: u64 = 3;
    /// The BanRegistryParameters already initialised
    const EALREADY_INITIALISED: u64 = 4;

    /// Holds ban registry parameters bytes and it's version
    struct BanRegistryParameters has drop, key, store {
        /// Denotes config bcs bytes
        config: vector<u8>,
        /// Denotes config version
        version: u8
    }

    /// Ban registry parameters v0
    struct BanRegistryParametersV0 has drop, key, store {
        /// Denotes initial election count denied
        initial_elections_denied: u8,
        /// Denotes max election count denied
        max_elections_denied: u32,
        /// Denotes the minimum number of validators that must remain eligible for proposal. This
        /// helps to preserve liveness in the presence of extended periods of network asynchrony.
        minimum_unbanned_proposers: u8,
        /// Denotes the number of elections a validator must serve on probation after ban expires.
        /// The ban duration compounds each time a validator is banned whilst on probation, and
        /// resets to the base duration if the validator passes probation.
        probation_elections: u8
    }

    /// Publishes the BanRegistryParameters config.
    public(friend) fun initialize(
        supra_framework: &signer, config: vector<u8>
    ) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(vector::length(&config) != 0, error::invalid_argument(EINVALID_CONFIG));
        assert!(
            !exists<BanRegistryParameters>(@supra_framework),
            error::already_exists(EALREADY_INITIALISED)
        );
        let v0 = deserialise_v0_params(config);
        if (option::is_none(&v0)) {
            abort error::invalid_argument(EINVALID_VERSION_BYTES)
        };
        // we always init with version 0
        move_to(supra_framework, BanRegistryParameters { config, version: 0 });
        let v0_params = option::extract(&mut v0);
        move_to(supra_framework, v0_params);
    }

    /// This can be called by on-chain governance to update on-chain configs for the next epoch.
    /// Example usage:
    /// ```
    /// supra_framework::leader_ban_registry_config::set_for_next_epoch(&framework_signer, some_config_bytes, version);
    /// supra_framework::supra_governance::reconfigure(&framework_signer);
    /// ```
    public fun set_for_next_epoch(
        account: &signer, config: vector<u8>, version: u8
    ) acquires BanRegistryParameters {
        system_addresses::assert_supra_framework(account);
        assert!(vector::length(&config) != 0, error::invalid_argument(EINVALID_CONFIG));
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_params =
                borrow_global<BanRegistryParameters>(@supra_framework);
            assert!(
                version >= ban_registry_params.version,
                error::invalid_argument(EINVALID_VERSION)
            );
        };
        std::config_buffer::upsert<BanRegistryParameters>(
            BanRegistryParameters { config, version }
        );
    }

    /// Only used in reconfigurations to apply the pending `BanRegistryParameters`, if there is any.
    public(friend) fun on_new_epoch(
        framework: &signer
    ) acquires BanRegistryParameters, BanRegistryParametersV0 {
        system_addresses::assert_supra_framework(framework);
        if (config_buffer::does_exist<BanRegistryParameters>()) {
            let new_config = config_buffer::extract<BanRegistryParameters>();
            if (exists<BanRegistryParameters>(@supra_framework)) {
                *borrow_global_mut<BanRegistryParameters>(@supra_framework) = new_config;
            } else {
                move_to(framework, new_config);
            };
            let params = borrow_global<BanRegistryParameters>(@supra_framework);
            if (params.version == 0) {
                let v0 = deserialise_v0_params(params.config);
                // This is to prevent abort on new epoch if deserialise failed.
                if (option::is_some(&v0)) {
                    let ban_registry_params = option::extract(&mut v0);
                    if (exists<BanRegistryParametersV0>(@supra_framework)) {
                        *borrow_global_mut<BanRegistryParametersV0>(@supra_framework) = ban_registry_params;
                    } else {
                        move_to(framework, ban_registry_params);
                    }
                }
            }
            // later version can be assigned here
        }
    }

    #[view]
    public fun get_ban_registry_params(): (vector<u8>, u8) acquires BanRegistryParameters {
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_config =
                borrow_global<BanRegistryParameters>(@supra_framework);
            return (ban_registry_config.config, ban_registry_config.version)
        };
        (vector::empty(), 0)
    }

    #[view]
    public fun get_ban_registry_params_v0(): (u8, u32, u8, u8) acquires BanRegistryParameters, BanRegistryParametersV0 {
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_config =
                borrow_global<BanRegistryParameters>(@supra_framework);
            if (ban_registry_config.version == 0) {
                let ban_registry_params =
                    borrow_global<BanRegistryParametersV0>(@supra_framework);
                return (
                    ban_registry_params.initial_elections_denied,
                    ban_registry_params.max_elections_denied,
                    ban_registry_params.minimum_unbanned_proposers,
                    ban_registry_params.probation_elections
                )
            }
        };
        (0, 0, 0, 0)
    }

    /// Provide initial election denied value
    public fun get_initial_elections_denied(): u8 acquires BanRegistryParameters, BanRegistryParametersV0 {
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_config =
                borrow_global<BanRegistryParameters>(@supra_framework);
            if (ban_registry_config.version == 0) {
                let ban_registry_params =
                    borrow_global<BanRegistryParametersV0>(@supra_framework);
                return ban_registry_params.initial_elections_denied
            }
        };
        0
    }

    /// Provide max election denied value
    public fun get_max_elections_denied(): u32 acquires BanRegistryParameters, BanRegistryParametersV0 {
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_config =
                borrow_global<BanRegistryParameters>(@supra_framework);
            if (ban_registry_config.version == 0) {
                let ban_registry_params =
                    borrow_global<BanRegistryParametersV0>(@supra_framework);
                return ban_registry_params.max_elections_denied
            }
        };
        0
    }

    /// Provide minimum unbanned proposers value
    public fun get_minimum_unbanned_proposers(): u8 acquires BanRegistryParameters, BanRegistryParametersV0 {
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_config =
                borrow_global<BanRegistryParameters>(@supra_framework);
            if (ban_registry_config.version == 0) {
                let ban_registry_params =
                    borrow_global<BanRegistryParametersV0>(@supra_framework);
                return ban_registry_params.minimum_unbanned_proposers
            }
        };
        0
    }

    /// Provide probation elections value
    public fun get_probation_elections(): u8 acquires BanRegistryParameters, BanRegistryParametersV0 {
        if (exists<BanRegistryParameters>(@supra_framework)) {
            let ban_registry_config =
                borrow_global<BanRegistryParameters>(@supra_framework);
            if (ban_registry_config.version == 0) {
                let ban_registry_params =
                    borrow_global<BanRegistryParametersV0>(@supra_framework);
                return ban_registry_params.probation_elections
            }
        };
        0
    }

    /// Decoding bytes to `BanRegistryParametersV0` using bcs
    fun deserialise_v0_params(bytes: vector<u8>): Option<BanRegistryParametersV0> {
        let bcs_bytes = decode_bcs::new(bytes);
        let initial_elections_denied: u8 = decode_bcs::peel_u8(&mut bcs_bytes);
        let max_elections_denied: u32 = decode_bcs::peel_u32(&mut bcs_bytes);
        let minimum_unbanned_proposers: u8 = decode_bcs::peel_u8(&mut bcs_bytes);
        let probation_elections: u8 = decode_bcs::peel_u8(&mut bcs_bytes);
        // making sure no bytes left to decode means correct parameter version
        if (vector::length(&decode_bcs::into_remainder_bytes(bcs_bytes)) == 0) {
            return option::some(
                BanRegistryParametersV0 {
                    initial_elections_denied,
                    max_elections_denied,
                    minimum_unbanned_proposers,
                    probation_elections
                }
            )
        };
        option::none<BanRegistryParametersV0>()
    }

    #[test_only]
    public fun get_test_ban_registry_params_v0(): BanRegistryParametersV0 {
        BanRegistryParametersV0 {
            initial_elections_denied: 1,
            max_elections_denied: 5,
            minimum_unbanned_proposers: 2,
            probation_elections: 1
        }
    }

    #[test_only]
    public fun get_custom_ban_registry_params_v0(
        initial_elections_denied: u8,
        max_elections_denied: u32,
        minimum_unbanned_proposers: u8,
        probation_elections: u8
    ): BanRegistryParametersV0 {
        BanRegistryParametersV0 {
            initial_elections_denied,
            max_elections_denied,
            minimum_unbanned_proposers,
            probation_elections
        }
    }

    #[test_only]
    public fun check_ban_registry_params_exist(sender: &signer): bool {
        exists<BanRegistryParameters>(signer::address_of(sender))
    }

    #[test_only]
    public fun check_ban_registry_params_v0_exist(sender: &signer): bool {
        exists<BanRegistryParameters>(signer::address_of(sender))
    }
}
