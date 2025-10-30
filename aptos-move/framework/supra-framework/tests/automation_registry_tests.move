#[test_only]
module std::automation_registry_tests {
    use std::bcs;
    use std::features;
    use std::signer;
    use std::signer::address_of;
    use std::vector;
    use aptos_std::debug::print;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::account;
    use supra_framework::account::create_signer_for_test;
    use supra_framework::multisig_account;
    use supra_framework::automation_registry::{
        check_task_priority, check_cycle_state_and_duration, check_next_task_index_to_be_processed,
        calculate_automation_fee_multiplier_for_committed_occupancy, AutomationRegistryConfigV2,
    };
    use supra_framework::coin;
    use supra_framework::config_buffer;
    use supra_framework::supra_coin;
    use supra_framework::timestamp;
    use supra_framework::automation_registry;

    /// Invalid expiry time: it cannot be earlier than the current time
    const EINVALID_EXPIRY_TIME: u64 = 1;
    /// Expiry time does not go beyond upper cap duration
    const EEXPIRY_TIME_UPPER: u64 = 2;
    /// Expiry time must be after the start of the next cycle
    const EEXPIRY_BEFORE_NEXT_CYCLE: u64 = 3;
    /// Invalid gas price: it cannot be zero
    const EINVALID_GAS_PRICE: u64 = 4;
    /// Invalid max gas amount for automated task: it cannot be zero
    const EINVALID_MAX_GAS_AMOUNT: u64 = 5;
    /// Task with provided task index not found
    const EAUTOMATION_TASK_NOT_FOUND: u64 = 6;
    /// Gas amount must not go beyond upper cap limit
    const EGAS_AMOUNT_UPPER: u64 = 7;
    /// Unauthorized access: the caller is not the owner of the task
    const EUNAUTHORIZED_TASK_OWNER: u64 = 8;
    /// Transaction hash that registering current task is invalid. Length should be 32.
    const EINVALID_TXN_HASH: u64 = 9;
    /// Current committed gas amount is greater than the automation gas limit.
    const EUNACCEPTABLE_AUTOMATION_GAS_LIMIT: u64 = 10;
    /// Task is already cancelled.
    const EALREADY_CANCELLED: u64 = 11;
    /// The gas committed for next epoch value is overflow after adding new max gas
    const EGAS_COMMITTEED_VALUE_OVERFLOW: u64 = 12;
    /// The gas committed for next epoch value is underflow after remove old max gas
    const EGAS_COMMITTEED_VALUE_UNDERFLOW: u64 = 13;
    /// Invalid number of auxiliary data.
    const EINVALID_AUX_DATA_LENGTH: u64 = 14;
    /// Supra native automation feature is not initialized or enabled
    const EDISABLED_AUTOMATION_FEATURE: u64 = 15;
    /// Insufficient balance in the resource wallet for withdrawal
    const EINSUFFICIENT_BALANCE: u64 = 16;
    /// Requested amount exceeds the locked balance
    const EREQUEST_EXCEEDS_LOCKED_BALANCE: u64 = 17;
    /// Current automation cycle interval is greater than specified task duration cap.
    const EUNACCEPTABLE_TASK_DURATION_CAP: u64 = 18;
    /// Congestion threshold should not exceed 100.
    const EMAX_CONGESTION_THRESHOLD: u64 = 19;
    /// Congestion exponent must be non-zero.
    const ECONGESTION_EXP_NON_ZERO: u64 = 20;
    /// Automation fee capacity for the epoch should not be less than estimated one.
    const EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH: u64 = 21;
    /// Automation registry max gas capacity cannot be zero.
    const EREGISTRY_MAX_GAS_CAP_NON_ZERO: u64 = 22;
    /// Registry task capacity has reached.
    const EREGISTRY_IS_FULL: u64 = 23;
    /// Task registration is currently disabled.
    const ETASK_REGISTRATION_DISABLED: u64 = 24;
    /// Task index list is empty.
    const EEMPTY_TASK_INDEXES: u64 = 25;
    /// Resource Account does not have sufficient balance to process the refund for the specified task.
    const EINSUFFICIENT_BALANCE_FOR_REFUND: u64 = 26;
    /// Failed to unlock/refund deposit for a task. Internal error, for more details see emitted error events.
    const EDEPOSIT_REFUND: u64 = 27;
    /// Failed to unlock/refund epoch fee for a task. Internal error, for more details see emitted error events.
    const EEPOCH_FEE_REFUND: u64 = 28;
    /// Deprecated function call since cycle based automation release.
    const EDEPRECATED_SINCE_V2: u64 = 29;
    /// Automation cycle duration cannot be zero.
    const ECYCLE_DURATION_NON_ZERO: u64 = 30;
    /// Attempt to do migration to cycle based automation which is already enabled.
    const EINVALID_MIGRATION_ACTION: u64 = 31;
    /// Attempt to register an automation task while cycle transition is in progress.
    const ECYCLE_TRANSITION_IN_PROGRESS: u64 = 32;
    /// Attempt to run operation in invalid registry state.
    const EINVALID_REGISTRY_STATE: u64 = 33;
    /// The tasks are requested to be processed for invalid cycle.
    const EINVALID_INPUT_CYCLE_INDEX: u64 = 34;
    /// Attempt to process a task when expected list of the tasks has been alrady processed.
    const EINCONSISTENT_TRANSITION_STATE: u64 = 35;
    /// The out of order task processing has been identified during transition.
    const EOUT_OF_ORDER_TASK_PROCESSING_REQUEST: u64 = 36;
    /// Automation registry max gas capacity for system tasks cannot be zero.
    const EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS: u64 = 37;
    /// Current automation cycle interval is greater than specified system task duration cap.
    const EUNACCEPTABLE_SYS_TASK_DURATION_CAP: u64 = 38;
    /// Task type specified as first elemeny of aux-data should have length 1
    const EINVALID_TASK_TYPE_LENGTH: u64 = 39;
    /// Invalid task type value. Supported 1 for user submitted tasks, 0 for system submitted tasks.
    const EINVALID_TASK_TYPE: u64 = 40;
    /// Attempt to register a system task with unauthorized account.
    const EUNAUTHORIZED_SYSTEM_ACCOUNT: u64 = 41;
    /// Attempt to register a system task with unauthorized account.
    const ESYSTEM_AUTOMATION_TASK_NOT_FOUND: u64 = 42;
    /// Type of the registered task does not match the expected one.
    const EREGISTERED_TASK_INVALID_TYPE: u64 = 43;
    /// Attempt to run an unsupported action for a task.
    const EUNSUPPORTED_TASK_OPERATION: u64 = 44;
    /// Current committed gas amount by system tasks is greater than the new system automation gas limit.
    const EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT: u64 = 45;
    /// Automation registry max gas capacity for  system tasks cannot be zero.
    const EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO: u64 = 46;
    /// The input address is not identified as multisig account.
    const EUNKNOWN_MULTISIG_ADDRESS: u64 = 47;

    /// Constants describing CYCLE state.
    /// State transition flow is:
    /// CYCLE_READY -> CYCLE_STARTED
    /// CYCLE_STARTED -> { CYCLE_FINISHED, CYCLE_SUSPENDED }
    /// CYCLE_FINISHED ->  CYCLE_STARTED
    /// CYCLE_SUSPENDED -> { CYCLE_READY, STARTED }
    const CYCLE_READY: u8 = 0;
    /// Triggered eigther when SUPRA_NATIVE_AUTOMATION feature is enabled or by registry when cycle transition is completed.
    const CYCLE_STARTED: u8 = 1;
    /// Triggered when cycle end is identified.
    const CYCLE_FINISHED: u8 = 2;
    /// State describing the entire lifecycle of automation being suspended.
    /// Triggered when SUPRA_NATIVE_AUTOMATION feature is disabled.
    const CYCLE_SUSPENDED: u8 = 3;

    /// Constants describing task state.
    const PENDING: u8 = 0;
    const ACTIVE: u8 = 1;
    const CANCELLED: u8 = 2;

    /// Constants decribing the task type, USER SUBMITTED TASK (UST - 1), GOVERNANCE SUBMITTED TASK(GST - 2)
    const UST: u8 = 1;
    const GST: u8 = 2;

    /// Defines divisor for refunds of deposit fees with penalty
    /// Factor of `2` suggests that `1/2` of the deposit will be refunded.
    const REFUND_FACTOR: u64 = 2;


    // CONSTANTS utilized during testing
    const AUTOMATION_MAX_GAS_TEST: u64 = 100_000_000;
    const TTL_UPPER_BOUND_TEST: u64 = 2_626_560;
    const AUTOMATION_BASE_FEE_TEST: u64 = 1000;
    const FLAT_REGISTRATION_FEE_TEST: u64 = 1_000_000;
    const CONGESTION_THRESHOLD_TEST: u8 = 80;
    const CONGESTION_BASE_FEE_TEST: u64 = 100;
    const CONGESTION_EXPONENT_TEST: u8 = 6;
    const TASK_CAPACITY_TEST: u16 = 500;
    const SYS_TASK_CAPACITY_TEST: u16 = 100;
    const SYS_AUTOMATION_MAX_GAS_TEST: u64 = 100_000;
    /// Value deinfed in seconds
    const SYS_TASK_DURATION_CAP_IN_SECS: u64 = 1_626_560;
    /// Value defined in microsecond
    const EPOCH_INTERVAL_FOR_TEST_IN_SECS: u64 = 7200;
    const PARENT_HASH: vector<u8> = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    const PAYLOAD: vector<u8> = x"0102030405060708090a0b0c0d0e0f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20101112131415161718191a1b1c1d1e1f20";
    const AUX_DATA: vector<vector<u8>> = vector[vector[1], vector[]];
    const SYS_AUX_DATA: vector<vector<u8>> = vector[vector[2], vector[]];
    const ACCOUNT_BALANCE: u64 = 10_000_000_000;
    const REGISTRY_DEFAULT_BALANCE: u64 = 100_000_000_000;

    const HAS_TRANSITION_STATE: bool = true;

    /// Initializes registry, config buffer, sets time,  and mints registry account
    fun prepare_for_tests(supra_framework: &signer) {

        timestamp::set_time_has_started_for_testing(supra_framework);

        automation_registry::initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
        supra_coin::mint(supra_framework, automation_registry::get_registry_fee_address(), REGISTRY_DEFAULT_BALANCE);
        config_buffer::initialize(supra_framework);
    }

    /// Initializes registry without enabling SUPRA_NATIVE_AUTOMATION and SUPRA_AUTOMATION_V2 feature flags
    fun initialize_registry_only_test(supra_framework: &signer) {
        let (burn_cap, mint_cap) = supra_coin::initialize_for_test(supra_framework);
        prepare_for_tests(supra_framework);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);

    }

    fun toggle_feature_flag(supra_framework: &signer, enable: bool) {
        let flag = vector[features::get_supra_native_automation_feature()];
        toggle_custom_feature_flags(supra_framework, flag, enable);
    }

    fun toggle_custom_feature_flags(supra_framework: &signer, flags: vector<u64>, enable: bool) {
        if (enable) {
            features::change_feature_flags_for_testing(supra_framework,
                flags,
                vector::empty<u64>());
        } else {
            features::change_feature_flags_for_testing(supra_framework,
                vector::empty<u64>(),
                flags)
        }
    }

    /// Initializes registry. enables SUPRA_NATIVE_AUTOMATION feature flag and initialize config-buffer
    fun initialize_registry_test(supra_framework: &signer, user: &signer) {
        let (burn_cap, mint_cap) = supra_coin::initialize_for_test(supra_framework);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
        toggle_feature_flag(supra_framework, true);
        toggle_custom_feature_flags(supra_framework, vector[features::get_supra_automation_v2_feature()], true);
        prepare_for_tests(supra_framework);

        topup_account(supra_framework, user, ACCOUNT_BALANCE);

    }

    /// Create a multisig account the input signer as owner of it.
    fun setup_multisig_account(supra_framework: &signer, user: &signer): (address, signer) {
        let multisig_address = multisig_account::get_next_multisig_account_address(address_of(user));
        let multisig_signer = create_signer_for_test(multisig_address);
        topup_account(supra_framework, &multisig_signer, ACCOUNT_BALANCE);

        multisig_account::create_with_owners(
            user,
            vector[],
            1,
            vector[],
            vector[],
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        (multisig_address, multisig_signer)
    }

    /// Initializes registry. enables SUPRA_NATIVE_AUTOMATION feature flag and initialize config-buffer
    fun initialize_sys_registry_test(supra_framework: &signer, user: &signer, multisig_user: &signer): (address, signer) {
        initialize_registry_test(supra_framework, user);
        topup_account(supra_framework, multisig_user, ACCOUNT_BALANCE);
        setup_multisig_account(supra_framework, multisig_user)
    }

    fun topup_account(supra_framework: &signer, account_signer: &signer, balance: u64) {

        let addr = signer::address_of(account_signer);
        account::create_account_for_test(addr);
        coin::register<SupraCoin>(account_signer);
        supra_coin::mint(supra_framework, addr, balance);

    }


    fun check_account_balance(
        account: address,
        expected_balance: u64,
    ) {
        let current_balance = coin::balance<SupraCoin>(account);
        assert!(current_balance == expected_balance, current_balance);
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = EUNACCEPTABLE_TASK_DURATION_CAP, location = automation_registry)]
    fun test_initialization_with_invalid_task_duration(
        supra_framework: &signer,
    ) {
        automation_registry::initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = EREGISTRY_MAX_GAS_CAP_NON_ZERO, location = automation_registry)]
    fun test_initialization_with_invalid_registry_max_gas_cap(
        supra_framework: &signer,
    ) {
        automation_registry::initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            0,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = ECONGESTION_EXP_NON_ZERO, location = automation_registry)]
    fun test_initialization_with_invalid_congestion_exponent(
        supra_framework: &signer,
    ) {
        automation_registry::initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            0,
            TASK_CAPACITY_TEST,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = EMAX_CONGESTION_THRESHOLD, location = automation_registry)]
    fun test_initialization_with_invalid_threshold_percentage(
        supra_framework: &signer,
    ) {
        automation_registry::initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            200,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(supra_framework = @supra_framework, user = @0x1cafe)]
    fun test_task_registration(
        supra_framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(supra_framework, user);
        automation_registry::register_with_state(user,  1000, 100_000_000, 86400, PENDING);
    }

    #[test(supra_framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EDISABLED_AUTOMATION_FEATURE, location = automation_registry)]
    fun test_registration_with_partial_initialization(
        supra_framework: &signer,
        user: &signer
    ) {
        initialize_registry_only_test(supra_framework);

        automation_registry::register_with_state(user,  1000, 100_000_000, 86400, PENDING);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_update_config_success_update(
        framework: &signer, user: &signer
    ) {

        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            50,
            1000,
            86400,
            PENDING
        );
        // Next epoch gas committed gas is less than the new limit value.
        // Configuration parameter will update after on new epoch
        automation_registry::update_config_v2(framework,
            1_626_560,
            75,
            1005,
            700000000,
            70,
            2000,
            5,
            200,
            1000,
            2000,
            1000,
            100,
        );
        automation_registry::check_automation_configuration(
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        let expected_cycle_duration_on_cycle_end = automation_registry::check_cycle_state_and_duration(
            CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE
        );
        automation_registry::monitor_cycle_end();

        automation_registry::check_cycle_state_and_duration(
            CYCLE_FINISHED, expected_cycle_duration_on_cycle_end, HAS_TRANSITION_STATE
        );

        automation_registry::check_cycle_new_duration(1000);

        automation_registry::check_automation_configuration(
            1_626_560,
            75,
            1005,
            700000000,
            70,
            2000,
            5,
            200,
            1000,
            2000,
            1000,
            100,
        );

    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_automation_gas_limit_update_corner_case(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            50,
            1000,
            86400,
            PENDING
        );

        // Next epoch gas committed gas is greater than the new limit value.
        automation_registry::update_config_v2(
            framework,
            TTL_UPPER_BOUND_TEST,
            50,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe, multisig_user = @0xad123)]
    fun check_sys_automation_gas_limit_update_corner_case(
        framework: &signer,
        user: &signer,
        multisig_user: &signer,
    ) {
        let (multisig_address, multisig_signer) = initialize_sys_registry_test(framework, user, multisig_user);

        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            50,
            86400,
            PENDING,
        );

        // Next epoch gas committed gas is less or equal than the new limit value.
        automation_registry::update_config_v2(
            framework,
            TTL_UPPER_BOUND_TEST,
            50,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            50,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNACCEPTABLE_AUTOMATION_GAS_LIMIT, location = automation_registry)]
    fun check_automation_gas_limit_failed_update(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            50,
            1000,
            86400,
            PENDING
        );

        // Next epoch gas committed gas is greater than the new limit value.
        automation_registry::update_config_v2(
            framework,
            TTL_UPPER_BOUND_TEST,
            45,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe, multisig_user = @0xad123)]
    #[expected_failure(abort_code = EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT, location = automation_registry)]
    fun check_sys_automation_gas_limit_failed_update(
        framework: &signer,
        user: &signer,
        multisig_user: &signer,
    ) {
        let (multisig_address, multisig_signer) =
            initialize_sys_registry_test(framework, user, multisig_user);

        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            50,
            86400,
            PENDING,
        );

        // Next epoch gas committed gas is less or equal than the new limit value.
        automation_registry::update_config_v2(
            framework,
            TTL_UPPER_BOUND_TEST,
            50,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            45,
            SYS_TASK_CAPACITY_TEST
        );
    }
    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNACCEPTABLE_TASK_DURATION_CAP, location = automation_registry)]
    fun check_config_udpate_with_invalid_task_duration_cap(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        automation_registry::update_config_v2(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNACCEPTABLE_SYS_TASK_DURATION_CAP, location = automation_registry)]
    fun check_config_udpate_with_invalid_system_task_duration_cap(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than cycle length
        automation_registry::update_config_v2(
            framework,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }
    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EMAX_CONGESTION_THRESHOLD, location = automation_registry)]
    fun check_config_udpate_with_max_congestion_threshold(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        automation_registry::update_config_v2(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            150,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ECONGESTION_EXP_NON_ZERO, location = automation_registry)]
    fun check_config_udpate_with_invalid_congestion_exponent(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        automation_registry::update_config_v2(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            0,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EREGISTRY_MAX_GAS_CAP_NON_ZERO, location = automation_registry)]
    fun check_config_udpate_with_invalid_registry_max_gas_cap(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::update_config_v2(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            0,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO, location = automation_registry)]
    fun check_config_udpate_with_invalid_registry_system_max_gas_cap(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::update_config_v2(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            0,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ECYCLE_DURATION_NON_ZERO, location = automation_registry)]
    fun check_config_udpate_with_invalid_cycle_duration(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        automation_registry::update_config_v2(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            0,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_registration(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let max_gas_amount = 10;
        let estimated_fee = automation_registry::estimate_automation_fee(max_gas_amount);
        automation_registry::register_with_state(user,
            max_gas_amount,
            estimated_fee,
            86400,
            PENDING
        );
        assert!(1 == automation_registry::get_next_task_index(), 1);
        assert!(max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 2);

        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + estimated_fee;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        let max_gas_amount_causing_of = (AUTOMATION_MAX_GAS_TEST * (CONGESTION_THRESHOLD_TEST as u64)) / 100;
        let estimated_fee = automation_registry::estimate_automation_fee(max_gas_amount_causing_of);
        automation_registry::register_with_state(user,
            max_gas_amount_causing_of,
            estimated_fee,
            86400,
            PENDING
        );
        assert!(2 == automation_registry::get_next_task_index(), 3);
        assert!(max_gas_amount_causing_of + max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 4);

        let registration_charges = FLAT_REGISTRATION_FEE_TEST + estimated_fee;
        let expected_current_balance = expected_current_balance - registration_charges;
        let expected_registry_balance = expected_registry_balance + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);
    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_priority_value(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let max_gas_amount = 10;
        let estimated_fee = automation_registry::estimate_automation_fee(max_gas_amount);
        automation_registry::register_with_custom_input(user,
            max_gas_amount,
            estimated_fee,
            20,
            86400,
            PARENT_HASH,
            AUX_DATA,
            PENDING,
        );

        let task_details = automation_registry::get_task_details(0);
        check_task_priority(&task_details, 0);

        automation_registry::register_with_custom_input(user,
            max_gas_amount,
            estimated_fee,
            20,
            86400,
            PARENT_HASH,
            AUX_DATA,
            PENDING,
        );

        let task_details = automation_registry::get_task_details(1);
        check_task_priority(&task_details, 1);

        let input_priority = 42u64;
        let aux_data = vector[vector[UST], bcs::to_bytes(&input_priority)];
        automation_registry::register_with_custom_input(user,
            max_gas_amount,
            estimated_fee,
            20,
            86400,
            PARENT_HASH,
            aux_data,
            PENDING,
        );
        let task_details = automation_registry::get_task_details(2);
        check_task_priority(&task_details, input_priority);


        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(
            &multisig_signer,
            70,
            86400,
            PENDING
        );
        let task_details = automation_registry::get_task_details(3);
        check_task_priority(&task_details, 3);
        assert!(automation_registry::is_of_type(&task_details, GST), 5);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EREGISTRY_IS_FULL, location = automation_registry)]
    fun check_registration_with_full_tasks(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::update_config_for_tests(
            framework,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            2,
        );
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        // Registry is already full
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_EXPIRY_TIME, location = automation_registry)]
    fun check_registration_invalid_expiry_time(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        timestamp::update_global_time_for_test_secs(50);

        automation_registry::register_with_state(user,
            25,
            1000,
            25,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EEXPIRY_BEFORE_NEXT_CYCLE, location = automation_registry)]
    fun check_registration_invalid_expiry_time_before_next_epoch(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            70,
            1000,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            PENDING,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EEXPIRY_TIME_UPPER, location = automation_registry)]
    fun check_registration_invalid_expiry_time_surpassing_task_duration_cap(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            70,
            1000,
            TTL_UPPER_BOUND_TEST + 1,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_registration_valid_expiry_time_matches_task_duration_cap(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            70,
            1000,
            TTL_UPPER_BOUND_TEST,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_GAS_PRICE, location = automation_registry)]
    fun check_registration_invalid_gas_price_cap(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_custom_input(user,
            70,
            1000,
            0,
            86400,
            PARENT_HASH,
            AUX_DATA,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_MAX_GAS_AMOUNT, location = automation_registry)]
    fun check_registration_invalid_max_gas_amount(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_custom_input(user,
            0,
            1000,
            20,
            86400,
            PARENT_HASH,
            AUX_DATA,
            PENDING
        );
    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TXN_HASH, location = automation_registry)]
    fun check_registration_invalid_parent_hash(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_custom_input(user,
            10,
            1000,
            20,
            86400,
            vector<u8>[0, 1, 2, 3],
            AUX_DATA,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_AUX_DATA_LENGTH, location = automation_registry)]
    fun check_registration_with_aux_data(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let aux_data = vector[];
        automation_registry::register_with_custom_input(user,
            10,
            1000,
            20,
            86400,
            PARENT_HASH,
            aux_data,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TASK_TYPE_LENGTH, location = automation_registry)]
    fun check_registration_with_invalid_task_type(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let aux_data = vector[vector[1, 2], vector[]];
        automation_registry::register_with_custom_input(user,
            10,
            1000,
            20,
            86400,
            PARENT_HASH,
            aux_data,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TASK_TYPE, location = automation_registry)]
    fun check_registration_with_invalid_task_type_data(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let aux_data = vector[vector[0], vector[]];
        automation_registry::register_with_custom_input(user,
            10,
            1000,
            20,
            86400,
            PARENT_HASH,
            aux_data,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EGAS_AMOUNT_UPPER, location = automation_registry)]
    fun check_registration_with_overflow_gas_limit(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            60000000,
            100_000_000,
            86400,
            PENDING,
        );
        assert!(1 == automation_registry::get_next_task_index(), 1);
        assert!(60000000 == automation_registry::get_gas_committed_for_next_epoch(), 1);
        automation_registry::register_with_state(user,
            60000000,
            100_000_000,
            86400,
            PENDING,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH, location = automation_registry)]
    fun check_registration_with_insufficient_automation_fee_cap(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            10000,
            1,
            86400,
            PENDING,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ECYCLE_TRANSITION_IN_PROGRESS, location = automation_registry)]
    fun check_registration_in_cycle_transition_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            10_000,
            100_000,
            86400,
            PENDING,
        );
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);

        // Expected cycle to be in finished state so the following registration should fail
        automation_registry::register_with_state(user,
            10_000,
            100_000,
            86400,
            PENDING,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_activation_on_new_cycle(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );

        // No active task and committed gas for the next epoch is total of the all registered tasks
        assert!(40 == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        assert!(active_task_ids == vector[], 1);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::process_tasks_for_tests( 2, vector[0, 1, 2, 3]);
        assert!(40 == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        // But here task 3 is in the active list as it is still active in this new epoch.
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_successful_cancellation(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );
        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );
        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );
        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );

        assert!(40 == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });

        // Cancel task 2. The committed gas for the next epoch will be updated,
        // but when requested active task it will be still available in the list
        automation_registry::cancel_task(user, 2);
        // Task will be still available in the registry but with cancelled state
        automation_registry::check_task_state(2, true, CANCELLED);

        assert!(30 == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });

        // Add and cancel the task in the same epoch. Task index will be 4
        assert!(automation_registry::get_next_task_index() == 4, 1);
        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            PENDING
        );
        automation_registry::cancel_task(user, 4);
        assert!(30 == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });
        // there is no task with index 4 and the next task index will be 5.
        assert!(!automation_registry::has_task_with_id(4), 1);
        assert!(automation_registry::get_next_task_index() == 5, 1)
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_pending_task_cancellation_refunds(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let automation_fee_cap = 1000;

        automation_registry::register_with_state(user,
            10,
            automation_fee_cap,
            86400,
            PENDING,
        );
        // check user balance after registered new task
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + automation_fee_cap;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        automation_registry::cancel_task(user, 0);
        // Pending task upon cancellation refunded only with half of the deposit;
        let expected_refund = automation_fee_cap / REFUND_FACTOR;
        check_account_balance(user_address, expected_current_balance + expected_refund);
        check_account_balance(registry_fee_address, expected_registry_balance - expected_refund);
    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EAUTOMATION_TASK_NOT_FOUND, location = automation_registry)]
    fun check_cancellation_of_non_existing_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::cancel_task(user, 1);
    }

    #[test(framework = @supra_framework, user = @0x1cafe, user2 = @0x1cafa)]
    #[expected_failure(abort_code = EUNAUTHORIZED_TASK_OWNER, location = automation_registry)]
    fun check_unauthorized_cancellation_task(
        framework: &signer,
        user: &signer,
        user2: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        automation_registry::cancel_task(user2, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EALREADY_CANCELLED, location = automation_registry)]
    fun check_cancellation_of_cancelled_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );
        // Cancel the same task 2 times
        automation_registry::cancel_task(user, 0);
        automation_registry::cancel_task(user, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ECYCLE_TRANSITION_IN_PROGRESS, location = automation_registry)]
    fun check_cancellation_in_transition_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        // Cancel the in the FINISHED state
        automation_registry::cancel_task(user, 0);
    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_normal_fee_charge_on_cycle_change(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let automation_fee_cap = 100_000;
        let max_gas_amount = 1_000_000;

        automation_registry::register_with_state(user,
            max_gas_amount, // normal gas amount
            100_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            PENDING,
        );

        // check user balance after registered new task
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + automation_fee_cap;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::process_tasks_for_tests( 2, vector[0]);

        // 10 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee = 10 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        // check user balance after on new epoch fee applied
        check_account_balance(user_address, expected_current_balance - expected_automation_fee);
        check_account_balance(
            registry_fee_address,
            expected_registry_balance + expected_automation_fee);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_congestion_fee_charge_on_charge(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let automation_fee_cap = 10_000_000;
        let max_gas_amount = 85_000_000;

        automation_registry::register_with_state(user,
            max_gas_amount, // congestion threshold reached
            automation_fee_cap,
            3 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            PENDING,
        );

        // check user balance after registered new task
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + automation_fee_cap;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::process_tasks_for_tests(2, vector[0]);

        automation_registry::has_task_with_id(0);

        // 85/100 * 1000 = 850 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 850;
        // 5% surpasses the threshold, ((1+(5/100))^exponent-1) * 100 = 34 congestion base fee, occupancy 85/100, 7200 epoch duration
        let expected_congestion_fee = 34 * 85 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let expected_epoch_fee = expected_automation_fee + expected_congestion_fee;
        // check user balance after on new epoch fee applied
        check_account_balance(user_address, expected_current_balance - expected_epoch_fee);
        check_account_balance(
            registry_fee_address,
            expected_registry_balance + expected_epoch_fee);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_drop_execution(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;
        let exists = true;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time / 2,
            ACTIVE);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task3 = automation_registry::register_with_state(
            user,
            11_000_000,
            automation_fee_cap,
            task_exipry_time,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);


        let user_address = address_of(user);

        // Update time so task1 is expired.
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
        automation_registry::monitor_cycle_end();
        // Make sure we are in FINISHED state
        check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        // Make sure that we attempt to drop only cancelled and expired tasks, to avoid any asserts in this scenario
        automation_registry::process_tasks_for_tests(2, vector[task1, task2]);

        // Check that we are still in finished state and processed-task are only task2 and task3
        check_next_task_index_to_be_processed(CYCLE_FINISHED, 2);

        // Check that both tasks have been refunded with depoit fee only
        let expected_total_deposit_refund = 2 * automation_fee_cap;
        check_account_balance(user_address, expected_user_current_balance + expected_total_deposit_refund);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance - expected_total_deposit_refund);
        // Check that only task3 still exists in pending state
        automation_registry::check_task_state(task3, exists, PENDING);
        assert!(!automation_registry::has_task_with_id(task1), 3);
        assert!(!automation_registry::has_task_with_id(task2), 4);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_drop_even_if_refund_fails(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let expected_user_current_balance = ACCOUNT_BALANCE - (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);

        let user_address = address_of(user);

        // Modify refund-bookkeeping locked deposit amount to cause refund failure
        automation_registry::set_total_deposited_automation_fee(0);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        // Make sure we are in FINISHED state
        automation_registry::monitor_cycle_end();
        // Make sure that we attempt to drop only cancelled and expired tasks, to avoid any asserts in this scenario
        automation_registry::process_tasks_for_tests( 2, vector[task1]);


        // As long as there was a single task in the registry registry will move to started state
        check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

        // Check that no refund has happened
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);

        // Check that task has been removed from registry
        assert!(!automation_registry::has_task_with_id(task1), 2);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_process_tasks_even_input_is_empty_or_non_existent(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let expected_user_current_balance = ACCOUNT_BALANCE - (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);

        let user_address = address_of(user);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        // Make sure we are in FINISHED state
        automation_registry::monitor_cycle_end();
        // Make sure that we attempt to drop only cancelled and expired tasks, to avoid any asserts in this scenario
        automation_registry::process_tasks_for_tests(2, vector[]);
        automation_registry::process_tasks_for_tests(2, vector[5]);


        // As long as there was a single task in the registry registry will move to started state
        check_next_task_index_to_be_processed(CYCLE_FINISHED, 0);

        // Check that no refund has happened
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);

        // Check that task has been removed from registry
        assert!(automation_registry::has_task_with_id(task1), 2);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EOUT_OF_ORDER_TASK_PROCESSING_REQUEST, location = automation_registry)]
    fun check_tasks_processing_out_of_order_fails_in_finished_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let _task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        // Make sure we are in FINISHED state
        automation_registry::monitor_cycle_end();
        // Start processing from task 2
        automation_registry::process_tasks_for_tests( 2, vector[task2]);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EOUT_OF_ORDER_TASK_PROCESSING_REQUEST, location = automation_registry)]
    fun check_tasks_processing_out_of_order_fails_in_suspended_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let _task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);

        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();
        automation_registry::process_tasks_for_tests( 1, vector[task2]);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EINVALID_REGISTRY_STATE, location = automation_registry)]
    fun check_process_tasks_fails_on_started_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            ACTIVE);
        // Attempt to drop in STARTED state
        automation_registry::process_tasks_for_tests(2, vector[task1]);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EINVALID_REGISTRY_STATE, location = automation_registry)]
    fun check_process_tasks_fails_on_ready_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        // feature is disabled in started state, when registry is empty, moves registry in ready state
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();
        automation_registry::process_tasks_for_tests(2, vector[0]);
    }


    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_charge_execution(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;
        let exists = true;
        let t1_t2_max_gas_amount = 44_000_000;
        let t3_max_gas_amount = 11_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            PENDING);
        let task2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            ACTIVE);
        let task3 = automation_registry::register_with_state(
            user,
            11_000_000,
            automation_fee_cap,
            task_exipry_time / 2,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let total_committed_gas = 2 * t1_t2_max_gas_amount + t3_max_gas_amount;


        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task_1_2 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 19% surpasses the threshold, ((1+(19/100))^exponent-1) * 100 = 183 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task_1_2 = 183 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;

        // 11/100 * 1000 = 110 - automation_epoch_fee_per_second, 7200 epoch duration (PENDING tasks are charge for the first cycle fully)
        let expected_automation_fee_per_task_3 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 110;
        // 19% surpasses the threshold, ((1+(19/100))^exponent-1) * 100 = 183 congestion base fee, occupancy 11/100, 7200 epoch duration
        let expected_congestion_fee_per_task_3 = 183 * 11 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;

        let user_address = address_of(user);

        // Update time so task3 is expired.
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        // Make sure we are in FINISHED state
        automation_registry::monitor_cycle_end();
        // Check that we are still in finished state and processed-task are only task1 and task2
        check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        automation_registry::process_tasks_for_tests(2, vector[task1, task2]);

        // Check that we are still in finished state and processed-task are only task2 and task3
        check_next_task_index_to_be_processed(CYCLE_FINISHED, 2);

        // Check that both tasks have been refunded with depoit fee only
        let expected_total_charge = 2 * (expected_automation_fee_per_task_1_2 + expected_congestion_fee_per_task_1_2);
        expected_user_current_balance = expected_user_current_balance - expected_total_charge;
        expected_registry_current_balance = expected_registry_current_balance + expected_total_charge;
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);
        // Check that task1 still exists in pending state
        automation_registry::check_task_state(task3, exists, PENDING);
        automation_registry::check_task_state(task2, exists, ACTIVE);
        automation_registry::check_task_state(task1, exists, ACTIVE);

        automation_registry::process_tasks_for_tests(2, vector[task3]);

        // Check that we are still in finished state and processed-task are only task2 and task3
        check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

        // Check that both tasks have been refunded with depoit fee only
        let expected_total_charge = expected_automation_fee_per_task_3 + expected_congestion_fee_per_task_3;
        expected_user_current_balance = expected_user_current_balance - expected_total_charge;
        expected_registry_current_balance = expected_registry_current_balance + expected_total_charge;
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);

        automation_registry::check_task_state(task3, exists, ACTIVE);
        automation_registry::check_task_state(task2, exists, ACTIVE);
        automation_registry::check_task_state(task1, exists, ACTIVE);
        expected_total_charge = expected_total_charge + 2 * (expected_automation_fee_per_task_1_2 + expected_congestion_fee_per_task_1_2);
        let expected_gas_for_next_cycle = total_committed_gas - t3_max_gas_amount;
        let expected_total_deposit_fee = 3 * automation_fee_cap;
        automation_registry::check_gas_and_fees_for_cycle(
            total_committed_gas,
            expected_gas_for_next_cycle,
            expected_total_charge,
            expected_total_deposit_fee)
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_charge_for_tasks_to_be_dropped_due_to_limitations(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let t1_t2_max_gas_amount = 44_000_000;
        let t3_max_gas_amount = 11_000_000;
        let automation_fee_cap = 100_000_000;

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task_1_2 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration

        // 11/100 * 1000 = 110 - automation_epoch_fee_per_second, 7200 epoch duration (PENDING tasks are charge for the first cycle fully)
        let expected_automation_fee_per_task_3 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 110;
        // 19% surpasses the threshold, ((1+(19/100))^exponent-1) * 100 = 183 congestion base fee, occupancy 11/100, 7200 epoch duration
        let expected_congestion_fee_per_task_3 = 183 * 11 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;

        let task1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            expected_automation_fee_per_task_1_2,
            task_exipry_time,
            ACTIVE);
        let task2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            PENDING);
        let task3 = automation_registry::register_with_state(
            user,
            11_000_000,
            expected_automation_fee_per_task_3 + expected_congestion_fee_per_task_3,
            task_exipry_time / 2,
            PENDING);

        let expected_deposit_charge =
            expected_automation_fee_per_task_1_2  // task1
                + automation_fee_cap // task2
                + expected_automation_fee_per_task_3 + expected_congestion_fee_per_task_3; // task3

        let expected_user_current_balance = ACCOUNT_BALANCE - (3 * FLAT_REGISTRATION_FEE_TEST + expected_deposit_charge);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * FLAT_REGISTRATION_FEE_TEST + expected_deposit_charge;

        let user_address = address_of(user);

        // Make sure that user account has only enough balance for task 1 automation fee
        coin::transfer<SupraCoin>(
            user,
            automation_registry::get_registry_fee_address(),
            expected_user_current_balance);
        expected_registry_current_balance = expected_registry_current_balance + expected_user_current_balance;
        expected_user_current_balance = 0;

        let expected_refund_after_charge =
            expected_automation_fee_per_task_1_2 // task1
                + expected_automation_fee_per_task_3 + expected_congestion_fee_per_task_3; // task3

        // Update time so task3 is considered as expired.
        timestamp::update_global_time_for_test_secs(3 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
        // Make sure we are in FINISHED state
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        automation_registry::process_tasks_for_tests(2,  vector[task1, task2, task3]);

        {
            // Check there are no tasks
            assert!(!automation_registry::has_task_with_id(task1), 1);
            assert!(!automation_registry::has_task_with_id(task2), 2);
            assert!(!automation_registry::has_task_with_id(task3), 3);

            // Check that we are in STARTED state as all expected tasks are handled.
            automation_registry::check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

            // Check that both tasks have been refunded with depoit fee only
            expected_user_current_balance = expected_user_current_balance + expected_refund_after_charge;
            expected_registry_current_balance = expected_registry_current_balance - expected_refund_after_charge;
            check_account_balance(user_address, expected_user_current_balance);
            check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);

            let total_committed_gas = 2 * t1_t2_max_gas_amount + t3_max_gas_amount;
            automation_registry::check_gas_and_fees_for_cycle(total_committed_gas, 0, 0, 0)

        };
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_refund_and_cleanup_execution(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;
        let exists = true;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            ACTIVE);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task3 = automation_registry::register_with_state(
            user,
            11_000_000,
            automation_fee_cap,
            task_exipry_time / 2,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task_1_2 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task_1_2 = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;

        let user_address = address_of(user);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 3);
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();

        let expected_remaining_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS - timestamp::now_seconds();
        let expected_automation_fee_multiplier = automation_registry::calculate_automation_fee_multiplier_for_current_cycle();

        // Check that we are still in finished state and processed-task are only task2 and task3
        automation_registry::check_suspended_cycle_state(expected_remaining_time, expected_automation_fee_multiplier);


        // set enough cycle fee to be able to refund
        automation_registry::set_locked_fee(10_000_000_000);
        automation_registry::process_tasks_for_tests(1, vector[task1, task2]);

        // Check that we are still in finished state and processed-task are only task2 and task3
        automation_registry::check_next_task_index_to_be_processed(CYCLE_SUSPENDED, 2);

        // Check that both tasks have been refunded with depoit fee and remaining cycle fee only
        let expected_total_deposit_refund = 2 * automation_fee_cap;
        let expected_fee_refund = 2 * (expected_automation_fee_per_task_1_2 + expected_congestion_fee_per_task_1_2) * 2 / 3;
        let total_expected_refund = expected_total_deposit_refund + expected_fee_refund;
        expected_user_current_balance = expected_user_current_balance + total_expected_refund;
        expected_registry_current_balance = expected_registry_current_balance - total_expected_refund;
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);
        // Check that only task1 still exists in pending state
        automation_registry::check_task_state(task3, exists, PENDING);
        assert!(!automation_registry::has_task_with_id(task1), 3);
        assert!(!automation_registry::has_task_with_id(task2), 4);

        // Empty input does not cause issues
        automation_registry::process_tasks_for_tests( 1, vector[]);

        // Non existing item does not cause issues
        automation_registry::process_tasks_for_tests( 1, vector[10, 12]);

        automation_registry::process_tasks_for_tests( 1, vector[task3]);

        // Check that we are still in finished state and processed-task are only task2 and task3
        check_cycle_state_and_duration(CYCLE_READY, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

        // Check that only depoit fee is refunded from 3rd PENDING task
        expected_user_current_balance = expected_user_current_balance + automation_fee_cap;
        expected_registry_current_balance = expected_registry_current_balance - automation_fee_cap;
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);
        automation_registry::check_gas_and_fees_for_cycle(0, 0, 0, 0);

        assert!(!automation_registry::has_task_with_id(task1), 10);
        assert!(!automation_registry::has_task_with_id(task2), 11);
        assert!(!automation_registry::has_task_with_id(task3), 12);

    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_refund_and_cleanup_even_if_refund_fails(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 2 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 2 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);

        let user_address = address_of(user);

        // Modify refund-bookkeeping locked deposit amount to cause refund failure
        automation_registry::set_total_deposited_automation_fee(0);


        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 3);
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();

        let expected_remaining_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS - timestamp::now_seconds();
        let expected_automation_fee_multiplier = automation_registry::calculate_automation_fee_multiplier_for_current_cycle();

        automation_registry::check_suspended_cycle_state(expected_remaining_time, expected_automation_fee_multiplier);

        automation_registry::process_tasks_for_tests(1, vector[task1, task2]);

        // As long as there was a single task in the registry registry will move to started state
        check_cycle_state_and_duration(CYCLE_READY, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

        // Check that no refund has happened
        check_account_balance(user_address, expected_user_current_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);
        automation_registry::check_gas_and_fees_for_cycle(0, 0, 0, 0);

        // Check that tasks have been removed from registry
        assert!(!automation_registry::has_task_with_id(task1), 7);
        assert!(!automation_registry::has_task_with_id(task2), 8);

    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_refund_and_cleanup_transitioned_from_finished_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            ACTIVE);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 2 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 2 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);

        let user_address = address_of(user);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        // Moves to Finished state
        automation_registry::monitor_cycle_end();
        // Right after it feature flag is disabled and chain state is reconfgured
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();

        automation_registry::check_suspended_cycle_state(0, 0);

        automation_registry::process_tasks_for_tests(  1, vector[task1, task2]);

        // As long as there was a single task in the registry registry will move to started state
        automation_registry::check_cycle_state_and_duration(CYCLE_READY, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

        // Check that no refund has happened
        let expected_only_deposit_refund = 2 * automation_fee_cap;
        check_account_balance(user_address, expected_user_current_balance + expected_only_deposit_refund);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance - expected_only_deposit_refund);

        automation_registry::check_gas_and_fees_for_cycle(0, 0, 0, 0);

        // Check that tasks have been removed from registry
        assert!(!automation_registry::has_task_with_id(task1), 5);
        assert!(!automation_registry::has_task_with_id(task2), 6);

    }

    #[test(framework = @supra_framework, user = @0x1cafb)]
    fun check_config_updated_from_start_to_suspended_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 11_000_000;
        let automation_fee_cap = 100_000_000;

        let _t1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let _t2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, CANCELLED);
        let _t3 = automation_registry::register_with_state(
            user,
            t3_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, PENDING);
        automation_registry::update_config_v2(framework,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST * 2,
            AUTOMATION_BASE_FEE_TEST / 2,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST / 2,
            CONGESTION_BASE_FEE_TEST / 2,
            CONGESTION_EXPONENT_TEST - 1,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
        // Disable feature and call on new epoch to check that when transition to suspened state from started no config is updated
        toggle_feature_flag(framework, false);
        let expected_cycle_duration = automation_registry::check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_SUSPENDED, expected_cycle_duration, HAS_TRANSITION_STATE);

        let expected_fee_per_sec = calculate_automation_fee_multiplier_for_committed_occupancy(
            2 * t1_t2_max_gas);
        automation_registry::check_suspended_cycle_state(EPOCH_INTERVAL_FOR_TEST_IN_SECS, expected_fee_per_sec);
        automation_registry::check_cycle_new_duration(expected_cycle_duration);

        automation_registry::check_automation_configuration(
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );

    }

    #[test(framework = @supra_framework, user = @0x1cafb)]
    fun check_config_updated_from_finished_suspended_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 11_000_000;
        let automation_fee_cap = 100_000_000;

        let _ = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let _ = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, CANCELLED);
        let _ = automation_registry::register_with_state(
            user,
            t3_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, PENDING);
        // Set some locked fee which is enough to pay refund if necessary
        automation_registry::set_locked_fee(100_000_000);
        automation_registry::update_config_v2(framework,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST / 2,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST / 2,
            CONGESTION_BASE_FEE_TEST / 2,
            CONGESTION_EXPONENT_TEST - 1,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        // Disable feature and call on new epoch to check that when transition to suspened state from started no config is updated
        toggle_feature_flag(framework, false);
        let expected_cycle_duration = automation_registry::check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS,HAS_TRANSITION_STATE);
        automation_registry::on_new_epoch();
        {
            automation_registry::check_cycle_state_and_duration(CYCLE_SUSPENDED, expected_cycle_duration, HAS_TRANSITION_STATE);
            automation_registry::check_suspended_cycle_state(0, 0);
            automation_registry::check_cycle_new_duration(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

            automation_registry::check_automation_configuration(
                TTL_UPPER_BOUND_TEST,
                AUTOMATION_MAX_GAS_TEST,
                AUTOMATION_BASE_FEE_TEST / 2,
                FLAT_REGISTRATION_FEE_TEST,
                CONGESTION_THRESHOLD_TEST / 2,
                CONGESTION_BASE_FEE_TEST / 2,
                CONGESTION_EXPONENT_TEST - 1,
                TASK_CAPACITY_TEST,
                EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
                SYS_TASK_DURATION_CAP_IN_SECS,
                SYS_AUTOMATION_MAX_GAS_TEST,
                SYS_TASK_CAPACITY_TEST
            );
        };
    }

    #[test(framework = @supra_framework, user = @0x1cafb)]
    fun check_config_updated_from_transition_suspended_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 11_000_000;
        let automation_fee_cap = 100_000_000;

        let t1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let _t2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, CANCELLED);
        let _t3 = automation_registry::register_with_state(
            user,
            t3_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, PENDING);

        automation_registry::update_config_v2(framework,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST / 2,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST / 2,
            CONGESTION_BASE_FEE_TEST / 2,
            CONGESTION_EXPONENT_TEST - 1,
            TASK_CAPACITY_TEST,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );


        // Move to FINISHED state
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();

        // Expected new cylce gas and automation_fee_per_sec. Calculate now to make sure that config update was done.
        let total_committed_gas_for_new_cycle = t1_t2_max_gas + t3_max_gas;
        let automation_fee_per_sec_for_new_cycle = calculate_automation_fee_multiplier_for_committed_occupancy(total_committed_gas_for_new_cycle);
        print(&automation_fee_per_sec_for_new_cycle);
        // Process one task to mark transition initiated
        automation_registry::process_tasks_for_tests(2, vector[t1]);
        // Disable feature and call on new epoch to check that when transition to suspened state from started no config is updated
        automation_registry::check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        toggle_feature_flag(framework, false);
        let new_cycle_duration = EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        // 44/100  task occupancy
        let expected_automation_fee_per_task_1_2 = new_cycle_duration * 44 * automation_fee_per_sec_for_new_cycle / 100;
        automation_registry::on_new_epoch();
        {
            automation_registry::check_cycle_transition_state(CYCLE_FINISHED,
            0,
                new_cycle_duration,
            automation_fee_per_sec_for_new_cycle,
            total_committed_gas_for_new_cycle,
            t1_t2_max_gas,
            0,
                expected_automation_fee_per_task_1_2,
            3,
                1
            );

            automation_registry::check_automation_configuration(
                TTL_UPPER_BOUND_TEST,
                AUTOMATION_MAX_GAS_TEST,
                AUTOMATION_BASE_FEE_TEST / 2,
                FLAT_REGISTRATION_FEE_TEST,
                CONGESTION_THRESHOLD_TEST / 2,
                CONGESTION_BASE_FEE_TEST / 2,
                CONGESTION_EXPONENT_TEST - 1,
                TASK_CAPACITY_TEST,
                EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
                SYS_TASK_DURATION_CAP_IN_SECS,
                SYS_AUTOMATION_MAX_GAS_TEST,
                SYS_TASK_CAPACITY_TEST
            );
        };
    }

    #[test(framework = @supra_framework, user = @0x1cafb)]
    fun check_feature_enable_in_suspended_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_max_gas = 44_000_000;
        let automation_fee_cap = 100_000_000;

        let _t1 = automation_registry::register_with_state(
            user,
            t1_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        // Disable feature and call on new epoch to check that when transition to suspened state from started no config is updated
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_SUSPENDED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        toggle_feature_flag(framework, true);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_SUSPENDED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
    }

    #[test(framework = @supra_framework, user = @0x1cafb)]
    fun check_feature_enable_in_ready_state(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_max_gas = 44_000_000;
        let automation_fee_cap = 100_000_000;

        let _t1 = automation_registry::register_with_state(
            user,
            t1_max_gas,
            automation_fee_cap,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            ACTIVE);
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_SUSPENDED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);

        automation_registry::process_tasks_for_tests( 1, vector[0]);
        automation_registry::check_cycle_state_and_duration(CYCLE_READY, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        toggle_feature_flag(framework, true);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_calculation(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;

        let _ = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let _ = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        // Epoch cut short 2 times

        // if epoch length matches or greater the expected epoch interval then no refund is expected
        // event if there is a locked fee.
        let tcmg = ((2 * t1_t2_max_gas) as u256);
        // Take into account CANCELLED tasks as well
        // Tasks are still valid
        let results = automation_registry::calculate_tasks_automation_fees(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            0,
            tcmg);
        assert!(vector::length(&results) == 2, 2);

        let expected_fee = expected_automation_fee_per_task + expected_congestion_fee_per_task;
        automation_registry::check_automation_fee(&results, 0, expected_fee);
        automation_registry::check_automation_fee(&results, 1, expected_fee);

        // Take into account CANCELLED tasks as well
        // Tasks are still valid but for the half of the epoch, current_time - task.expiry time == epoch_duration / 2
        let current_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let results = automation_registry::calculate_tasks_automation_fees(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            current_time,
            tcmg,
        );
        // Pending task is ignored
        assert!(vector::length(&results) == 2, 2);

        let expected_fee = (expected_automation_fee_per_task + expected_congestion_fee_per_task) / 2;
        automation_registry::check_automation_fee(&results, 0, expected_fee);
        automation_registry::check_automation_fee(&results, 1, expected_fee);

        // Tasks are considered as expired even if they are part of the registry due to some bug, they will not be charged.
        let current_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        let results = automation_registry::calculate_tasks_automation_fees(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            current_time,
            tcmg,
        );
        // Pending task is ignored
        assert!(vector::length(&results) == 2, 2);

        automation_registry::check_automation_fee(&results, 0, 0);
        automation_registry::check_automation_fee(&results, 1, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_calculation_for_short_tasks(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let expiry_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;

        // Old but short task, will be charged according to active time
        let task1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            expiry_time,
            ACTIVE);

        // New short task will be charged full epoch fee
        let task2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            expiry_time,
            PENDING);

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        // Epoch cut short 2 times

        // if epoch length matches or greater the expected epoch interval then no refund is expected
        // event if there is a locked fee.
        let tcmg = ((2 * t1_t2_max_gas) as u256);
        // Take into account CANCELLED tasks as well
        // Tasks are still valid
        let results = automation_registry::calculate_tasks_automation_fees(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            tcmg);
        assert!(vector::length(&results) == 2, 2);

        let expected_fee = expected_automation_fee_per_task + expected_congestion_fee_per_task;
        automation_registry::check_automation_fee(&results, task1, expected_fee / 2);
        automation_registry::check_automation_fee(&results, task2, expected_fee);

        // Now lets assume task as activated and epoch has been kept short, refund for both tasks will be done in the
        // same manner according to their expiration time.
        automation_registry::update_task_state(task2, ACTIVE);

        let current_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 4;
        let refund_interval = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS - current_time;
        // Each task will be active still for EPOCH_INTERVAL_FOR_TEST_IN_SECS / 4 duration,
        // as expiry time was EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let results = automation_registry::calculate_tasks_automation_fees(
            refund_interval,
            current_time,
            tcmg);
        assert!(vector::length(&results) == 2, 5);

        let expected_fee = expected_automation_fee_per_task + expected_congestion_fee_per_task;
        automation_registry::check_automation_fee(&results, task1, expected_fee / 4);
        automation_registry::check_automation_fee(&results, task2, expected_fee / 4);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_calculation_with_zero_multipliers(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;

        let _ = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let _ = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        // Epoch cut short 2 times

        // Update config with 0 automation base fee
        automation_registry::update_config_for_tests(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            0,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );

        let tcmg = ((2 * t1_t2_max_gas) as u256);

        {
            let results = automation_registry::calculate_tasks_automation_fees(
                EPOCH_INTERVAL_FOR_TEST_IN_SECS,
                0,
                tcmg);
            assert!(vector::length(&results) == 2, 10);

            let expected_fee = expected_congestion_fee_per_task;
            automation_registry::check_automation_fee(&results, 0, expected_fee);
            automation_registry::check_automation_fee(&results, 1, expected_fee);
        };

        // Update config with 100% congestion treshold, no congestion fee is expected
        automation_registry::update_config_for_tests(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            100,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );

        let tcmg = ((2 * t1_t2_max_gas) as u256);

        {
            let results = automation_registry::calculate_tasks_automation_fees(
                EPOCH_INTERVAL_FOR_TEST_IN_SECS,
                0,
                tcmg);
            assert!(vector::length(&results) == 2, 11);

            let expected_fee = expected_automation_fee_per_task;
            automation_registry::check_automation_fee(&results, 0, expected_fee);
            automation_registry::check_automation_fee(&results, 1, expected_fee);
        };

        // Update config with 0 congestion base fee, no congestion fee is expected
        automation_registry::update_config_for_tests(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            0,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );

        let tcmg = ((2 * t1_t2_max_gas) as u256);

        {
            let results = automation_registry::calculate_tasks_automation_fees(
                EPOCH_INTERVAL_FOR_TEST_IN_SECS,
                0,
                tcmg);
            assert!(vector::length(&results) == 2, 12);

            let expected_fee = expected_automation_fee_per_task;
            automation_registry::check_automation_fee(&results, 0, expected_fee);
            automation_registry::check_automation_fee(&results, 1, expected_fee);
        };
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_withdrawal_on_charge(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 10_000_000;
        let automation_fee_cap_t1 = 10_000_000;
        let automation_fee_cap_t2_t3 = 100_000_000;

        // Automation fee cap overflow
        let t1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            10_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let t2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas,
            100_000_000,
            3 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        // Not enough balance to pay fee
        let t3 = automation_registry::register_with_state(
            user,
            t3_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);

        // Update config to cause automation fee cap overflow for the 1st task
        automation_registry::update_config_for_tests(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST * 100,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );

        // TASK 1 and 2 expected epoch fee calculation
        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_for_t1_2 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 18% surpasses the threshold, ((1+(18/100))^exponent-1) * 10000 = 1.6995541 * 10000 congestion base fee,
        // occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_for_t1_2 = 16995 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let expected_epoch_fee_for_t1_2 = expected_automation_fee_for_t1_2 + expected_congestion_fee_for_t1_2;

        // 3 tasks have been registered
        let registration_charges = 3 * FLAT_REGISTRATION_FEE_TEST + automation_fee_cap_t1 + 2 * automation_fee_cap_t2_t3;
        let expected_user_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        // Make sure that user account has only enough balance for task 2 automation fee
        let withdraw_amount = expected_user_current_balance - expected_epoch_fee_for_t1_2;
        coin::transfer<SupraCoin>(
            user,
            automation_registry::get_registry_fee_address(),
            withdraw_amount);
        let expected_registry_current_balance = expected_registry_current_balance + withdraw_amount;

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        let committed_gas_for_new_cycle = 2 * t1_t2_max_gas + t3_max_gas;

        // Not only charges will be applied but also state will be updated to STARTED as all expected tasks will be processed.
        automation_registry::process_tasks_for_tests( 2, vector[t1, t2, t3]);

        let user_address = address_of(user);

        // TASK 1 cancelled due-to automation fee cap surrpass and task 3 is cancelled due to insufficient balance.
        // So for task 1 full deposit refund is expected and for task 3 no refund is expected.
        assert!(!automation_registry::has_task_with_id(t1), 1);
        assert!(!automation_registry::has_task_with_id(t3), 2);
        assert!(automation_registry::has_sender_active_task_with_id(user_address, t2), 3);
        // only one task is charged as the other 2 are cancelled/removed.
        // and uppon cancellation no deposit is refunded.
        check_account_balance(user_address, automation_fee_cap_t1);
        check_account_balance(
            automation_registry::get_registry_fee_address(),
            expected_registry_current_balance + expected_epoch_fee_for_t1_2 - automation_fee_cap_t1
        );

        automation_registry::check_gas_and_fees_for_cycle(committed_gas_for_new_cycle, t1_t2_max_gas, expected_epoch_fee_for_t1_2, automation_fee_cap_t2_t3);
        automation_registry::check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_estimate_api(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_max_gas = 10_000_000;
        // 10/100  * 1000 = 100 - automation_epoch_fee_per_sec, 7200 epoch duration. no congestion fee as threshold is not crossed.
        let expected_automation_fee = 100 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        let result = automation_registry::estimate_automation_fee(task_max_gas);
        assert!(result == expected_automation_fee, 1);

        // expected congestion fee with 85 % congestion
        // 5% surpass, ((1+(5/100))^exponent-1) * 100 = 34 (acf), task occupancy 10% epoch interval 7200
        let expected_congestion_fee = 34 * 10 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let result = automation_registry::estimate_automation_fee_with_committed_occupancy(task_max_gas, 75_000_000);
        assert!(result == expected_automation_fee + expected_congestion_fee, 2);

        // update next epoch committed max gas to cause the same congestion
        automation_registry::set_gas_committed_for_next_cycle(75_000_000);

        let result = automation_registry::estimate_automation_fee(task_max_gas);
        assert!(result == expected_automation_fee + expected_congestion_fee, 2);

        // update next epoch registry max gas cap to resolve the congestion
        automation_registry::update_config_for_tests(
            framework,
            TTL_UPPER_BOUND_TEST,
            200_000_000,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,

        );

        // 10/200 * 1000 occupancy - 50 - automation_epoch_fee_per_sec, 7200 epoch duration. no congestion fee as threshold is not crossed.
        let expected_automation_fee = 50 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        let result = automation_registry::estimate_automation_fee(task_max_gas);
        assert!(result == expected_automation_fee, 2);

        // expected congestion fee with 86 % congestion
        // 6% surpass, ((1+(6/100))^exponent-1) * 100 = 41 (acf), task occupancy 5% epoch interval 7200
        let expected_congestion_fee = 41 * 5 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let result = automation_registry::estimate_automation_fee_with_committed_occupancy(task_max_gas, 162_000_000);
        assert!(result == expected_automation_fee + expected_congestion_fee, 2);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_registry_fee_success_withdrawal(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::set_locked_fee(100_000_000);
        let withdraw_amount = 99_999_999;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE - withdraw_amount;
        let expected_user_balance = ACCOUNT_BALANCE + withdraw_amount;
        automation_registry::withdraw_automation_task_fees(framework, address_of(user), withdraw_amount);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        check_account_balance(address_of(user), expected_user_balance);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EREQUEST_EXCEEDS_LOCKED_BALANCE, location = automation_registry)]
    fun check_registry_fee_failed_withdrawal_locked_balance(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        automation_registry::set_locked_fee(100_000_000);
        let withdraw_amount = REGISTRY_DEFAULT_BALANCE - 80_000_000;
        automation_registry::withdraw_automation_task_fees(framework, address_of(user), withdraw_amount);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EINSUFFICIENT_BALANCE, location = automation_registry)]
    fun check_registry_fee_failed_withdrawal_insufficient_balance(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let withdraw_amount = REGISTRY_DEFAULT_BALANCE + 1;
        automation_registry::withdraw_automation_task_fees(framework, address_of(user), withdraw_amount);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun test_registration_enable_disable(framework: &signer, user: &signer) {
        initialize_registry_test(framework, user);
        assert!(automation_registry::is_registration_enabled(), 14);

        automation_registry::disable_registration(framework);
        assert!(!automation_registry::is_registration_enabled(), 15);

        automation_registry::enable_registration(framework);
        assert!(automation_registry::is_registration_enabled(), 16);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ETASK_REGISTRATION_DISABLED, location = automation_registry)]
    fun test_register_fails_when_registration_disabled(
        framework: &signer, user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::disable_registration(framework);
        assert!(!automation_registry::is_registration_enabled(), 17);

        automation_registry::register_with_state(user,
            50,
            1000,
            86400,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1caff)]
    fun check_task_successful_stopped(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let automation_fee_cap = 1000;
        let max_gas_amount = 200;

        let t1 = automation_registry::register_with_state(
            user,
            max_gas_amount,
            automation_fee_cap,
            86400,
            ACTIVE,
        );
        let t2 = automation_registry::register_with_state(
            user,
            max_gas_amount,
            automation_fee_cap,
            86400,
            ACTIVE,
        );
        let t3 = automation_registry::register_with_state(
            user,
            max_gas_amount,
            automation_fee_cap,
            86400,
            ACTIVE,
        );
        let t4 = automation_registry::register_with_state(
            user,
            max_gas_amount,
            automation_fee_cap,
            86400,
            ACTIVE,
        );

        // check user balance after registered new task
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_account = address_of(user);
        let registration_charges = 4 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_account, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        let committed_gas_for_new_cycle = 4 * max_gas_amount;

        // Not only charges will be applied but also state will be updated to STARTED as all expected tasks will be processed.
        automation_registry::process_tasks_for_tests(2, vector[t1, t2, t3, t4]);

        assert!(committed_gas_for_new_cycle == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });

        // 0.002 (*4) - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee = 4 * (max_gas_amount * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100000);
        expected_current_balance = expected_current_balance - expected_automation_fee;
        expected_registry_balance = expected_registry_balance + expected_automation_fee;
        check_account_balance(user_account, expected_current_balance );
        check_account_balance( registry_fee_address, expected_registry_balance );

        timestamp::update_global_time_for_test_secs(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + (EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2)
        );

        // Stop task 2. and it's removed from active task list immediately
        automation_registry::stop_tasks(user, vector[2]);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });
        // There is no task with index 2 now.
        assert!(!automation_registry::has_task_with_id(2), 1);
        assert!(3 * max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 1);

        // Because the on of the task stopped halfway, the user gets a 50% refund for the unused time.
        // which is equivalent to a 25% refund of the full epoch for single task and deposited fee upon registration
        let expected_refund = (max_gas_amount * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100000) / 4 + automation_fee_cap;
        expected_current_balance = expected_current_balance + expected_refund;
        expected_registry_balance = expected_registry_balance - expected_refund;
        check_account_balance(user_account, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        // Add and stop the task in the same epoch. Task index will be 4
        assert!(automation_registry::get_next_task_index() == 4, 1);
        automation_registry::register_with_state(user,
            max_gas_amount,
            1000,
            86400,
            PENDING

        );
        assert!(automation_registry::has_task_with_id(4), 1);

        registration_charges = FLAT_REGISTRATION_FEE_TEST + automation_fee_cap;
        expected_current_balance = expected_current_balance - registration_charges;
        expected_registry_balance = expected_registry_balance + registration_charges;
        check_account_balance(user_account, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        // Stop newly added task
        automation_registry::stop_tasks(user, vector[4]);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });
        // There is no task with index 4 and the next task index will be 5.
        assert!(!automation_registry::has_task_with_id(4), 1);
        assert!(automation_registry::get_next_task_index() == 5, 1);
        assert!(3 * max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 1);

        // Expected refund for the stopping pending task is only the half of the deposited fee
        expected_refund = automation_fee_cap / REFUND_FACTOR;
        check_account_balance(user_account, expected_current_balance + expected_refund);
        check_account_balance(registry_fee_address, expected_registry_balance - expected_refund);
    }

    #[test(framework = @supra_framework, user = @0x1cafe, user2 = @0x1cafa)]
    #[expected_failure(abort_code = EUNAUTHORIZED_TASK_OWNER, location = automation_registry)]
    fun check_unauthorized_stopping_task(
        framework: &signer,
        user: &signer,
        user2: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        automation_registry::stop_tasks(user2, vector[0]);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_stopping_of_stopped_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        timestamp::update_global_time_for_test_secs(50);
        // Stop the same task 2 times, second time it will not abort it just skip the task_id if it's not found
        automation_registry::stop_tasks(user, vector[0]);
        assert!(!automation_registry::has_task_with_id(0), 1);
        automation_registry::stop_tasks(user, vector[0]);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_stopping_of_cancelled_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let automation_fee_cap = 1000;

        automation_registry::register_with_state(user,
            2000,
            automation_fee_cap,
            86400,
            PENDING,
        );
        assert!(2000 == automation_registry::get_gas_committed_for_next_epoch(), 1);

        // check user balance after registered new task
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + automation_fee_cap;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        // Start new cycle
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::process_tasks_for_tests( 2, vector[0]);

        // 0.002 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee = 2000 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100000;
        expected_current_balance = expected_current_balance - expected_automation_fee;
        expected_registry_balance = expected_registry_balance + expected_automation_fee;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance( registry_fee_address, expected_registry_balance );

        // Task is active state and after cancelling it, status will be update to cancelled
        automation_registry::cancel_task(user, 0);
        assert!(automation_registry::has_task_with_id(0), 1);
        assert!(0 == automation_registry::get_gas_committed_for_next_epoch(), 1);

        // balance is keep remain same
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);

        // After cancelling the task, the user stops it after 50% of the next epoch has passed.
        timestamp::update_global_time_for_test_secs(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + (EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2)
        );

        automation_registry::stop_tasks(user, vector[0]);
        assert!(!automation_registry::has_task_with_id(0), 1);
        assert!(0 == automation_registry::get_gas_committed_for_next_epoch(), 1);

        // Because the on of the task stopped after 50% epoch time passed, the user gets a 50% refund for the unused time.
        // which is equivalent to a 25% refund of the full epoch for single task + refund of deposited amount upon registration
        let refund_automation_fee = (2000 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100000) / 4 + automation_fee_cap;
        check_account_balance( user_address, expected_current_balance + refund_automation_fee );
        check_account_balance( registry_fee_address, expected_registry_balance - refund_automation_fee );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ECYCLE_TRANSITION_IN_PROGRESS, location = automation_registry)]
    fun check_stopping_in_transition_state(
        framework: &signer,
        user: &signer,
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_with_state(user,
            10,
            1000,
            86400,
            PENDING,
        );
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        automation_registry::stop_tasks(user, vector[0]);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_bookkeeping_refunds_and_unlocks(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let automation_fee_cap = 1000;
        let max_gas_amout = 2000;

        let user_address = address_of(user);


        automation_registry::register_with_state(user,
            2000,
            automation_fee_cap,
            86400,
            PENDING
        );

        let expected_user_balance = ACCOUNT_BALANCE - automation_fee_cap - FLAT_REGISTRATION_FEE_TEST;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + automation_fee_cap + FLAT_REGISTRATION_FEE_TEST;
        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);

        let expected_total_locked = automation_fee_cap;
        automation_registry::check_gas_and_fees_for_cycle(0, max_gas_amout, 0, expected_total_locked);

        // refund only 10 % and unlock half of the initial deposit;
        let refund = automation_fee_cap / 10;
        let unlock = automation_fee_cap / 2;
        let result = automation_registry::safe_deposit_refund_for_tests(
            0,
            user_address,
            refund,
            unlock);
        assert!(result, 1);

        expected_user_balance = expected_user_balance + refund;
        expected_registry_balance = expected_registry_balance - refund;
        expected_total_locked = expected_total_locked - unlock;

        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        automation_registry::check_gas_and_fees_for_cycle(0, max_gas_amout, 0, expected_total_locked);

        // try to refund availbale amount but unlock more than is locked balance
        // Niether unlock nor refund should succeed.
        let result = automation_registry::safe_deposit_refund_for_tests(
            0,
            user_address,
            refund,
            automation_fee_cap);
        assert!(!result, 3);

        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        automation_registry::check_gas_and_fees_for_cycle(0, max_gas_amout, 0, expected_total_locked);

        // try to refund more then registry account has but unlock acceptable amount of deposit.
        // Unlock will succeed but not refund.
        let result = automation_registry::safe_deposit_refund_for_tests(
            0,
            user_address,
            expected_registry_balance + automation_fee_cap,
            unlock);
        assert!(!result, 4);

        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        automation_registry::check_gas_and_fees_for_cycle(0, max_gas_amout, 0, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_epoch_fee_refunds_and_unlocks(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let locked_epoch_fee = 1000;

        let user_address = address_of(user);

        automation_registry::set_locked_fee(locked_epoch_fee);

        let expected_user_balance = ACCOUNT_BALANCE;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE;

        // refund only 10 % and unlock half of the initial deposit;
        let refund = locked_epoch_fee / 10;
        let (result, remaining_epoch_locked_fees) = automation_registry::safe_fee_refund_for_tests(
            locked_epoch_fee,
            0,
            user_address,
            refund, );
        assert!(result, 1);

        expected_user_balance = expected_user_balance + refund;
        expected_registry_balance = expected_registry_balance - refund;
        let expected_total_locked = locked_epoch_fee - refund;

        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        assert!(remaining_epoch_locked_fees == expected_total_locked, 2);

        // try to refund more than locked
        // Niether unlock nor refund should succeed.
        let (result, remaining_epoch_locked_fees) = automation_registry::safe_fee_refund_for_tests(
            remaining_epoch_locked_fees,
            0,
            user_address,
            locked_epoch_fee,
        );
        assert!(!result, 3);

        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        assert!(remaining_epoch_locked_fees == expected_total_locked, 4);

        // Assume there is no enough balance to refund the epoch fee in registry account.
        // No refund but fee is unlocked.
        let epoch_locked_fees = REGISTRY_DEFAULT_BALANCE;

        let (result, remaining_epoch_locked_fees) = automation_registry::safe_fee_refund_for_tests(
            epoch_locked_fees,
            0,
            user_address,
            expected_registry_balance + 1,
        );
        assert!(!result, 3);

        check_account_balance(user_address, expected_user_balance);
        check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_balance);
        assert!(remaining_epoch_locked_fees == epoch_locked_fees - expected_registry_balance - 1, 4);
    }



    // Register 500 tasks to measure registration time/used-gas
    #[test_only]
    fun task_registration_performance(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let count = 0;
        let exp_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let max_task_count = (TASK_CAPACITY_TEST as u64);
        while (count < max_task_count) {
            automation_registry::register_with_state(user,
                10000,
                1000000,
                exp_time + EPOCH_INTERVAL_FOR_TEST_IN_SECS * (count % 2),
                PENDING,
            );
            count = count + 1;
        };

        // No active task and committed gas for the next epoch is total of the all registered tasks
        assert!(10000 * max_task_count == automation_registry::get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        assert!(active_task_ids == vector[], 1);
    }

    #[test_only]
    // Kept only for performance analysis intentions
    // Register 500 tasks to measure registration time/used-gas
    // #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_registration_performance(
        framework: &signer,
        user: &signer
    ) {
        task_registration_performance(framework, user);
    }

    #[test_only]
    // Kept only for performance analysis intentions
    // Register 500 tasks with 1.5 EPOCH_INTERVAL duration/expiration time
    // And run 3 epochs to check gas-used when
    //  - full epoch passed
    //  - 1/3 of epoch passed to simulate refund, but tasks are still active
    //  - last epoch identifies all tasks are expired
    // #[test(framework = @supra_framework, user = @0x1cafe)]
    fun process_tasks_in_batch_performance(
        cycle_index:u64,
    ) {
        let task_indexes = automation_registry::get_task_ids();
        let count = vector::length(&task_indexes);
        let i = 0;
        let batch = 25;
        while (i < count) {
            let task_partition = vector::range(i, i + batch);
            automation_registry::process_tasks_for_tests( cycle_index, task_partition);
            i = i + batch;
        };
    }

    // #[test_only]
    // Kept only for performance analysis intentions
    // Register 500 tasks with 1.5 EPOCH_INTERVAL duration/expiration time
    // And run 3 epochs to check gas-used when
    //  - full epoch passed
    //  - 1/3 of epoch passed to simulate refund, but tasks are still active
    //  - last epoch identifies all tasks are expired
    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_activation_on_new_epoch_performance(
        framework: &signer,
        user: &signer
    ) {
        task_registration_performance(framework, user);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        process_tasks_in_batch_performance(2);

        timestamp::update_global_time_for_test_secs(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS
        );
        automation_registry::monitor_cycle_end();
        process_tasks_in_batch_performance(3);

        timestamp::update_global_time_for_test_secs(3 * EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        process_tasks_in_batch_performance(4);
    }


    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_successful_transition_from_finished_to_ready(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task2 = automation_registry::register_with_state(
            user,
            44_000_000,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);


        // Update time so task3 is expired.
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
        // Make sure we are in FINISHED state
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);
        // Make sure that we attempt to drop only cancelled and expired tasks, to avoid any asserts in this scenario
        automation_registry::process_tasks_for_tests( 2, vector[task1]);

        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_FINISHED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, HAS_TRANSITION_STATE);

        // Make sure that we attempt to drop only cancelled and expired tasks, to avoid any asserts in this scenario
        automation_registry::process_tasks_for_tests( 2, vector[task2]);
        {
            automation_registry::check_gas_and_fees_for_cycle(0,
                0,
                0,
                0);

            // We move directly to READY state as after drops the cycle moves to STARTED state with empty task
            // list so there is nothing to refund.
            automation_registry::check_cycle_state_and_duration(CYCLE_READY, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);

            assert!(!automation_registry::has_task_with_id(task1), 5);
            assert!(!automation_registry::has_task_with_id(task2), 6);
        };
    }



    #[test(framework = @supra_framework)]
    fun check_monitor_cycle_end_when_feature_flags_are_disabled(framework: &signer)
    {
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], false);
        initialize_registry_only_test(framework);
        automation_registry::check_cycle_state(CYCLE_READY, 0, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Updating the time which should cause cycle end if in proper state
        timestamp::update_global_time_for_test_secs(2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Both SUPRA_NATIVE_AUTOMATION and SUPRA_AUTOMATION_V2 are disabled
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_READY, 0, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        //Enable feature
        toggle_feature_flag(framework, true);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_READY, 0, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        //Disable feature enable automation cycle
        toggle_feature_flag(framework, false);
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], true);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_READY, 0, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        //Both enabled feature and automation cycle,
        // but as long as registry is not in STARTED state no transistion will happen even if cycle duration passed
        toggle_feature_flag(framework, true);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_READY, 0, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_monitor_cycle_end_from_started_state(framework: &signer, user: &signer)
    {
        initialize_registry_test(framework, user);
        automation_registry::check_cycle_state(CYCLE_STARTED, 1, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Updating the time which should cause cycle end if in proper state
        timestamp::update_global_time_for_test_secs(2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // SUPRA_AUTOMATION_V2 is disabled
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], false);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_STARTED, 1, 0, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Both feature flags are enabled, but as long as registry is empty we will only progress
        // in cycle and remain in started state.
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], true);
        automation_registry::monitor_cycle_end();
        let recent_chain_time = timestamp::now_seconds();
        automation_registry::check_cycle_state(CYCLE_STARTED, 2, recent_chain_time, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Register a task to move to FINISHED state if cycle end is identified
        automation_registry::register_with_state(user,
            2000,
            100_000_000,
            86400,
            PENDING
        );

        // Chain time remains the same, so no transition will happen
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_STARTED, 2, recent_chain_time, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Chain time updated but epoch end was not reached
        timestamp::update_global_time_for_test_secs(recent_chain_time + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_STARTED, 2, recent_chain_time, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Chain time remains the same, so no transition will happen
        timestamp::update_global_time_for_test_secs(recent_chain_time + EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::check_cycle_state(CYCLE_FINISHED, 2, recent_chain_time, EPOCH_INTERVAL_FOR_TEST_IN_SECS);

        // Process the single task which will lead to the state to be updated to STARTED again
        recent_chain_time = timestamp::now_seconds();
        automation_registry::process_tasks_for_tests( 3, vector[0]);
        automation_registry::check_cycle_state(CYCLE_STARTED, 3, recent_chain_time, EPOCH_INTERVAL_FOR_TEST_IN_SECS);
    }


    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_migration(framework: &signer, user: &signer)
    {
        initialize_registry_test(framework, user);

        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;
        let task_exists = true;
        let t1_t2_max_gas_amount = 44_000_000;
        let t3_max_gas_amount = 11_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            ACTIVE);
        let task2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);
        let task3 = automation_registry::register_with_state(
            user,
            11_000_000,
            automation_fee_cap,
            task_exipry_time,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * (FLAT_REGISTRATION_FEE_TEST + automation_fee_cap);


        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task_1_2 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task_1_2 = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;

        // 11/100 * 1000 = 110 - automation_epoch_fee_per_second, 7200 epoch duration (PENDING tasks are charge for the first cycle fully)
        let expected_automation_fee_per_task_3 = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 110;

        let user_address = address_of(user);

        // set enough cycle fee to be able to refund
        automation_registry::set_locked_fee(10_000_000_000);

        automation_registry::prepare_state_for_migration(framework);
        let epoch_data_exists = true;
        let cycle_data_exists = true;
        automation_registry::check_epoch_and_cycle_resources(epoch_data_exists, !cycle_data_exists);

        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], false);
        // Simulate that half of the epoch passed, when migration was requested
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        automation_registry::migrate_v2(framework, EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );

        automation_registry::check_epoch_and_cycle_resources(!epoch_data_exists, cycle_data_exists);
        assert!(automation_registry::is_feature_enabled_and_initialized(), 2);
        automation_registry::check_cycle_state(CYCLE_FINISHED, 0, timestamp::now_seconds(), EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        {
            // Check refunds have been done for both task1 and task2 only
            let total_expected_fee_refunds_task_1_2 = expected_automation_fee_per_task_1_2 + expected_congestion_fee_per_task_1_2;
            expected_user_current_balance = expected_user_current_balance + total_expected_fee_refunds_task_1_2;
            expected_registry_current_balance = expected_registry_current_balance - total_expected_fee_refunds_task_1_2;
            check_account_balance(user_address, expected_user_current_balance);
            check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);

            automation_registry::check_task_state(task1, task_exists, ACTIVE);
            automation_registry::check_task_state(task2, task_exists, CANCELLED);
            automation_registry::check_task_state(task3, task_exists, PENDING);

        };

        // after epoch change we are still in the same state and waiting for actions from native layer to have tasks processed for the new cycle
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state(CYCLE_FINISHED, 0, timestamp::now_seconds(), EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        {
            // Check refunds have been done for both task1 and task2 only
            check_account_balance(user_address, expected_user_current_balance);
            check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance);

            automation_registry::check_task_state(task1, task_exists, ACTIVE);
            automation_registry::check_task_state(task2, task_exists, CANCELLED);
            automation_registry::check_task_state(task3, task_exists, PENDING);

        };

        // Process tasks and check the registry state after it
        let total_committed_gas_for_new_cycle = t1_t2_max_gas_amount + t3_max_gas_amount; // task 2 was cancelled.
        automation_registry::process_tasks_for_tests( 1, vector[task1, task2, task3]);
        automation_registry::check_cycle_state(CYCLE_STARTED, 1, timestamp::now_seconds(), EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        {
            // Check charges are done only for task 1 and task3 and task 3 was refunded with deposit fee.

            // As cycle duration is half of the previous epoch duration, and these values are calculated for epoch duraton.
            let total_expected_fee_charges_task_1_3 = (expected_automation_fee_per_task_1_2 + expected_automation_fee_per_task_3) / 2;

            check_account_balance(user_address, expected_user_current_balance - total_expected_fee_charges_task_1_3 + automation_fee_cap);
            check_account_balance(automation_registry::get_registry_fee_address(), expected_registry_current_balance + total_expected_fee_charges_task_1_3 - automation_fee_cap);

            automation_registry::check_gas_and_fees_for_cycle(
                total_committed_gas_for_new_cycle,
                total_committed_gas_for_new_cycle,
                total_expected_fee_charges_task_1_3,
                2 * automation_fee_cap
            );
            let active_task_ids = automation_registry::get_active_task_ids();
            assert!(vector::contains(&active_task_ids, &task1), 6);
            assert!(vector::contains(&active_task_ids, &task3), 7);
            assert!(!vector::contains(&active_task_ids, &task2), 7);

            automation_registry::check_task_state(task1, task_exists, ACTIVE);
            automation_registry::check_task_state(task3, task_exists, ACTIVE);
            assert!(!automation_registry::has_task_with_id(task2), 9);
        };
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EINVALID_MIGRATION_ACTION, location = automation_registry)]
    fun check_migration_fails_on_second_round_even_if_automation_cycle_is_not_enabled(framework: &signer, user: &signer)
    {
        initialize_registry_test(framework, user);

        automation_registry::prepare_state_for_migration(framework);
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], false);
        // Simulate that half of the epoch passed, when migration was requested
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        automation_registry::migrate_v2(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );

        let epoch_data_exists = true;
        let cycle_data_exists = true;
        automation_registry::check_epoch_and_cycle_resources(!epoch_data_exists, cycle_data_exists);
        assert!(automation_registry::is_feature_enabled_and_initialized(), 2);
        automation_registry::check_cycle_state(CYCLE_STARTED, 1, timestamp::now_seconds(), EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        automation_registry::migrate_v2(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EINVALID_MIGRATION_ACTION, location = automation_registry)]
    fun check_migration_fails_on_second_round(framework: &signer, user: &signer)
    {
        initialize_registry_test(framework, user);

        automation_registry::prepare_state_for_migration(framework);
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], false);
        // Simulate that half of the epoch passed, when migration was requested
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        automation_registry::migrate_v2(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
        toggle_custom_feature_flags(framework, vector[features::get_supra_automation_v2_feature()], true);
        automation_registry::on_new_epoch();

        let epoch_data_exists = true;
        let cycle_data_exists = true;
        automation_registry::check_epoch_and_cycle_resources(!epoch_data_exists, cycle_data_exists);
        assert!(automation_registry::is_feature_enabled_and_initialized(), 2);

        automation_registry::check_cycle_state(CYCLE_STARTED, 1, timestamp::now_seconds(), EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);

        automation_registry::migrate_v2(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_transitions_to_ready_from_suspended(framework: &signer, user: &signer)
    {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;
        let t1_t2_max_gas_amount = 44_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            ACTIVE);

        let new_cycle_duration = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        automation_registry::update_config_v2(
            framework,
            3 * EPOCH_INTERVAL_FOR_TEST_IN_SECS ,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            new_cycle_duration,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );

        let expected_cycle_duration = automation_registry::check_cycle_state_and_duration(CYCLE_STARTED, EPOCH_INTERVAL_FOR_TEST_IN_SECS, !HAS_TRANSITION_STATE);
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();
        automation_registry::check_cycle_state_and_duration(CYCLE_SUSPENDED, expected_cycle_duration, HAS_TRANSITION_STATE);
        automation_registry::check_cycle_new_duration(expected_cycle_duration);
        // updated configs are not read from buffer
        assert!(config_buffer::does_exist<AutomationRegistryConfigV2>(), 6);

        // Process tasks to transition to ready state;
        automation_registry::process_tasks_for_tests(1, vector[task1]);
        automation_registry::check_cycle_state_and_duration(CYCLE_READY, expected_cycle_duration, !HAS_TRANSITION_STATE);
        // still, updated configs are not read from buffer
        assert!(config_buffer::does_exist<AutomationRegistryConfigV2>(), 9);


    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_transitions_to_ready_from_finished_suspended(framework: &signer, user: &signer)
    {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let automation_fee_cap = 100_000_000;
        let t1_t2_max_gas_amount = 44_000_000;

        let task1 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);

        let task2 = automation_registry::register_with_state(
            user,
            t1_t2_max_gas_amount,
            automation_fee_cap,
            task_exipry_time,
            CANCELLED);

        let new_cycle_duration = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        automation_registry::update_config_v2(
            framework,
            3 * EPOCH_INTERVAL_FOR_TEST_IN_SECS ,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
            new_cycle_duration,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            SYS_TASK_CAPACITY_TEST
        );
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        toggle_feature_flag(framework, false);
        automation_registry::on_new_epoch();

        automation_registry::check_cycle_state_and_duration(
            CYCLE_SUSPENDED,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            HAS_TRANSITION_STATE);

        // Process tasks to transition to ready state;
        automation_registry::process_tasks_for_tests( 1, vector[task1, task2]);
        automation_registry::check_cycle_transition_state(CYCLE_READY,
            0, new_cycle_duration, 0, 0, 0, 0, 0, 0, 0
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe, multisig_user = @0xad123)]
    fun check_system_task_registration(
        framework: &signer,
        user: &signer,
        multisig_user: &signer
    ) {
        let (multisig_address, multisig_signer) = initialize_sys_registry_test(framework, user, multisig_user);
        automation_registry::grant_authorization(framework, multisig_address);

        let max_gas_amount = 10;
        let estimated_fee = automation_registry::estimate_automation_fee(max_gas_amount);
        automation_registry::register_with_state(user,
            max_gas_amount,
            estimated_fee,
            86400,
            PENDING
        );
        assert!(1 == automation_registry::get_next_task_index(), 1);
        assert!(max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 2);

        // calculate estimated fee in case of congestion before having any system task registered
        let max_gas_amount_causing_congestion = (AUTOMATION_MAX_GAS_TEST * (CONGESTION_THRESHOLD_TEST as u64)) / 100;
        let estimated_fee_with_congestion = automation_registry::estimate_automation_fee(max_gas_amount_causing_congestion);

        // register a system task
        automation_registry::register_system_task_with_state(&multisig_signer,
            max_gas_amount,
            86400,
            PENDING
        );
        assert!(2 == automation_registry::get_next_task_index(), 3);
        assert!(max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 4);
        assert!(max_gas_amount == automation_registry::get_system_gas_committed_for_next_cycle(), 5);
        let registered_system_tasks = automation_registry::get_system_task_indexes();
        assert!(vector::contains(&registered_system_tasks, &1), 6);
        assert!(!vector::contains(&registered_system_tasks, &0), 7);
        assert!(automation_registry::get_system_task_count() == 1, 9);

        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + estimated_fee;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(address_of(multisig_user), ACCOUNT_BALANCE);

        automation_registry::register_with_state(user,
            max_gas_amount_causing_congestion,
            estimated_fee_with_congestion,
            86400,
            PENDING,
        );
        assert!(3 == automation_registry::get_next_task_index(), 6);
        assert!(max_gas_amount_causing_congestion + max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 7);
        assert!(max_gas_amount == automation_registry::get_system_gas_committed_for_next_cycle(), 8);

        let registration_charges = FLAT_REGISTRATION_FEE_TEST + estimated_fee_with_congestion;
        let expected_current_balance = expected_current_balance - registration_charges;
        let expected_registry_balance = expected_registry_balance + registration_charges;
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);
    }

    #[test(framework = @supra_framework, user = @0x1cafe, multisig_user = @0xad123)]
    fun check_system_task_activation(
        framework: &signer,
        user: &signer,
        multisig_user: &signer
    ) {
        let (multisig_address, multisig_signer) = initialize_sys_registry_test(framework, user, multisig_user);
        automation_registry::grant_authorization(framework, multisig_address);

        let max_gas_amount = 10;
        let estimated_fee = automation_registry::estimate_automation_fee(max_gas_amount);
        automation_registry::register_with_state(user,
            max_gas_amount,
            estimated_fee,
            86400,
            PENDING,

        );
        assert!(1 == automation_registry::get_next_task_index(), 1);
        assert!(max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 2);

        // register a system task
        automation_registry::register_system_task_with_state(&multisig_signer,
            max_gas_amount,
            86400,
            PENDING
        );
        assert!(2 == automation_registry::get_next_task_index(), 3);
        assert!(max_gas_amount == automation_registry::get_gas_committed_for_next_epoch(), 4);
        assert!(max_gas_amount == automation_registry::get_system_gas_committed_for_next_cycle(), 5);
        let registered_system_tasks = automation_registry::get_system_task_indexes();
        assert!(vector::contains(&registered_system_tasks, &1), 6);
        assert!(!vector::contains(&registered_system_tasks, &0), 7);
        assert!(automation_registry::get_system_task_count() == 1, 8);

        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        let registration_charges = FLAT_REGISTRATION_FEE_TEST + estimated_fee;
        let expected_current_balance = ACCOUNT_BALANCE - registration_charges;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE + registration_charges;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, expected_registry_balance);
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(address_of(multisig_user), ACCOUNT_BALANCE);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::process_tasks_for_tests( 2, vector[0, 1]);

        // Check the balance of the accounts, make sure that multisig account is not charged
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(address_of(multisig_user), ACCOUNT_BALANCE);
        check_account_balance(user_address, expected_current_balance - estimated_fee);
        check_account_balance(registry_fee_address, expected_registry_balance + estimated_fee);
        assert!(automation_registry::has_sender_active_task_with_id(user_address, 0), 9);
        assert!(automation_registry::has_sender_active_system_task_with_id(multisig_address, 1), 10);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_EXPIRY_TIME, location = automation_registry)]
    fun check_system_task_registration_invalid_expiry_time(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        timestamp::update_global_time_for_test_secs(50);
        automation_registry::register_system_task_with_state(
            &multisig_signer,
            70,
            25,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EEXPIRY_BEFORE_NEXT_CYCLE, location = automation_registry)]
    fun check_system_task_registration_invalid_expiry_time_before_next_epoch(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            70,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            PENDING,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EEXPIRY_TIME_UPPER, location = automation_registry)]
    fun check_system_task_registration_invalid_expiry_time_surpassing_task_duration_cap(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            70,
            SYS_TASK_DURATION_CAP_IN_SECS + 1,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_system_task_registration_valid_expiry_time_matches_task_duration_cap(
        framework: &signer,
        user: &signer
    )  {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            70,
            SYS_TASK_DURATION_CAP_IN_SECS,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNAUTHORIZED_SYSTEM_ACCOUNT, location = automation_registry)]
    fun check_system_task_registration_with_unauthorized_signer(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        automation_registry::register_system_task_with_state(user,
            70,
            SYS_TASK_DURATION_CAP_IN_SECS + 1,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EREGISTRY_IS_FULL, location = automation_registry)]
    fun check_system_task_registration_with_full_tasks(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::update_system_task_config_for_tests(
            framework,
            SYS_TASK_DURATION_CAP_IN_SECS,
            SYS_AUTOMATION_MAX_GAS_TEST,
            2,
        );
        automation_registry::register_system_task_with_state(&multisig_signer,
            10,
            86400,
            PENDING,
        );
        automation_registry::register_system_task_with_state(&multisig_signer,
            10,
            86400,
            PENDING,
        );
        // Registry is already full
        automation_registry::register_system_task_with_state(&multisig_signer,
            10,
            86400,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EGAS_AMOUNT_UPPER, location = automation_registry)]
    fun check_system_task_registration_with_overflow_gas_limit(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);
        let max_gas_amount = SYS_AUTOMATION_MAX_GAS_TEST / 2;
        automation_registry::register_system_task_with_state(&multisig_signer,
            max_gas_amount,
            86400,
            PENDING
        );
        assert!(1 == automation_registry::get_next_task_index(), 1);
        assert!(max_gas_amount == automation_registry::get_system_gas_committed_for_next_cycle(), 1);
        automation_registry::register_system_task_with_state(&multisig_signer,
            max_gas_amount + 1,
            86400,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_MAX_GAS_AMOUNT, location = automation_registry)]
    fun check_system_registration_invalid_max_gas_amount(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);
        automation_registry::register_system_task_with_state(&multisig_signer,
            0,
            86400,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TXN_HASH, location = automation_registry)]
    fun check_system_task_registration_invalid_parent_hash(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);
        automation_registry::register_system_task_with_custom_input(&multisig_signer,
            10,
            86400,
            vector<u8>[0, 1, 2, 3],
            SYS_AUX_DATA,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TASK_TYPE, location = automation_registry)]
    fun check_system_task_registration_with_invalid_task_type_data(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);
        automation_registry::register_system_task_with_custom_input(&multisig_signer,
            10,
            86400,
            PARENT_HASH,
            AUX_DATA,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_AUX_DATA_LENGTH, location = automation_registry)]
    fun check_system_task_registration_without_aux_data(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);
        let aux_data = vector[];
        automation_registry::register_system_task_with_custom_input(&multisig_signer,
            10,
            86400,
            PARENT_HASH,
            aux_data,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TASK_TYPE_LENGTH, location = automation_registry)]
    fun check_system_task_registration_with_invalid_task_type(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);
        let aux_data = vector[vector[1, 2], vector[]];
        automation_registry::register_system_task_with_custom_input(&multisig_signer,
            10,
            86400,
            PARENT_HASH,
            aux_data,
            PENDING
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_system_task_successful_cancellation(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        let _ = automation_registry::register_system_task_with_state(
            &multisig_signer,
            10,
            86400,
            ACTIVE
        );
        let _ = automation_registry::register_system_task_with_state(
            &multisig_signer,
            10,
            86400,
            ACTIVE
        );
        // check account balances after registration
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_address = address_of(user);
        check_account_balance(user_address, ACCOUNT_BALANCE);
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(registry_fee_address, REGISTRY_DEFAULT_BALANCE);

        assert!(20 == automation_registry::get_system_gas_committed_for_next_cycle(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1];
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });

        // Cancel task 2. The committed gas for the next epoch will be updated,
        // but when requested active task it will be still available in the list
        automation_registry::cancel_system_task(&multisig_signer, 1);
        // Task will be still available in the registry but with cancelled state
        automation_registry::check_task_state(1, true, CANCELLED);

        assert!(10 == automation_registry::get_system_gas_committed_for_next_cycle(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });

        // Add and cancel the task in the same epoch. Task index will be 4
        assert!(automation_registry::get_next_task_index() == 2, 1);
        automation_registry::register_system_task_with_state(&multisig_signer,
            10,
            86400,
            PENDING,
        );
        automation_registry::cancel_system_task(&multisig_signer, 2);
        assert!(10 == automation_registry::get_system_gas_committed_for_next_cycle(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });
        // there is no task with index 2 and the next task index will be 3.
        assert!(!automation_registry::has_task_with_id(2), 1);
        assert!(automation_registry::get_next_task_index() == 3, 1);

        // Check account balances after cancellation
        check_account_balance(user_address, ACCOUNT_BALANCE);
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(registry_fee_address, REGISTRY_DEFAULT_BALANCE);
    }


    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ESYSTEM_AUTOMATION_TASK_NOT_FOUND, location = automation_registry)]
    fun check_cancellation_of_non_existing_system_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::cancel_system_task(&multisig_signer, 1);
    }

    #[test(framework = @supra_framework, user = @0x1cafe, user2 = @0x1cafa)]
    #[expected_failure(abort_code = EUNAUTHORIZED_TASK_OWNER, location = automation_registry)]
    fun check_unauthorized_system_task_cancellation(
        framework: &signer,
        user: &signer,
        user2: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            10,
            86400,
            PENDING
        );
        automation_registry::cancel_system_task(user2, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EALREADY_CANCELLED, location = automation_registry)]
    fun check_cancellation_of_cancelled_system_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        let _ = automation_registry::register_system_task_with_state(
            &multisig_signer,
            10,
            86400,
            ACTIVE
        );
        // Cancel the same task 2 times
        automation_registry::cancel_system_task(&multisig_signer, 0);
        automation_registry::cancel_system_task(&multisig_signer, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNSUPPORTED_TASK_OPERATION, location = automation_registry)]
    fun check_system_task_cancellation_via_user_api(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        let _ = automation_registry::register_system_task_with_state(
            &multisig_signer,
            10,
            86400,
            ACTIVE
        );
        automation_registry::cancel_task(user, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ESYSTEM_AUTOMATION_TASK_NOT_FOUND, location = automation_registry)]
    fun check_user_task_cancellation_via_system_api(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);

        let _ = automation_registry::register_with_state(
            user,
            10,
            1000,
            86400,
            ACTIVE
        );
        automation_registry::cancel_system_task(user, 0);
    }

    #[test(framework = @supra_framework, user = @0x1caff)]
    fun check_system_task_successful_stopped(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        let max_gas_amount = 200;

        let t1 = automation_registry::register_system_task_with_state(
            &multisig_signer,
            max_gas_amount,
            86400,
            ACTIVE,
        );
        let t2 = automation_registry::register_system_task_with_state(
            &multisig_signer,
            max_gas_amount,
            86400,
            ACTIVE,
        );
        let t3 = automation_registry::register_system_task_with_state(
            &multisig_signer,
            max_gas_amount,
            86400,
            ACTIVE,
        );
        let t4 = automation_registry::register_system_task_with_state(
            &multisig_signer,
            max_gas_amount,
            86400,
            ACTIVE,
        );

        // Check account balances before stopping
        let registry_fee_address = automation_registry::get_registry_fee_address();
        let user_account = address_of(user);
        check_account_balance( multisig_address, ACCOUNT_BALANCE );
        check_account_balance( user_account, ACCOUNT_BALANCE );
        check_account_balance( registry_fee_address, REGISTRY_DEFAULT_BALANCE );


        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        let committed_gas_for_new_cycle = 4 * max_gas_amount;

        // Not only charges will be applied but also state will be updated to STARTED as all expected tasks will be processed.
        automation_registry::process_tasks_for_tests( 2, vector[t1, t2, t3, t4]);

        assert!(committed_gas_for_new_cycle == automation_registry::get_system_gas_committed_for_current_cycle(), 1);
        let active_task_ids = automation_registry::get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 2);
        });

        // 0.002 (*4) - automation_epoch_fee_per_second, 7200 epoch duration
        check_account_balance( multisig_address, ACCOUNT_BALANCE );
        check_account_balance(user_account, ACCOUNT_BALANCE );
        check_account_balance( registry_fee_address, REGISTRY_DEFAULT_BALANCE );

        timestamp::update_global_time_for_test_secs(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + (EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2)
        );

        // Stop task 2. and it's removed from active task list immediately
        automation_registry::stop_system_tasks(&multisig_signer, vector[2]);
        let active_task_ids = automation_registry::get_active_task_ids();
        let available_system_task_ids = automation_registry::get_system_task_indexes();
        let expected_ids = vector<u64>[0, 1, 3];
        assert!(vector::length(&active_task_ids) == vector::length(&available_system_task_ids), 1);
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&available_system_task_ids, &task_index), 1);
        });
        // There is no task with index 2 now.
        assert!(!automation_registry::has_task_with_id(2), 1);
        assert!(3 * max_gas_amount == automation_registry::get_system_gas_committed_for_next_cycle(), 1);

        // Add and stop the task in the same epoch. Task index will be 4
        assert!(automation_registry::get_next_task_index() == 4, 1);
        automation_registry::register_system_task_with_state(&multisig_signer,
            max_gas_amount,
            86400,
            PENDING
        );
        assert!(automation_registry::has_task_with_id(4), 1);


        // Stop newly added task
        automation_registry::stop_system_tasks(&multisig_signer, vector[4]);
        let active_task_ids = automation_registry::get_active_task_ids();
        let available_system_task_ids = automation_registry::get_system_task_indexes();
        let expected_ids = vector<u64>[0, 1, 3];
        assert!(vector::length(&active_task_ids) == vector::length(&available_system_task_ids), 1);
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&active_task_ids, &task_index), 1);
        });
        vector::for_each(expected_ids, |task_index| {
            assert!(vector::contains(&available_system_task_ids, &task_index), 1);
        });
        // There is no task with index 4 and the next task index will be 5.
        assert!(!automation_registry::has_task_with_id(4), 1);
        assert!(automation_registry::get_next_task_index() == 5, 1);
        assert!(3 * max_gas_amount == automation_registry::get_system_gas_committed_for_next_cycle(), 1);

        // Check balances after test execution
        check_account_balance(multisig_address, ACCOUNT_BALANCE);
        check_account_balance(user_account, ACCOUNT_BALANCE);
        check_account_balance(registry_fee_address, REGISTRY_DEFAULT_BALANCE);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_stopping_of_stopped_system_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            10,
            86400,
            PENDING,
        );
        timestamp::update_global_time_for_test_secs(50);
        // Stop the same task 2 times, second time it will not abort it just skip the task_id if it's not found
        automation_registry::stop_system_tasks(&multisig_signer, vector[0]);
        assert!(!automation_registry::has_task_with_id(0), 1);
        automation_registry::stop_system_tasks(&multisig_signer, vector[0]);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_stopping_of_cancelled_system_task(
        framework: &signer,
        user: &signer
    ) {
        initialize_registry_test(framework, user);
        let (multisig_address, multisig_signer) = setup_multisig_account(framework, user);
        automation_registry::grant_authorization(framework, multisig_address);

        automation_registry::register_system_task_with_state(&multisig_signer,
            2000,
            86400,
            PENDING
        );
        assert!(2000 == automation_registry::get_system_gas_committed_for_next_cycle(), 1);

        // check user balance after registered new task

        // Start new cycle
        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        automation_registry::monitor_cycle_end();
        automation_registry::process_tasks_for_tests( 2, vector[0]);

        // Task is active state and after cancelling it, status will be update to cancelled
        automation_registry::cancel_system_task(&multisig_signer, 0);
        assert!(automation_registry::has_task_with_id(0), 1);
        assert!(0 == automation_registry::get_system_gas_committed_for_next_cycle(), 1);

        // After cancelling the task, the user stops it after 50% of the next epoch has passed.
        timestamp::update_global_time_for_test_secs(
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + (EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2)
        );

        automation_registry::stop_system_tasks(&multisig_signer, vector[0]);
        assert!(!automation_registry::has_task_with_id(0), 1);
        assert!(0 == automation_registry::get_system_gas_committed_for_next_cycle(), 1);
    }

    #[test(framework = @supra_framework, user = @0x1cafe, user2 = @0x2da5ef)]
    fun check_account_authorization(
        framework: &signer,
        user: &signer,
        user2: &signer
    ) {
        initialize_registry_test(framework, user);
        // To make sure that account resource exists
        topup_account(framework, user2, 1000);
        let (multisig_address, _multisig_signer) = setup_multisig_account(framework, user);
        let (multisig_address2, _multisig_signer2) = setup_multisig_account(framework, user2);
        automation_registry::grant_authorization(framework, multisig_address);
        automation_registry::grant_authorization(framework, multisig_address2);
        assert!(automation_registry::is_authorized_account(multisig_address), 1);
        assert!(automation_registry::is_authorized_account(multisig_address2), 2);
        automation_registry::revoke_authorization(framework, multisig_address);
        assert!(!automation_registry::is_authorized_account(multisig_address), 3);
        assert!(automation_registry::is_authorized_account(multisig_address2), 4);
    }

}
