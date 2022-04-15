/// Maintains information about the set of validators used during consensus.
/// Provides functions to add, remove, and update validators in the
/// validator set.
///
/// > Note: When trying to understand this code, it's important to know that "config"
/// and "configuration" are used for several distinct concepts.
module AptosFramework::ValidatorSet {
    use Std::Errors;
    use Std::Option::{Self, Option};
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Reconfiguration;
    use AptosFramework::SystemAddresses;
    use AptosFramework::Timestamp;
    use AptosFramework::ValidatorConfig;

    /// Information about a Validator Owner.
    struct ValidatorInfo has copy, drop, store {
        /// The address (account) of the Validator Owner
        addr: address,
        /// The voting power of the Validator Owner (currently always 1).
        consensus_voting_power: u64,
        /// Configuration information about the Validator, such as the
        /// Validator Operator, human name, and info such as consensus key
        /// and network addresses.
        config: ValidatorConfig::Config,
        /// The time of last reconfiguration invoked by this validator
        /// in microseconds
        last_config_update_time: u64,
    }

    /// The ValidatorSet struct stores the validator set and crypto scheme in
    /// Reconfiguration. The ValidatorSet struct is stored by Reconfiguration, which publishes a
    /// Reconfiguration<ValidatorSet> resource.
    struct ValidatorSet has key, copy, drop {
        /// The current consensus crypto scheme.
        scheme: u8,
        /// The current validator set.
        validators: vector<ValidatorInfo>,
    }

    /// The `ValidatorSet` resource was not in the required state
    const ECONFIG: u64 = 0;
    /// Tried to add a validator with an invalid state to the validator set
    const EINVALID_PROSPECTIVE_VALIDATOR: u64 = 1;
    /// Tried to add a validator to the validator set that was already in it
    const EALREADY_A_VALIDATOR: u64 = 2;
    /// An operation was attempted on an address not in the vaidator set
    const ENOT_AN_ACTIVE_VALIDATOR: u64 = 3;
    /// The validator operator is not the operator for the specified validator
    const EINVALID_TRANSACTION_SENDER: u64 = 4;
    /// An out of bounds index for the validator set was encountered
    const EVALIDATOR_INDEX: u64 = 5;
    /// Rate limited when trying to update config
    const ECONFIG_UPDATE_RATE_LIMITED: u64 = 6;
    /// Validator set already at maximum allowed size
    const EMAX_VALIDATORS: u64 = 7;
    /// Validator config update time overflows
    const ECONFIG_UPDATE_TIME_OVERFLOWS: u64 = 8;

    /// Number of microseconds in 5 minutes
    const FIVE_MINUTES: u64 = 300000000;

    /// The maximum number of allowed validators in the validator set
    const MAX_VALIDATORS: u64 = 256;

    /// The largest possible u64 value
    const MAX_U64: u64 = 18446744073709551615;

    ///////////////////////////////////////////////////////////////////////////
    // Setup methods
    ///////////////////////////////////////////////////////////////////////////


    /// Publishes the ValidatorSet struct, which contains the current validator set.
    /// Must be invoked by @CoreResources a single time in Genesis.
    public fun initialize_validator_set(
        account: &signer,
    ) {
        Timestamp::assert_genesis();
        SystemAddresses::assert_core_resource(account);

        assert!(!exists<ValidatorSet>(@CoreResources), Errors::already_published(ECONFIG));
        move_to(
            account,
            ValidatorSet {
                scheme: 0,
                validators: Vector::empty(),
            },
        );
    }

    /// Copies a ValidatorSet struct into the ValidatorSet resource
    /// Called by the add, remove, and update functions.
    fun set_validator_system_config(value: ValidatorSet) acquires ValidatorSet {
        Timestamp::assert_operating();
        assert!(
            exists<ValidatorSet>(@CoreResources),
            Errors::not_published(ECONFIG)
        );
        // Updates the ValidatorSet and emits a reconfigure event.
        let config_ref = borrow_global_mut<ValidatorSet>(@CoreResources);
        *config_ref = value;
        Reconfiguration::reconfigure();
    }

    ///////////////////////////////////////////////////////////////////////////
    // Methods operating the Validator Set config callable by the root account
    ///////////////////////////////////////////////////////////////////////////

    /// Adds a new validator to the validator set.
    public(script) fun add_validator(
        account: signer,
        validator_addr: address,
    ) acquires ValidatorSet {
        add_validator_internal(&account, validator_addr);
    }

    public fun add_validator_internal(
        account: &signer,
        validator_addr: address,
    ) acquires ValidatorSet {
        Timestamp::assert_operating();
        SystemAddresses::assert_core_resource(account);

        // A prospective validator must have a validator config resource
        assert!(
            ValidatorConfig::is_valid(validator_addr),
            Errors::invalid_argument(EINVALID_PROSPECTIVE_VALIDATOR)
        );

        // Bound the validator set size
        assert!(
            validator_set_size() < MAX_VALIDATORS,
            Errors::limit_exceeded(EMAX_VALIDATORS)
        );

        let validator_system_config = get_validator_system_config();

        // Ensure that this address is not already a validator
        assert!(
            !is_validator_(validator_addr, &validator_system_config.validators),
            Errors::invalid_argument(EALREADY_A_VALIDATOR)
        );

        // it is guaranteed that the config is non-empty
        let config = ValidatorConfig::get_config(validator_addr);
        Vector::push_back(&mut validator_system_config.validators, ValidatorInfo {
            addr: validator_addr,
            config, // copy the config over to ValidatorSet
            consensus_voting_power: 1,
            last_config_update_time: Timestamp::now_microseconds(),
        });

        set_validator_system_config(validator_system_config);
    }

    /// Removes a validator, aborts unless called by root account
    public(script) fun remove_validator(
        account: signer,
        validator_addr: address,
    ) acquires ValidatorSet {
        remove_validator_internal(
            &account,
            validator_addr,
        );
    }

    public fun remove_validator_internal(
        account: &signer,
        validator_addr: address,
    ) acquires ValidatorSet {
        Timestamp::assert_operating();
        SystemAddresses::assert_core_resource(account);

        let validator_system_config = get_validator_system_config();
        // Ensure that this address is an active validator
        let to_remove_index_vec = get_validator_index_(&validator_system_config.validators, validator_addr);
        assert!(Option::is_some(&to_remove_index_vec), Errors::invalid_argument(ENOT_AN_ACTIVE_VALIDATOR));
        let to_remove_index = *Option::borrow(&to_remove_index_vec);
        // Remove corresponding ValidatorInfo from the validator set
        _  = Vector::swap_remove(&mut validator_system_config.validators, to_remove_index);

        set_validator_system_config(validator_system_config);
    }

    /// Copy the information from ValidatorConfig into the validator set.
    /// This function makes no changes to the size or the members of the set.
    /// If the config in the ValidatorSet changes, it stores the new ValidatorSet
    /// and emits a reconfigurationevent.
    public fun update_config_and_reconfigure(
        validator_operator_account: &signer,
        validator_addr: address,
    ) acquires ValidatorSet {
        Timestamp::assert_operating();
        assert!(
            ValidatorConfig::get_operator(validator_addr) == Signer::address_of(validator_operator_account),
            Errors::invalid_argument(EINVALID_TRANSACTION_SENDER)
        );
        let validator_system_config = get_validator_system_config();
        let to_update_index_vec = get_validator_index_(&validator_system_config.validators, validator_addr);
        assert!(Option::is_some(&to_update_index_vec), Errors::invalid_argument(ENOT_AN_ACTIVE_VALIDATOR));
        let to_update_index = *Option::borrow(&to_update_index_vec);
        let is_validator_info_updated = update_ith_validator_info_(&mut validator_system_config.validators, to_update_index);
        if (is_validator_info_updated) {
            let validator_info = Vector::borrow_mut(&mut validator_system_config.validators, to_update_index);
            assert!(
                validator_info.last_config_update_time <= MAX_U64 - FIVE_MINUTES,
                Errors::limit_exceeded(ECONFIG_UPDATE_TIME_OVERFLOWS)
            );
            assert!(
                Timestamp::now_microseconds() > validator_info.last_config_update_time + FIVE_MINUTES,
                Errors::limit_exceeded(ECONFIG_UPDATE_RATE_LIMITED)
            );
            validator_info.last_config_update_time = Timestamp::now_microseconds();
            set_validator_system_config(validator_system_config);
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Publicly callable APIs: getters
    ///////////////////////////////////////////////////////////////////////////

    /// Get the ValidatorSet configuration from Reconfiguration
    public fun get_validator_system_config(): ValidatorSet acquires ValidatorSet {
        *borrow_global<ValidatorSet>(@CoreResources)
    }

    /// Return true if `addr` is in the current validator set
    public fun is_validator(addr: address): bool acquires ValidatorSet {
        is_validator_(addr, &get_validator_system_config().validators)
    }

    /// Returns validator config. Aborts if `addr` is not in the validator set.
    public fun get_validator_config(addr: address): ValidatorConfig::Config acquires ValidatorSet {
        let validator_system_config = get_validator_system_config();
        let validator_index_vec = get_validator_index_(&validator_system_config.validators, addr);
        assert!(Option::is_some(&validator_index_vec), Errors::invalid_argument(ENOT_AN_ACTIVE_VALIDATOR));
        *&(Vector::borrow(&validator_system_config.validators, *Option::borrow(&validator_index_vec))).config
    }

    /// Return the size of the current validator set
    public fun validator_set_size(): u64 acquires ValidatorSet {
        Vector::length(&get_validator_system_config().validators)
    }

    /// Get the `i`'th validator address in the validator set.
    public fun get_ith_validator_address(i: u64): address acquires ValidatorSet{
        assert!(i < validator_set_size(), Errors::invalid_argument(EVALIDATOR_INDEX));
        Vector::borrow(&get_validator_system_config().validators, i).addr
    }

    ///////////////////////////////////////////////////////////////////////////
    // Private functions
    ///////////////////////////////////////////////////////////////////////////

    /// Get the index of the validator by address in the `validators` vector
    /// It has a loop, so there are spec blocks in the code to assert loop invariants.
    fun get_validator_index_(validators: &vector<ValidatorInfo>, addr: address): Option<u64> {
        let size = Vector::length(validators);
        let i = 0;
        while (i < size) {
            let validator_info_ref = Vector::borrow(validators, i);
            if (validator_info_ref.addr == addr) {
                return Option::some(i)
            };
            i = i + 1;
        };
        return Option::none()
    }

    /// Updates *i*th validator info, if nothing changed, return false.
    /// This function never aborts.
    fun update_ith_validator_info_(validators: &mut vector<ValidatorInfo>, i: u64): bool {
        let size = Vector::length(validators);
        // This provably cannot happen, but left it here for safety.
        if (i >= size) {
            return false
        };
        let validator_info = Vector::borrow_mut(validators, i);
        // "is_valid" below should always hold based on a global invariant later
        // in the file (which proves if we comment out some other specifications),
        // but it is left here for safety.
        if (!ValidatorConfig::is_valid(validator_info.addr)) {
            return false
        };
        let new_validator_config = ValidatorConfig::get_config(validator_info.addr);
        // check if information is the same
        let config_ref = &mut validator_info.config;
        if (config_ref == &new_validator_config) {
            return false
        };
        *config_ref = new_validator_config;
        true
    }

    /// Private function checks for membership of `addr` in validator set.
    fun is_validator_(addr: address, validators_vec_ref: &vector<ValidatorInfo>): bool {
        Option::is_some(&get_validator_index_(validators_vec_ref, addr))
    }
}
