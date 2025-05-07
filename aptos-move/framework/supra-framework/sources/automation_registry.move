/// Supra Automation Registry
///
/// This contract is part of the Supra Framework and is designed to manage automated task entries
module supra_framework::automation_registry {

    use std::features;
    use std::signer;
    use std::vector;
    use aptos_std::math64;

    use supra_std::enumerable_map::{Self, EnumerableMap};

    use supra_framework::account::{Self, SignerCapability};
    use supra_framework::coin;
    use supra_framework::config_buffer;
    use supra_framework::create_signer::create_signer;
    use supra_framework::event;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::system_addresses;
    use supra_framework::timestamp;

    #[test_only]
    use std::signer::address_of;

    friend supra_framework::block;
    friend supra_framework::reconfiguration;
    friend supra_framework::genesis;

    /// Invalid expiry time: it cannot be earlier than the current time
    const EINVALID_EXPIRY_TIME: u64 = 1;
    /// Expiry time does not go beyond upper cap duration
    const EEXPIRY_TIME_UPPER: u64 = 2;
    /// Expiry time must be after the start of the next epoch
    const EEXPIRY_BEFORE_NEXT_EPOCH: u64 = 3;
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
    /// Auxiliary data during registration is not supported
    const ENO_AUX_DATA_SUPPORTED: u64 = 14;
    /// Supra native automation feature is not initialized or enabled
    const EDISABLED_AUTOMATION_FEATURE: u64 = 15;
    /// Insufficient balance in the resource wallet for withdrawal
    const EINSUFFICIENT_BALANCE: u64 = 16;
    /// Requested amount exceeds the locked balance
    const EREQUEST_EXCEEDS_LOCKED_BALANCE: u64 = 17;
    /// Current epoch interval is greater than specified task duration cap.
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

    /// The length of the transaction hash.
    const TXN_HASH_LENGTH: u64 = 32;
    /// Conversion factor between microseconds and second
    const MICROSECS_CONVERSION_FACTOR: u64 = 1_000_000;
    /// Registry resource creation seed
    const REGISTRY_RESOURCE_SEED: vector<u8> = b"supra_framework::automation_registry";
    /// Max U64 value
    const MAX_U64: u128 = 18446744073709551615;
    /// Decimal place to make
    const DECIMAL: u256 = 100_000_000; // 10^8 Power
    /// 100 Percentage
    const MAX_PERCENTAGE: u8 = 100;

    /// Constants describing task state.
    const PENDING: u8 = 0;
    const ACTIVE: u8 = 1;
    const CANCELLED: u8 = 2;

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    struct ActiveAutomationRegistryConfig has key {
        main_config: AutomationRegistryConfig,
        /// Will be the same as main_config.registry_max_gas_cap, unless updated during the epoch.
        next_epoch_registry_max_gas_cap: u64,
        /// Flag indicating whether the task registration is enabled or paused.
        /// If paused a new task registration will fail.
        registration_enabled: bool,
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    #[event]
    /// Automation registry configuration parameters
    struct AutomationRegistryConfig has key, store, drop, copy {
        /// Maximum allowable duration (in seconds) from the registration time that an automation task can run.
        /// If the expiration time exceeds this duration, the task registration will fail.
        task_duration_cap_in_secs: u64,
        /// Maximum gas allocation for automation tasks per epoch
        /// Exceeding this limit during task registration will cause failure and is used in fee calculation.
        registry_max_gas_cap: u64,
        /// Base fee per second for the full capacity of the automation registry, measured in quants/sec.
        /// The capacity is considered full if the total committed gas of all registered tasks equals registry_max_gas_cap.
        automation_base_fee_in_quants_per_sec: u64,
        /// Flat registration fee charged by default for each task.
        flat_registration_fee_in_quants: u64,
        /// Ratio (in the range [0;100]) representing the acceptable upper limit of committed gas amount
        /// relative to registry_max_gas_cap. Beyond this threshold, congestion fees apply.
        congestion_threshold_percentage: u8,
        /// Base fee per second for the full capacity of the automation registry when the congestion threshold is exceeded.
        congestion_base_fee_in_quants_per_sec: u64,
        /// The congestion fee increases exponentially based on this value, ensuring higher fees as the registry approaches full capacity.
        congestion_exponent: u8,
        /// Maximum number of tasks that registry can hold.
        task_capacity: u16,
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    /// It tracks entries both pending and completed, organized by unique indices.
    struct AutomationRegistry has key, store {
        /// A collection of automation task entries that are active state.
        tasks: EnumerableMap<u64, AutomationTaskMetaData>,
        /// Automation task index which increase
        current_index: u64,
        /// Gas committed for next epoch
        gas_committed_for_next_epoch: u64,
        /// Total fee charged to users during the epoch, which is not withdrawable
        epoch_locked_fees: u64,
        /// Total committed max gas amount at the beginning of the current epoch.
        gas_committed_for_this_epoch: u256,
        /// It's resource address which is use to deposit user automation fee
        registry_fee_address: address,
        /// Resource account signature capability
        registry_fee_address_signer_cap: SignerCapability,
        /// Cached active task indexes for the current epoch.
        epoch_active_task_ids: vector<u64>
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    /// Epoch state
    struct AutomationEpochInfo has key, copy {
        /// Epoch expected duration at the beginning of the new epoch, Based on this and actual
        /// epoch_duration which will be (current_time - last_reconfiguration_time) automation tasks
        /// refunds will be calculated.
        /// it will be updated upon each new epoch start with epoch_interval value.
        /// Although we should be careful with refunds if block production interval is quite high.
        expected_epoch_duration: u64,
        /// Epoch interval that can be updated any moment of the time
        epoch_interval: u64,
        /// Current epoch start time which is the same as last_reconfiguration_time
        start_time: u64,
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    #[event]
    /// `AutomationTaskMetaData` represents a single automation task item, containing metadata.
    struct AutomationTaskMetaData has key, copy, store, drop {
        /// Automation task index in registry
        task_index: u64,
        /// The address of the task owner.
        owner: address,
        /// The function signature associated with the registry entry.
        payload_tx: vector<u8>,
        /// Expiry of the task, represented in a timestamp in second.
        expiry_time: u64,
        /// The transaction hash of the request transaction.
        tx_hash: vector<u8>,
        /// Max gas amount of automation task
        max_gas_amount: u64,
        /// Maximum gas price cap for the task
        gas_price_cap: u64,
        /// Maximum automation fee for epoch to be paid ever.
        automation_fee_cap_for_epoch: u64,
        /// Auxiliary data specified for the task to aid registration.
        /// Not used currently. Reserved for future extensions.
        aux_data: vector<vector<u8>>,
        /// Registration timestamp in seconds
        registration_time: u64,
        /// Flag indicating whether the task is active, cancelled or pending.
        state: u8,
        /// Fee locked for the task estimated for the next epoch at the start of the current epoch.
        locked_fee_for_next_epoch: u64,
    }

    #[event]
    /// Event on task registration fee withdrawal from owner account upon registration.
    struct TaskRegistrationFeeWithdraw has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    #[event]
    /// Emitted on withdrawal of specified amount from automation registry fee address to the specified address.
    struct RegistryFeeWithdraw has drop, store {
        to: address,
        amount: u64
    }

    #[event]
    /// Event emitted when an automation fee is charged for an automation task for the epoch.
    struct TaskEpochFeeWithdraw has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    #[event]
    /// Event emitted when an automation fee is refunded for an automation task at the end of the epoch for excessive
    /// duration paid at the beginning of the epoch due to epoch-duration reduction by governance.
    struct TaskFeeRefund has drop, store {
        task_index: u64,
        owner: address,
        amount: u64,
    }

    #[event]
    /// Event emitted on automation task cancellation by owner.
    struct TaskCancelled has drop, store {
        task_index: u64,
        owner: address,
    }

    #[event]
    /// Event emitted when an automation task is cancelled due to insufficient balance.
    struct TaskCancelledInsufficentBalance has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    #[event]
    /// Event emitted when an automation task is cancelled due to automation fee capacity surpass.
    struct TaskCancelledCapacitySurpassed has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
        automation_fee_cap: u64,
    }

    #[event]
    /// Event emitted when on new epoch a task is accessed with index of the task for the expected list
    /// but value does not exist in the map
    struct ErrorTaskDoesNotExist has drop, store {
        task_index: u64,
    }

    #[event]
    /// Event emitted when on new epoch a task is accessed with index of the task automation fee withdrawal
    /// but it does not exist in the list.
    struct ErrorTaskDoesNotExistForWithdrawal has drop, store {
        task_index: u64,
    }

    #[event]
    /// Emitted when the registration in the automation registry is enabled.
    struct EnabledRegistrationEvent has drop, store {}

    #[event]
    /// Emitted when the registration in the automation registry is disabled.
    struct DisabledRegistrationEvent has drop, store {}

    /// Represents the fee charged for an automation task execution and some additional information.
    struct AutomationTaskFee has drop {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    /// Represents intermediate state of the registry on epoch change.
    struct IntermediateState has drop {
        active_task_ids: vector<u64>,
        gas_committed_for_next_epoch: u64,
        epoch_locked_fees: u64,
    }

    fun active_task_ids(intermediate_state: IntermediateState): vector<u64> {
        intermediate_state.active_task_ids
    }

    #[view]
    /// Checks whether all required resources are created.
    public fun is_initialized(): bool {
        exists<AutomationRegistry>(@supra_framework)
            && exists<AutomationEpochInfo>(@supra_framework)
            && exists<ActiveAutomationRegistryConfig>(@supra_framework)
    }

    #[view]
    /// Means to query by user whether the automation registry has been properly initialized and ready to be utilized.
    public fun is_feature_enabled_and_initialized(): bool {
        features::supra_native_automation_enabled() && is_initialized()
    }

    #[view]
    /// Returns next task index in registry
    public fun get_next_task_index(): u64 acquires AutomationRegistry {
        let automation_registry = borrow_global<AutomationRegistry>(@supra_framework);
        automation_registry.current_index
    }

    #[view]
    /// Returns number of available tasks.
    public fun get_task_count(): u64 acquires AutomationRegistry {
        let state = borrow_global<AutomationRegistry>(@supra_framework);
        enumerable_map::length(&state.tasks)
    }

    #[view]
    /// List all automation task ids available in register.
    public fun get_task_ids(): vector<u64> acquires AutomationRegistry {
        let state = borrow_global<AutomationRegistry>(@supra_framework);
        enumerable_map::get_map_list(&state.tasks)
    }

    #[view]
    /// Get locked balance of the resource account
    public fun get_epoch_locked_balance(): u64 acquires AutomationRegistry {
        let automation_registry = borrow_global<AutomationRegistry>(@supra_framework);
        automation_registry.epoch_locked_fees
    }

    #[view]
    /// List all active automation task ids for the current epoch.
    /// Note that the tasks with CANCELLED state are still considered active for the current epoch,
    /// as cancellation takes effect in the next epoch only.
    public fun get_active_task_ids(): vector<u64> acquires AutomationRegistry {
        let state = borrow_global<AutomationRegistry>(@supra_framework);
        state.epoch_active_task_ids
    }

    #[view]
    /// Retrieves the details of a automation task entry by its task index.
    /// Error will be returned if entry with specified task index does not exist.
    public fun get_task_details(task_index: u64): AutomationTaskMetaData acquires AutomationRegistry {
        let automation_task_metadata = borrow_global<AutomationRegistry>(@supra_framework);
        assert!(enumerable_map::contains(&automation_task_metadata.tasks, task_index), EAUTOMATION_TASK_NOT_FOUND);
        enumerable_map::get_value(&automation_task_metadata.tasks, task_index)
    }

    #[view]
    /// Checks whether there is an active task in registry with specified input task index.
    public fun has_sender_active_task_with_id(sender: address, task_index: u64): bool acquires AutomationRegistry {
        let automation_task_metadata = borrow_global<AutomationRegistry>(@supra_framework);
        if (enumerable_map::contains(&automation_task_metadata.tasks, task_index)) {
            let value = enumerable_map::get_value_ref(&automation_task_metadata.tasks, task_index);
            value.state != PENDING && value.owner == sender
        } else {
            false
        }
    }

    #[view]
    /// Get registry fee resource account address
    public fun get_registry_fee_address(): address {
        account::create_resource_address(&@supra_framework, REGISTRY_RESOURCE_SEED)
    }

    #[view]
    /// Get gas committed for next epoch
    public fun get_gas_committed_for_next_epoch(): u64 acquires AutomationRegistry {
        let automation_registry = borrow_global<AutomationRegistry>(@supra_framework);
        automation_registry.gas_committed_for_next_epoch
    }

    #[view]
    /// Get gas committed for the current epoch at the beginning of the epoch.
    public fun get_gas_committed_for_current_epoch(): u64 acquires AutomationRegistry {
        let automation_registry = borrow_global<AutomationRegistry>(@supra_framework);
        (automation_registry.gas_committed_for_this_epoch as u64)
    }

    #[view]
    /// Get automation registry configuration
    public fun get_automation_registry_config(): AutomationRegistryConfig acquires ActiveAutomationRegistryConfig {
        borrow_global<ActiveAutomationRegistryConfig>(@supra_framework).main_config
    }

    #[view]
    /// Get automation registry maximum gas capacity for the next epoch
    public fun get_next_epoch_registry_max_gas_cap(): u64 acquires ActiveAutomationRegistryConfig {
        borrow_global<ActiveAutomationRegistryConfig>(@supra_framework).next_epoch_registry_max_gas_cap
    }

    #[view]
    /// Get automation epoch info
    public fun get_automation_epoch_info(): AutomationEpochInfo acquires AutomationEpochInfo {
        *borrow_global<AutomationEpochInfo>(@supra_framework)
    }

    #[view]
    /// Estimates automation fee for the next epoch for specified task occupancy for the configured epoch-interval
    /// referencing the current automation registry fee parameters, current total occupancy and registry maximum allowed
    /// occupancy for the next epoch.
    public fun estimate_automation_fee(
        task_occupancy: u64
    ): u64 acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        let registry = borrow_global<AutomationRegistry>(@supra_framework);
        estimate_automation_fee_with_committed_occupancy(task_occupancy, registry.gas_committed_for_next_epoch)
    }

    #[view]
    /// Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
    /// referencing the current automation registry fee parameters, specified total/committed occupancy and registry
    /// maximum allowed occupancy for the next epoch.
    public fun estimate_automation_fee_with_committed_occupancy(
        task_occupancy: u64,
        committed_occupancy: u64
    ): u64 acquires AutomationEpochInfo, ActiveAutomationRegistryConfig {
        let epoch_info = borrow_global<AutomationEpochInfo>(@supra_framework);
        let config = borrow_global<ActiveAutomationRegistryConfig>(@supra_framework);
        estimate_automation_fee_with_committed_occupancy_internal(task_occupancy, committed_occupancy, epoch_info, config)
    }

    #[view]
    /// Returns the current status of the registration in the automation registry.
    public fun is_registration_enabled(): bool acquires ActiveAutomationRegistryConfig {
        borrow_global<ActiveAutomationRegistryConfig>(@supra_framework).registration_enabled
    }

    /// Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
    /// referencing the current automation registry fee parameters, specified total/committed occupancy and registry
    /// maximum allowed occupancy for the next epoch.
    fun estimate_automation_fee_with_committed_occupancy_internal(
        task_occupancy: u64,
        committed_occupancy: u64,
        epoch_info: &AutomationEpochInfo,
        active_config: &ActiveAutomationRegistryConfig
    ): u64 {
        let total_committed_max_gas = committed_occupancy + task_occupancy;

        let congestion_base_fee_per_sec = calculate_automation_congestion_fee(
            &active_config.main_config,
            (total_committed_max_gas as u256),
            active_config.next_epoch_registry_max_gas_cap);

        let automation_fee_per_sec = (active_config.main_config.automation_base_fee_in_quants_per_sec as u256) +
            congestion_base_fee_per_sec;

        if (automation_fee_per_sec == 0) {
            return 0
        };

        calculate_automation_fee_for_interval(
            epoch_info.epoch_interval,
            task_occupancy,
            automation_fee_per_sec,
            active_config.next_epoch_registry_max_gas_cap)
    }

    fun validate_configuration_parameters_common(
        epoch_interval_secs: u64,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        congestion_threshold_percentage: u8,
        congestion_exponent: u8,
    ){
        assert!(congestion_threshold_percentage <= MAX_PERCENTAGE, EMAX_CONGESTION_THRESHOLD);
        assert!(congestion_exponent > 0, ECONGESTION_EXP_NON_ZERO);
        assert!(task_duration_cap_in_secs > epoch_interval_secs, EUNACCEPTABLE_TASK_DURATION_CAP);
        assert!(registry_max_gas_cap > 0, EREGISTRY_MAX_GAS_CAP_NON_ZERO);
    }

    fun create_registry_resource_account(supra_framework: &signer): (signer, SignerCapability) {
        let (registry_fee_resource_signer, registry_fee_address_signer_cap) = account::create_resource_account(
            supra_framework,
            REGISTRY_RESOURCE_SEED
        );
        coin::register<SupraCoin>(&registry_fee_resource_signer);
        (registry_fee_resource_signer, registry_fee_address_signer_cap)
    }

    /// Initialization of Automation Registry with configuration parameters is expected metrics.
    public(friend) fun initialize(
        supra_framework: &signer,
        epoch_interval_secs: u64,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
    ) {
        system_addresses::assert_supra_framework(supra_framework);
        validate_configuration_parameters_common(
            epoch_interval_secs,
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            congestion_threshold_percentage,
            congestion_exponent);

        let (registry_fee_resource_signer, registry_fee_address_signer_cap) = create_registry_resource_account(supra_framework);

        move_to(supra_framework, AutomationRegistry {
            tasks: enumerable_map::new_map(),
            current_index: 0,
            gas_committed_for_next_epoch: 0,
            epoch_locked_fees: 0,
            gas_committed_for_this_epoch: 0,
            registry_fee_address: signer::address_of(&registry_fee_resource_signer),
            registry_fee_address_signer_cap,
            epoch_active_task_ids: vector[],
        });

        move_to(supra_framework, ActiveAutomationRegistryConfig {
            main_config: AutomationRegistryConfig {
                task_duration_cap_in_secs,
                registry_max_gas_cap,
                automation_base_fee_in_quants_per_sec,
                flat_registration_fee_in_quants,
                congestion_threshold_percentage,
                congestion_base_fee_in_quants_per_sec,
                congestion_exponent,
                task_capacity,
            },
            next_epoch_registry_max_gas_cap: registry_max_gas_cap,
            registration_enabled: true,
        });

        move_to(supra_framework, AutomationEpochInfo {
            expected_epoch_duration: epoch_interval_secs,
            epoch_interval: epoch_interval_secs,
            start_time: 0,
        });
    }


    /// On new epoch this function will be triggered and update the automation registry state
    public(friend) fun on_new_epoch() acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        // Unless registry in initialized, registry will not be updated on new epoch.
        // Here we need to be careful as well. If the feature is disabled for the current epoch then
        //  - refund for the previous epoch should be done if any charges has been done.
        //  - all tasks should be removed from registry state
        // Note that with the current setup feature::on_new_epoch is called before automation_registry::on_new_epoch
        if (!is_initialized()) {
            return
        };
        let automation_registry = borrow_global_mut<AutomationRegistry>(@supra_framework);
        let automation_epoch_info = borrow_global_mut<AutomationEpochInfo>(@supra_framework);

        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfig>(
            @supra_framework
        ).main_config;

        let current_time = timestamp::now_seconds();

        // Refund the task if the epoch was shorter than expected.
        adjust_tasks_epoch_fee_refund(
            automation_registry,
            &automation_registry_config,
            automation_epoch_info,
            current_time
        );

        // Apply the latest configuration if any parameter has been updated
        // only after refund has been done for previous epoch.
        update_config_from_buffer();

        // If feature is not enabled then we are not charging and tasks are cleared.
        if (!features::supra_native_automation_enabled()) {

            automation_registry.gas_committed_for_next_epoch = 0;
            automation_registry.epoch_locked_fees = 0;
            automation_registry.gas_committed_for_this_epoch = 0;
            automation_registry.epoch_active_task_ids = vector[];
            enumerable_map::clear(&mut automation_registry.tasks);

            automation_epoch_info.start_time = current_time;
            automation_epoch_info.expected_epoch_duration = automation_epoch_info.epoch_interval;
            return
        };

        // Accumulated maximum gas amount of the registered tasks for the current epoch
        let tcmg = cleanup_and_activate_tasks(automation_registry, current_time);


        let tasks_automation_fees = calculate_tasks_automation_fees(
            automation_registry,
            &automation_registry_config,
            automation_epoch_info.epoch_interval,
            current_time,
            tcmg,
            false
        );

        let intermediate_state = try_withdraw_task_automation_fees(
            automation_registry,
            tasks_automation_fees,
            current_time,
            automation_epoch_info.epoch_interval
        );

        automation_registry.gas_committed_for_next_epoch = intermediate_state.gas_committed_for_next_epoch;
        automation_registry.epoch_locked_fees = intermediate_state.epoch_locked_fees;
        automation_registry.gas_committed_for_this_epoch = tcmg;
        automation_registry.epoch_active_task_ids = active_task_ids(intermediate_state);

        automation_epoch_info.start_time = current_time;
        automation_epoch_info.expected_epoch_duration = automation_epoch_info.epoch_interval;
    }

    /// Adjusts task fees and processes refunds when there's a change in epoch duration.
    fun adjust_tasks_epoch_fee_refund(
        automation_registry: &AutomationRegistry,
        arc: &AutomationRegistryConfig,
        aei: &AutomationEpochInfo,
        current_time: u64
    ) {
        // If no funds were locked for the previous epoch then there is nothing to refund.
        // This may happen when feature was disabled, and no automation task was registered and charged for the next epoch.
        if (automation_registry.epoch_locked_fees == 0) {
            return
        };

        // If epoch actual duration is greater or equal to expected epoch-duration then there is nothing to refund.
        let epoch_duration = current_time - aei.start_time;
        if (aei.expected_epoch_duration <= epoch_duration) {
            return
        };

        let residual_time = aei.expected_epoch_duration - epoch_duration;
        let tcmg = automation_registry.gas_committed_for_this_epoch;
        let registry_fee_address_signer_cap = &automation_registry.registry_fee_address_signer_cap;
        let tasks_automation_refund_fees = calculate_tasks_automation_fees(
            automation_registry,
            arc,
            residual_time,
            current_time,
            tcmg,
            true
        );
        refund_tasks_fee(registry_fee_address_signer_cap, tasks_automation_refund_fees);
    }

    /// Processes refunds for automation task fees.
    fun refund_tasks_fee(
        resource_signer_cap: &SignerCapability,
        tasks_automation_refund_fees: vector<AutomationTaskFee>
    ) {
        let resource_signer = account::create_signer_with_capability(resource_signer_cap);

        vector::for_each(tasks_automation_refund_fees, |task| {
            let task: AutomationTaskFee = task;
            if (task.fee != 0) {
                coin::transfer<SupraCoin>(&resource_signer, task.owner, task.fee);
                event::emit(TaskFeeRefund { task_index: task.task_index, owner: task.owner, amount: task.fee });
            }
        });
    }

    /// Cleanup and activate the automation task also it's calculate and return total committed max gas
    fun cleanup_and_activate_tasks(automation_registry: &mut AutomationRegistry, current_time: u64): u256 {
        let ids = enumerable_map::get_map_list(&automation_registry.tasks);
        let tcmg = 0;

        // Perform clean up and updation of state (we can't use enumerable_map::for_each, as actually we need value as mutable ref)
        vector::for_each(ids, |task_index| {
            if (!enumerable_map::contains(&automation_registry.tasks, task_index)) {
                event::emit(ErrorTaskDoesNotExist { task_index })
            } else {
                let task = enumerable_map::get_value_mut(&mut automation_registry.tasks, task_index);

                // Drop or activate task for this current epoch.
                if (task.expiry_time <= current_time || task.state == CANCELLED) {
                    enumerable_map::remove_value(&mut automation_registry.tasks, task_index);
                } else {
                    task.state = ACTIVE;
                    tcmg = tcmg + (task.max_gas_amount as u256);
                }
            }
        });
        tcmg
    }

    /// Calculates automation task fees for the active tasks for the provided interval with provided tcmg occupancy.
    /// The CANCELLED tasks are also taken into account if include_cancelled_task is true.
    fun calculate_tasks_automation_fees(
        automation_registry: &AutomationRegistry,
        arc: &AutomationRegistryConfig,
        interval: u64,
        current_time: u64,
        tcmg: u256,
        include_cancelled_task: bool
    ): vector<AutomationTaskFee> {
        // Compute the automation congestion fee (acf) for the epoch
        let acf = calculate_automation_congestion_fee(arc, tcmg, arc.registry_max_gas_cap);
        let task_with_fees = vector[];
        // Automation fee per second is the sum of the automation base fee per second and congeation fee per second
        // calculated based on the current registry occupancy.
        let automation_fee_per_sec = acf + (arc.automation_base_fee_in_quants_per_sec as u256);

        // Return early if automation fee per second is 0
        if (automation_fee_per_sec == 0) {
            return task_with_fees
        };

        // Process each active task and calculate fee for the epoch for the tasks
        enumerable_map::for_each_value_ref(&automation_registry.tasks, |task| {
            let task: &AutomationTaskMetaData = task;
            if (task.state == ACTIVE || (include_cancelled_task && task.state == CANCELLED)) {
                let task_fee = calculate_task_fee(arc, task, interval, current_time, automation_fee_per_sec);
                vector::push_back(&mut task_with_fees, AutomationTaskFee {
                    task_index: task.task_index,
                    owner: task.owner,
                    fee: task_fee,
                });
            }
        });
        task_with_fees
    }

    /// Calculates automation task fees for a single task at the time of new epoch.
    /// This is supposed to be called only after removing expired task and must not be called for expired task.
    /// It returns calculated task fee for the interval the task will be active.
    fun calculate_task_fee(
        arc: &AutomationRegistryConfig,
        task: &AutomationTaskMetaData,
        interval: u64,
        current_time: u64,
        automation_fee_per_sec: u256
    ): u64 {
        if (task.expiry_time <= current_time) { return 0 };
        // Subtraction is safe here, as we already excluded expired tasks
        let remaining_time = task.expiry_time - current_time;
        let min_interval = math64::min(remaining_time, interval);
        calculate_automation_fee_for_interval(
            min_interval,
            task.max_gas_amount,
            automation_fee_per_sec,
            arc.registry_max_gas_cap)
    }

    /// Calculates automation task fees for a single task at the time of new epoch.
    /// This is supposed to be called only after removing expired task and must not be called for expired task.
    fun calculate_automation_fee_for_interval(
        interval: u64,
        task_occupancy: u64,
        automation_fee_per_sec: u256,
        registry_max_gas_cap: u64,
    ): u64 {
        let max_gas_cap = (registry_max_gas_cap as u256);
        let duration = (interval as u256);
        let task_occupancy_ratio_by_duration = (duration * upscale_from_u64(task_occupancy)) / max_gas_cap;

        let automation_fee_for_interval = automation_fee_per_sec * task_occupancy_ratio_by_duration;

        downscale_to_u64(automation_fee_for_interval)
    }

    /// Calculate automation congestion fee for the epoch
    fun calculate_automation_congestion_fee(
        arc: &AutomationRegistryConfig,
        tcmg: u256,
        registry_max_gas_cap: u64
    ): u256 {
        if (arc.congestion_threshold_percentage == MAX_PERCENTAGE || arc.congestion_base_fee_in_quants_per_sec == 0) {
            return 0
        };

        let max_gas_cap = (registry_max_gas_cap as u256);
        let threshold_percentage = upscale_from_u8(arc.congestion_threshold_percentage);

        // Calculate congestion threshold surplus for the current epoch
        let threshold_usage = upscale_from_u256(tcmg) * 100 / max_gas_cap;
        if (threshold_usage < threshold_percentage) 0
        else {
            let threshold_surplus_normalized = (threshold_usage - threshold_percentage) / 100;

            // Ensure threshold + threshold_surplus does not exceeds 1 (1 in scaled terms)
            let threshold_percentage_scaled = threshold_percentage / 100;
            let threshold_surplus_clip = if ((threshold_surplus_normalized + threshold_percentage_scaled) > DECIMAL) {
                DECIMAL - threshold_percentage_scaled
            } else {
                threshold_surplus_normalized
            };
            // Compute the automation congestion fee (acf) for the epoch
            let threshold_surplus_exponential = calculate_exponentiation(
                threshold_surplus_clip,
                arc.congestion_exponent
            );

            // Calculate acf by multiplying base fee with exponential result
            let acf = (arc.congestion_base_fee_in_quants_per_sec as u256) * threshold_surplus_exponential;
            downscale_to_u256(acf)
        }
    }

    /// Calculates (1 + base)^exponent, where `base` is represented with `DECIMAL` decimal places.
    /// For example, if `base` is 0.5, it should be passed as 0.5 * DECIMAL (i.e., 50000000).
    /// The result is returned as an integer with `DECIMAL` decimal places.
    /// It will return the result of (((1 + base)^exponent) - 1), scaled by `DECIMAL` (e.g., 103906250 for 1.0390625).
    /// The reason for using `(1 + base)^exponent` is that `base` would be the fraction by which the congestion threshold is crossed,
    ///     thus highly likely to be less than one. To ensure that as `exponent` increases, the function increases, `1` is added.
    ///     In the final result, after `(1 + base)^exponent` is calculated, `1` is subtracted so as not to subsume the automation
    ///     base fee in this component. This would allow the freedom to set a multiplier for the automation base fee separately
    ///     from the congestion fee.
    /// `exponent` here acts as the degree of the polynomial, therefore an `exponent` of `2` or higher
    ///     would allow the congestion fee to increase in a non-linear fashion.
    fun calculate_exponentiation(base: u256, exponent: u8): u256 {
        // Add 1 (represented as DECIMAL) to the base
        let one_scaled = DECIMAL; // 1.0 in DECIMAL representation
        let adjusted_base = base + one_scaled; // (1 + base) in DECIMAL representation

        // Initialize result as 1 (represented in DECIMAL)
        let result = one_scaled;

        // Perform exponential calculation using integer arithmetic
        let i = 0;
        while (i < exponent) {
            result = result * adjusted_base / DECIMAL; // Adjust for decimal places
            i = i + 1;
        };

        // Subtract the initial added 1 (DECIMAL) to get the final result
        result - one_scaled
    }

    /// Processes automation task fees by checking user balances and task's commitment on automation-fee, i.e. automation-fee-cap
    /// - If the user has sufficient balance, deducts the fee and emits a success event.
    /// - If the balance is insufficient, removes the task and emits a cancellation event.
    /// - If calculated fee for the epoch surpasses task's automation-fee-cap task is removed and cancellation event is emitted.
    /// Return estimated committed gas for the next epoch, locked automation fee amount for this epoch, and list of active task indexes
    fun try_withdraw_task_automation_fees(
        automation_registry: &mut AutomationRegistry,
        tasks_automation_fees: vector<AutomationTaskFee>,
        current_time: u64,
        epoch_interval: u64,
    ): IntermediateState {
        let intermediate_state = IntermediateState {
            gas_committed_for_next_epoch: 0,
            epoch_locked_fees: 0,
            active_task_ids: vector[]
        };

        sort_by_task_index(&mut tasks_automation_fees);

        vector::for_each(tasks_automation_fees, |task| {
            let task: AutomationTaskFee = task;
            if (!enumerable_map::contains(&automation_registry.tasks, task.task_index)) {
                event::emit(ErrorTaskDoesNotExistForWithdrawal {task_index: task.task_index})
            } else {
                try_withdraw_task_automation_fee(automation_registry, task, current_time, epoch_interval, &mut intermediate_state);
            };
        });
        intermediate_state
    }

    fun try_withdraw_task_automation_fee(
        automation_registry: &mut AutomationRegistry,
        task: AutomationTaskFee,
        current_time: u64,
        epoch_interval: u64,
        intermediate_state: &mut IntermediateState) {

        let task_metadata = enumerable_map::get_value(&automation_registry.tasks, task.task_index);

        // Remove the automation task if the epoch fee cap is exceeded
        if (task.fee > task_metadata.automation_fee_cap_for_epoch) {
            enumerable_map::remove_value(&mut automation_registry.tasks, task.task_index);
            event::emit(TaskCancelledCapacitySurpassed {
                task_index: task.task_index,
                owner: task_metadata.owner,
                fee: task.fee,
                automation_fee_cap: task_metadata.automation_fee_cap_for_epoch,
            });
        } else {
            let user_balance = coin::balance<SupraCoin>(task_metadata.owner);
            if (user_balance < task.fee) {
                // If the user does not have enough balance, remove the task and emit an event
                enumerable_map::remove_value(&mut automation_registry.tasks, task.task_index);
                event::emit(TaskCancelledInsufficentBalance {
                    task_index: task.task_index,
                    owner: task_metadata.owner,
                    fee: task.fee,
                });
            } else {
                // Charge the fee and emit a success event
                coin::transfer<SupraCoin>(
                    &create_signer(task_metadata.owner),
                    automation_registry.registry_fee_address,
                    task.fee
                );
                event::emit(TaskEpochFeeWithdraw {
                    task_index: task.task_index,
                    owner: task_metadata.owner,
                    fee: task.fee,
                });
                // Total task fees deducted from the user's account
                intermediate_state.epoch_locked_fees = intermediate_state.epoch_locked_fees + task.fee;
                vector::push_back(&mut intermediate_state.active_task_ids, task.task_index);

                // Calculate gas commitment for the next epoch only for valid active tasks
                if (task_metadata.expiry_time > (current_time + epoch_interval)) {
                    intermediate_state.gas_committed_for_next_epoch = intermediate_state.gas_committed_for_next_epoch + task_metadata.max_gas_amount;
                };
            };
        }
    }

    /// The function updates the ActiveAutomationRegistryConfig structure with values extracted from the buffer, if the buffer exists.
    fun update_config_from_buffer() acquires ActiveAutomationRegistryConfig {
        if (config_buffer::does_exist<AutomationRegistryConfig>()) {
            let buffer = config_buffer::extract<AutomationRegistryConfig>();
            let automation_registry_config = &mut borrow_global_mut<ActiveAutomationRegistryConfig>(
                @supra_framework
            ).main_config;
            automation_registry_config.task_duration_cap_in_secs = buffer.task_duration_cap_in_secs;
            automation_registry_config.registry_max_gas_cap = buffer.registry_max_gas_cap;
            automation_registry_config.automation_base_fee_in_quants_per_sec = buffer.automation_base_fee_in_quants_per_sec;
            automation_registry_config.flat_registration_fee_in_quants = buffer.flat_registration_fee_in_quants;
            automation_registry_config.congestion_threshold_percentage = buffer.congestion_threshold_percentage;
            automation_registry_config.congestion_base_fee_in_quants_per_sec = buffer.congestion_base_fee_in_quants_per_sec;
            automation_registry_config.congestion_exponent = buffer.congestion_exponent;
            automation_registry_config.task_capacity = buffer.task_capacity;
        };
    }

    /// Withdraw accumulated automation task fees from the resource account - access by admin
    public fun withdraw_automation_task_fees(
        supra_framework: &signer,
        to: address,
        amount: u64
    ) acquires AutomationRegistry {
        system_addresses::assert_supra_framework(supra_framework);
        transfer_fee_to_account_internal(to, amount);
        event::emit(RegistryFeeWithdraw { to, amount });
    }

    /// Transfers the specified fee amount from the resource account to the target account.
    fun transfer_fee_to_account_internal(to: address, amount: u64) acquires AutomationRegistry {
        let automation_registry = borrow_global_mut<AutomationRegistry>(@supra_framework);
        let resource_balance = coin::balance<SupraCoin>(automation_registry.registry_fee_address);

        assert!(resource_balance >= amount, EINSUFFICIENT_BALANCE);

        assert!((resource_balance - amount) >= automation_registry.epoch_locked_fees, EREQUEST_EXCEEDS_LOCKED_BALANCE);

        let resource_signer = account::create_signer_with_capability(
            &automation_registry.registry_fee_address_signer_cap
        );
        coin::transfer<SupraCoin>(&resource_signer, to, amount);
    }

    /// Update Automation Registry Config
    public fun update_config(
        supra_framework: &signer,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
    ) acquires AutomationRegistry, ActiveAutomationRegistryConfig, AutomationEpochInfo {
        system_addresses::assert_supra_framework(supra_framework);

        let automation_registry = borrow_global<AutomationRegistry>(@supra_framework);
        let automation_epoch_info = borrow_global<AutomationEpochInfo>(@supra_framework);

        validate_configuration_parameters_common(
            automation_epoch_info.epoch_interval,
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            congestion_threshold_percentage,
            congestion_exponent);

        assert!(
            automation_registry.gas_committed_for_next_epoch < registry_max_gas_cap,
            EUNACCEPTABLE_AUTOMATION_GAS_LIMIT
        );

        let new_automation_registry_config = AutomationRegistryConfig {
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            automation_base_fee_in_quants_per_sec,
            flat_registration_fee_in_quants,
            congestion_threshold_percentage,
            congestion_base_fee_in_quants_per_sec,
            congestion_exponent,
            task_capacity
        };
        config_buffer::upsert(copy new_automation_registry_config);

        // next_epoch_registry_max_gas_cap will be update instantly
        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfig>(@supra_framework);
        automation_registry_config.next_epoch_registry_max_gas_cap = registry_max_gas_cap;

        event::emit(new_automation_registry_config);
    }

    /// Enables the registration process in the automation registry.
    public fun enable_registration(supra_framework: &signer) acquires ActiveAutomationRegistryConfig {
        system_addresses::assert_supra_framework(supra_framework);
        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfig>(@supra_framework);
        automation_registry_config.registration_enabled = true;
        event::emit(EnabledRegistrationEvent {});
    }

    /// Disables the registration process in the automation registry.
    public fun disable_registration(supra_framework: &signer) acquires ActiveAutomationRegistryConfig {
        system_addresses::assert_supra_framework(supra_framework);
        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfig>(@supra_framework);
        automation_registry_config.registration_enabled = false;
        event::emit(DisabledRegistrationEvent {});
    }

    /// Registers a new automation task entry.
    fun register(
        owner_signer: &signer,
        payload_tx: vector<u8>,
        expiry_time: u64,
        max_gas_amount: u64,
        gas_price_cap: u64,
        automation_fee_cap_for_epoch: u64,
        tx_hash: vector<u8>,
        aux_data: vector<vector<u8>>
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        // Guarding registration if feature is not enabled.
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        assert!(vector::is_empty(&aux_data), ENO_AUX_DATA_SUPPORTED);

        let automation_registry_config = borrow_global<ActiveAutomationRegistryConfig>(@supra_framework);
        assert!(automation_registry_config.registration_enabled, ETASK_REGISTRATION_DISABLED);

        // If registry is full, reject task registration
        assert!((get_task_count() as u16) < automation_registry_config.main_config.task_capacity, EREGISTRY_IS_FULL);

        let owner = signer::address_of(owner_signer);
        let automation_registry = borrow_global_mut<AutomationRegistry>(@supra_framework);
        let automation_epoch_info = borrow_global<AutomationEpochInfo>(@supra_framework);

        //Well-formedness check of payload_tx is done in native layer beforehand.

        let registration_time = timestamp::now_seconds();
        check_registration_task_duration(
            expiry_time,
            registration_time,
            &automation_registry_config.main_config,
            automation_epoch_info
        );

        assert!(gas_price_cap > 0, EINVALID_GAS_PRICE);
        assert!(max_gas_amount > 0, EINVALID_MAX_GAS_AMOUNT);
        assert!(vector::length(&tx_hash) == TXN_HASH_LENGTH, EINVALID_TXN_HASH);

        let committed_gas = (automation_registry.gas_committed_for_next_epoch as u128) + (max_gas_amount as u128);
        assert!(committed_gas <= MAX_U64, EGAS_COMMITTEED_VALUE_OVERFLOW);

        let committed_gas = (committed_gas as u64);
        assert!(committed_gas <= automation_registry_config.next_epoch_registry_max_gas_cap, EGAS_AMOUNT_UPPER);

        // Check the automation fee capacity
        let estimated_automation_fee_for_epoch = estimate_automation_fee_with_committed_occupancy_internal(
            max_gas_amount,
            committed_gas,
            automation_epoch_info,
            automation_registry_config);
        assert!(automation_fee_cap_for_epoch >= estimated_automation_fee_for_epoch,
            EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH
        );

        automation_registry.gas_committed_for_next_epoch = committed_gas;
        let task_index = automation_registry.current_index;

        let automation_task_metadata = AutomationTaskMetaData {
            task_index,
            owner,
            payload_tx,
            expiry_time,
            max_gas_amount,
            gas_price_cap,
            automation_fee_cap_for_epoch,
            aux_data,
            state: PENDING,
            registration_time,
            tx_hash,
            locked_fee_for_next_epoch: 0
        };

        enumerable_map::add_value(&mut automation_registry.tasks, task_index, automation_task_metadata);
        automation_registry.current_index = automation_registry.current_index + 1;

        // Charge flat registration fee from the user at the time of registration
        let fee = automation_registry_config.main_config.flat_registration_fee_in_quants;
        coin::transfer<SupraCoin>(owner_signer, automation_registry.registry_fee_address, fee);

        event::emit(TaskRegistrationFeeWithdraw { task_index, owner, fee });
        event::emit(automation_task_metadata);
    }

    fun check_registration_task_duration(
        expiry_time: u64,
        registration_time: u64,
        automation_registry_config: &AutomationRegistryConfig,
        automation_epoch_info: &AutomationEpochInfo
    ) {
        assert!(expiry_time > registration_time, EINVALID_EXPIRY_TIME);
        let task_duration = expiry_time - registration_time;
        assert!(task_duration <= automation_registry_config.task_duration_cap_in_secs, EEXPIRY_TIME_UPPER);

        // Check that task is valid at least in the next epoch
        assert!(
            expiry_time > (automation_epoch_info.start_time + automation_epoch_info.epoch_interval),
            EEXPIRY_BEFORE_NEXT_EPOCH
        );
    }

    /// Cancel Automation task with specified task_index.
    /// Only existing task, which is PENDING or ACTIVE, can be cancelled and only by task owner.
    /// If the task is
    ///   - active, its state is updated to be CANCELLED.
    ///   - pending, it is removed form the list.
    ///   - cancelled, an error is reported
    /// Committed gas-limit is updated by reducing it with the max-gas-amount of the cancelled task.
    public entry fun cancel_task(owner_signer: &signer, task_index: u64) acquires AutomationRegistry {
        let automation_registry = borrow_global_mut<AutomationRegistry>(@supra_framework);
        assert!(enumerable_map::contains(&automation_registry.tasks, task_index), EAUTOMATION_TASK_NOT_FOUND);

        let automation_task_metadata = enumerable_map::get_value(&mut automation_registry.tasks, task_index);
        let owner = signer::address_of(owner_signer);
        assert!(automation_task_metadata.owner == owner, EUNAUTHORIZED_TASK_OWNER);
        assert!(automation_task_metadata.state != CANCELLED, EALREADY_CANCELLED);
        if (automation_task_metadata.state == PENDING) {
            enumerable_map::remove_value(&mut automation_registry.tasks, task_index);
        } else if (automation_task_metadata.state == ACTIVE) {
            let automation_task_metadata_mut = enumerable_map::get_value_mut(
                &mut automation_registry.tasks,
                task_index
            );
            automation_task_metadata_mut.state = CANCELLED;
        };

        assert!(
            automation_registry.gas_committed_for_next_epoch >= automation_task_metadata.max_gas_amount,
            EGAS_COMMITTEED_VALUE_UNDERFLOW
        );
        // Adjust the gas committed for the next epoch by subtracting the gas amount of the cancelled task
        automation_registry.gas_committed_for_next_epoch = automation_registry.gas_committed_for_next_epoch - automation_task_metadata.max_gas_amount;

        event::emit(TaskCancelled { task_index: automation_task_metadata.task_index, owner });
    }

    /// Update epoch interval in registry while actually update happens in block module
    public(friend) fun update_epoch_interval_in_registry(epoch_interval_microsecs: u64) acquires AutomationEpochInfo {
        if (exists<AutomationEpochInfo>(@supra_framework)) {
            let automation_epoch_info = borrow_global_mut<AutomationEpochInfo>(@supra_framework);
            automation_epoch_info.epoch_interval = epoch_interval_microsecs / MICROSECS_CONVERSION_FACTOR;
        };
    }

    /// Sorting vector implementation
    fun sort_by_task_index(v: &mut vector<AutomationTaskFee>) {
        let len = vector::length(v);
        let i = 0;
        while (i < len) {
            let j = i + 1;
            while (j < len) {
                if (vector::borrow(v, i).task_index > vector::borrow(v, j).task_index) {
                    vector::swap(v, i, j)
                };
                j = j + 1;
            };
            i = i + 1;
        };
    }

    fun upscale_from_u8(value: u8): u256 { (value as u256) * DECIMAL }

    fun upscale_from_u64(value: u64): u256 { (value as u256) * DECIMAL }

    fun upscale_from_u256(value: u256): u256 { value * DECIMAL }

    fun downscale_to_u64(value: u256): u64 { ((value / DECIMAL) as u64) }

    fun downscale_to_u256(value: u256): u256 { value / DECIMAL }

    #[test_only]
    const AUTOMATION_MAX_GAS_TEST: u64 = 100_000_000;
    #[test_only]
    const TTL_UPPER_BOUND_TEST: u64 = 2_626_560;
    #[test_only]
    const AUTOMATION_BASE_FEE_TEST: u64 = 1000;
    #[test_only]
    const FLAT_REGISTRATION_FEE_TEST: u64 = 1_000_000;
    #[test_only]
    const CONGESTION_THRESHOLD_TEST: u8 = 80;
    #[test_only]
    const CONGESTION_BASE_FEE_TEST: u64 = 100;
    #[test_only]
    const CONGESTION_EXPONENT_TEST: u8 = 6;
    #[test_only]
    const TASK_CAPACITY_TEST: u16 = 50;
    #[test_only]
    /// Value defined in microsecond
    const EPOCH_INTERVAL_FOR_TEST_IN_SECS: u64 = 7200;
    #[test_only]
    const PARENT_HASH: vector<u8> = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    #[test_only]
    const PAYLOAD: vector<u8> = x"0102030405060708090a0b0c0d0e0f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20101112131415161718191a1b1c1d1e1f20";
    #[test_only]
    const AUX_DATA: vector<vector<u8>> = vector[];
    #[test_only]
    const ACCOUNT_BALANCE: u64 = 10_000_000_000;
    #[test_only]
    const REGISTRY_DEFAULT_BALANCE: u64 = 100_000_000_000;


    #[test_only]
    /// Initializes registry without enabling SUPRA_NATIVE_AUTOMATION feature flag
    fun initialize_registry_test_partially(supra_framework: &signer, user: &signer) {
        use supra_framework::coin;
        use supra_framework::supra_coin::{Self, SupraCoin};

        let user_addr = signer::address_of(user);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(@supra_framework);

        let (burn_cap, mint_cap) = supra_coin::initialize_for_test(supra_framework);

        initialize(
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
        );

        coin::register<SupraCoin>(user);
        supra_coin::mint(supra_framework, user_addr, ACCOUNT_BALANCE);
        supra_coin::mint(supra_framework, get_registry_fee_address(), REGISTRY_DEFAULT_BALANCE);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);

        timestamp::set_time_has_started_for_testing(supra_framework);
    }

    #[test_only]
    fun toggle_feature_flag(supra_framework: &signer, enable: bool) {
        let flag = vector[features::get_supra_native_automation_feature()];
        if (enable) {
            features::change_feature_flags_for_testing(supra_framework,
                flag,
                vector::empty<u64>());
        } else {
            features::change_feature_flags_for_testing(supra_framework,
                vector::empty<u64>(),
                flag)
        }
    }

    #[test_only]
    public fun update_config_for_tests(
        supra_framework: &signer,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
    ) acquires ActiveAutomationRegistryConfig {
        system_addresses::assert_supra_framework(supra_framework);

        let new_automation_registry_config = AutomationRegistryConfig {
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            automation_base_fee_in_quants_per_sec,
            flat_registration_fee_in_quants,
            congestion_threshold_percentage,
            congestion_base_fee_in_quants_per_sec,
            congestion_exponent,
            task_capacity
        };

        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfig>(@supra_framework);
        automation_registry_config.main_config = new_automation_registry_config;
        automation_registry_config.next_epoch_registry_max_gas_cap = registry_max_gas_cap;

        event::emit(new_automation_registry_config);
    }

    #[test_only]
    /// Initializes registry and enables SUPRA_NATIVE_AUTOMATION feature flag
    fun initialize_registry_test(supra_framework: &signer, user: &signer) {
        initialize_registry_test_partially(supra_framework, user);
        toggle_feature_flag(supra_framework, true);
    }


    #[test_only]
    fun has_task_with_id(task_index: u64): bool acquires AutomationRegistry {
        let automation_registry = borrow_global<AutomationRegistry>(@supra_framework);
        enumerable_map::contains(&automation_registry.tasks, task_index)
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    fun register_with_state(
        framework: &signer,
        user: &signer,
        max_gas_amount: u64,
        automation_fee_cap: u64,
        expiry_time: u64,
        state: u8,
    ): u64 acquires AutomationRegistry, ActiveAutomationRegistryConfig, AutomationEpochInfo {
        register(user,
            PAYLOAD,
            expiry_time,
            max_gas_amount,
            20,
            automation_fee_cap,
            PARENT_HASH,
            AUX_DATA
        );
        let automation_registry = borrow_global_mut<AutomationRegistry>(address_of(framework));
        let task_index = automation_registry.current_index - 1;
        let task_details = enumerable_map::get_value_mut(&mut automation_registry.tasks, task_index);
        if (state != PENDING) {
            automation_registry.gas_committed_for_this_epoch = automation_registry.gas_committed_for_this_epoch + (max_gas_amount as u256);
        };
        task_details.state = state;
        task_index
    }

    #[test_only]
    fun set_locked_fee(
        framework: &signer,
        locked_fee: u64,
    ) acquires AutomationRegistry {
        let automation_registry = borrow_global_mut<AutomationRegistry>(address_of(framework));
        automation_registry.epoch_locked_fees = locked_fee;
    }

    #[test_only]
    fun check_account_balance(
        account: address,
        expected_balance: u64,
    ) {
        let current_balance = coin::balance<SupraCoin>(account);
        assert!(current_balance == expected_balance, current_balance);
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = EUNACCEPTABLE_TASK_DURATION_CAP, location = Self)]
    fun test_initialization_with_invalid_task_duration(
        supra_framework: &signer,
    )  {
        initialize(
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
        );
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = EREGISTRY_MAX_GAS_CAP_NON_ZERO, location = Self)]
    fun test_initialization_with_invalid_registry_max_gas_cap(
        supra_framework: &signer,
    )  {
        initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            0,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST
        );
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = ECONGESTION_EXP_NON_ZERO, location = Self)]
    fun test_initialization_with_invalid_congestion_exponent(
        supra_framework: &signer,
    )  {
        initialize(
            supra_framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            0,
            TASK_CAPACITY_TEST
        );
    }

    #[test(supra_framework = @supra_framework)]
    #[expected_failure(abort_code = EMAX_CONGESTION_THRESHOLD, location = Self)]
    fun test_initialization_with_invalid_threshold_percentage(
        supra_framework: &signer,
    )  {
        initialize(
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
        );
    }

    #[test(supra_framework = @supra_framework, user = @0x1cafe)]
    fun test_registry(
        supra_framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(supra_framework, user);

        let payload = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132";
        let parent_hash = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        register(user, payload, 86400, 1000, 100000, 100_000_00, parent_hash, AUX_DATA);
    }

    #[test]
    fun test_on_new_epoch_without_initialization(
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        // Nothing will be attempted if the registry is not initialized.
        on_new_epoch()
    }

    #[test(supra_framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EDISABLED_AUTOMATION_FEATURE, location = Self)]
    fun test_registration_with_partial_initialization(
        supra_framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test_partially(supra_framework, user);

        let payload = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132";
        let parent_hash = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        register(user, payload, 86400, 1000, 100000, 100_000_00, parent_hash, AUX_DATA);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_update_config_success_update(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            50,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        config_buffer::initialize(framework);
        // Next epoch gas committed gas is less than the new limit value.
        // Configuration parameter will update after on new epoch
        update_config(framework,
            1_626_560,
            75,
            1005,
            700000000,
            70,
            2000,
            5,
            200);

        let state = borrow_global<ActiveAutomationRegistryConfig>(@supra_framework);
        assert!(state.main_config.registry_max_gas_cap == AUTOMATION_MAX_GAS_TEST, 1);
        assert!(state.next_epoch_registry_max_gas_cap == 75, 1);

        // Automation gas limit
        on_new_epoch();
        let state = borrow_global<ActiveAutomationRegistryConfig>(@supra_framework).main_config;
        assert!(state.registry_max_gas_cap == 75, 2);
        assert!(state.task_duration_cap_in_secs == 1_626_560, 3);
        assert!(state.automation_base_fee_in_quants_per_sec == 1005, 4);
        assert!(state.flat_registration_fee_in_quants == 700000000, 5);
        assert!(state.congestion_threshold_percentage == 70, 6);
        assert!(state.congestion_base_fee_in_quants_per_sec == 2000, 7);
        assert!(state.congestion_exponent == 5, 8);
        assert!(state.task_capacity == 200, 9);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNACCEPTABLE_AUTOMATION_GAS_LIMIT, location = Self)]
    fun check_automation_gas_limit_failed_update(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            50,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );

        // Next epoch gas committed gas is greater than the new limit value.
        update_config(
            framework,
            TTL_UPPER_BOUND_TEST,
            45,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EUNACCEPTABLE_TASK_DURATION_CAP, location = Self)]
    fun check_config_udpate_with_invalid_task_duration_cap(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        update_config(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EMAX_CONGESTION_THRESHOLD, location = Self)]
    fun check_config_udpate_with_max_congestion_threshold(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        update_config(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            150,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ECONGESTION_EXP_NON_ZERO, location = Self)]
    fun check_config_udpate_with_invalid_congestion_exponent(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        update_config(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            0,
            TASK_CAPACITY_TEST,
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EREGISTRY_MAX_GAS_CAP_NON_ZERO, location = Self)]
    fun check_config_udpate_with_invalid_registry_max_gas_cap(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        // Specified task duration cap is less than epoch length
        update_config(
            framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS + 1,
            0,
            AUTOMATION_BASE_FEE_TEST,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            CONGESTION_BASE_FEE_TEST,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_registration(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        assert!(1 == get_next_task_index(), 1);
        assert!(10 == get_gas_committed_for_next_epoch(), 1)
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EREGISTRY_IS_FULL, location = Self)]
    fun check_registration_with_full_tasks(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        update_config_for_tests(
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
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        // Registry is already full
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_EXPIRY_TIME, location = Self)]
    fun check_registration_invalid_expiry_time(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        timestamp::update_global_time_for_test_secs(50);
        register(user,
            PAYLOAD,
            25,
            70,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EEXPIRY_BEFORE_NEXT_EPOCH, location = Self)]
    fun check_registration_invalid_expiry_time_before_next_epoch(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2,
            70,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EEXPIRY_TIME_UPPER, location = Self)]
    fun check_registration_invalid_expiry_time_surpassing_task_duration_cap(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            TTL_UPPER_BOUND_TEST + 1,
            70,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_registration_valid_expiry_time_matches_task_duration_cap(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            TTL_UPPER_BOUND_TEST,
            70,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_GAS_PRICE, location = Self)]
    fun check_registration_invalid_gas_price_cap(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            86400,
            70,
            0,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_MAX_GAS_AMOUNT, location = Self)]
    fun check_registration_invalid_max_gas_amount(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            0,
            70,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINVALID_TXN_HASH, location = Self)]
    fun check_registration_invalid_parent_hash(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            10,
            70,
            1000,
            vector<u8>[0, 1, 2, 3],
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ENO_AUX_DATA_SUPPORTED, location = Self)]
    fun check_registration_with_aux_data(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        let new_param1 = vector[0u8, 1, 2];
        let aux_data = vector[new_param1];
        register(user,
            PAYLOAD,
            86400,
            10,
            70,
            1000,
            PARENT_HASH,
            aux_data
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EGAS_AMOUNT_UPPER, location = Self)]
    fun check_registration_with_overflow_gas_limit(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            60000000,
            20,
            100_000_000,
            PARENT_HASH,
            AUX_DATA
        );
        assert!(1 == get_next_task_index(), 1);
        assert!(60000000 == get_gas_committed_for_next_epoch(), 1);
        register(user,
            PAYLOAD,
            86400,
            50000000,
            20,
            100_000_000,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH, location = Self)]
    fun check_registration_with_insufficient_automation_fee_cap(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            10_000,
            70,
            1,
            PARENT_HASH,
            AUX_DATA
        );
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_activation_on_new_epoch(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );

        // No active task and committed gas for the next epoch is total of the all registered tasks
        assert!(40 == get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = get_active_task_ids();
        assert!(active_task_ids == vector[], 1);

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        on_new_epoch();
        assert!(40 == get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = get_active_task_ids();
        // But here task 3 is in the active list as it is still active in this new epoch.
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_task_successful_cancellation(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS);
        on_new_epoch();
        assert!(40 == get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });

        // Cancel task 2. The committed gas for the next epoch will be updated,
        // but when requested active task it will be still available in the list
        cancel_task(user, 2);
        // Task will be still available in the registry but with cancelled state
        let task_2_details = get_task_details(2);
        assert!(task_2_details.state == CANCELLED, 1);

        assert!(30 == get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });

        // Add and cancel the task in the same epoch. Task index will be 4
        assert!(get_next_task_index() == 4, 1);
        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        cancel_task(user, 4);
        assert!(30 == get_gas_committed_for_next_epoch(), 1);
        let active_task_ids = get_active_task_ids();
        let expected_ids = vector<u64>[0, 1, 2, 3];
        vector::for_each(active_task_ids, |task_index| {
            assert!(vector::contains(&expected_ids, &task_index), 1);
        });
        // there is no task with index 4 and the next task index will be 5.
        assert!(!has_task_with_id(4), 1);
        assert!(get_next_task_index() == 5, 1)
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EAUTOMATION_TASK_NOT_FOUND, location = Self)]
    fun check_cancellation_of_non_existing_task(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry {
        initialize_registry_test(framework, user);

        cancel_task(user, 1);
    }

    #[test(framework = @supra_framework, user = @0x1cafe, user2 = @0x1cafa)]
    #[expected_failure(abort_code = EUNAUTHORIZED_TASK_OWNER, location = Self)]
    fun check_unauthorized_cancellation_task(
        framework: &signer,
        user: &signer,
        user2: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        cancel_task(user2, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = EALREADY_CANCELLED, location = Self)]
    fun check_cancellation_of_cancelled_task(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            86400,
            10,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
        timestamp::update_global_time_for_test_secs(50);
        on_new_epoch();
        // Cancel the same task 2 times
        cancel_task(user, 0);
        cancel_task(user, 0);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_normal_fee_charge_on_new_epoch(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            86400,
            1_000_000, // normal gas amount
            20,
            100_000,
            PARENT_HASH,
            AUX_DATA
        );

        // check user balance after registered new task
        let registry_fee_address = get_registry_fee_address();
        let user_account = address_of(user);
        let expected_current_balance = ACCOUNT_BALANCE - FLAT_REGISTRATION_FEE_TEST;
        check_account_balance(user_account, expected_current_balance);
        check_account_balance(registry_fee_address, REGISTRY_DEFAULT_BALANCE + FLAT_REGISTRATION_FEE_TEST);

        timestamp::update_global_time_for_test_secs(50);
        on_new_epoch();

        // 10 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee = 10 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        // check user balance after on new epoch fee applied
        check_account_balance(user_account, expected_current_balance - expected_automation_fee);
        check_account_balance(
            registry_fee_address,
            REGISTRY_DEFAULT_BALANCE + FLAT_REGISTRATION_FEE_TEST + expected_automation_fee);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    fun check_congestion_fee_charge_on_new_epoch(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        register(user,
            PAYLOAD,
            86400,
            85_000_000, // congestion threshold reached
            20,
            10_000_000,
            PARENT_HASH,
            AUX_DATA
        );

        // check user balance after registered new task
        let registry_fee_address = get_registry_fee_address();
        let user_address = address_of(user);
        let expected_current_balance = ACCOUNT_BALANCE - FLAT_REGISTRATION_FEE_TEST;
        check_account_balance(user_address, expected_current_balance);
        check_account_balance(registry_fee_address, REGISTRY_DEFAULT_BALANCE + FLAT_REGISTRATION_FEE_TEST);

        timestamp::update_global_time_for_test_secs(50);
        on_new_epoch();

        // 85/100 * 1000 = 850 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 850;
        // 5% surpasses the threshold, ((1+(5/100))^exponent-1) * 100 = 34 congestion base fee, occupancy 85/100, 7200 epoch duration
        let expected_congestion_fee = 34 * 85 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let expected_epoch_fee = expected_automation_fee + expected_congestion_fee;
        // check user balance after on new epoch fee applied
        check_account_balance(user_address, expected_current_balance - expected_epoch_fee);
        check_account_balance(
            registry_fee_address,
            REGISTRY_DEFAULT_BALANCE + FLAT_REGISTRATION_FEE_TEST + expected_epoch_fee);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_refund(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        let task_exipry_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;

        register_with_state(
            framework,
            user,
            44_000_000,
            100_000_000,
            task_exipry_time,
            ACTIVE);
        register_with_state(
            framework,
            user,
            44_000_000,
            100_000_000,
            task_exipry_time,
            CANCELLED);
        register_with_state(
            framework,
            user,
            11_000_000,
            100_000_000,
            task_exipry_time,
            PENDING);
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * FLAT_REGISTRATION_FEE_TEST;
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * FLAT_REGISTRATION_FEE_TEST;


        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let fwk_address = address_of(framework);
        let user_address = address_of(user);

        // No refund when there is no locked fee.
        set_locked_fee(framework, 0);
        {
            let ar = borrow_global<AutomationRegistry>(fwk_address);
            let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
            let aei = borrow_global<AutomationEpochInfo>(fwk_address);
            adjust_tasks_epoch_fee_refund(ar, arc, aei, 3600);
            // If there is no locked fee, nothing to refund;
            check_account_balance(user_address, expected_user_current_balance);
            check_account_balance(ar.registry_fee_address, expected_registry_current_balance);
        };

        // Set some locked fee which is enough to pay refund if necessary
        set_locked_fee(framework, 100_000_000);

        {
            // if epoch length matches or greater the expected epoch interval then no refund is expected
            // event if there is a locked fee.
            let ar = borrow_global<AutomationRegistry>(fwk_address);
            let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
            let aei = borrow_global<AutomationEpochInfo>(fwk_address);
            adjust_tasks_epoch_fee_refund(ar, arc, aei, EPOCH_INTERVAL_FOR_TEST_IN_SECS);
            check_account_balance(user_address, expected_user_current_balance);
            check_account_balance(ar.registry_fee_address, expected_registry_current_balance);

            adjust_tasks_epoch_fee_refund(ar, arc, aei, task_exipry_time + EPOCH_INTERVAL_FOR_TEST_IN_SECS);
            check_account_balance(user_address, expected_user_current_balance);
            check_account_balance(ar.registry_fee_address, expected_registry_current_balance);

            // Refund is expected only for ACTIVE AND CANCELLED TASK BUT NOT FOR PENDING
            adjust_tasks_epoch_fee_refund(ar, arc, aei, EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
            // as account have 2 tasks with same automation and congestion fees then refund is double
            let expected_refund = expected_congestion_fee_per_task + expected_automation_fee_per_task;
            expected_user_current_balance = expected_user_current_balance + expected_refund;
            expected_registry_current_balance = expected_registry_current_balance - expected_refund;
            check_account_balance(user_address, expected_user_current_balance);
            // Check registry balance
            check_account_balance(ar.registry_fee_address, expected_registry_current_balance);
        };

        // Refund is expected only for the remaing time till the task expiry time for both cancelled and active tasks
        // Task was expiring in the middle of the 3rd epoch, but epoch duration was cat short by 3/4
        let current_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 4;
        // update epoch-start-time to be 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS
        {
            let aei = borrow_global_mut<AutomationEpochInfo>(fwk_address);
            aei.start_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        };

        let ar = borrow_global<AutomationRegistry>(fwk_address);
        let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
        let aei = borrow_global<AutomationEpochInfo>(fwk_address);
        adjust_tasks_epoch_fee_refund(ar, arc, aei, current_time);
        // It is expected that the tasks will be chared only for 1/2 epoch fee, so if the epoch lenght is 1/4,
        // then refund should be 1/4 and as long as we have 2 tasks for the account the sum will be 1/2
        // as account have 2 tasks with same automation and congestion fees then refund is double
        let expected_refund = (expected_congestion_fee_per_task + expected_automation_fee_per_task) / 2;
        check_account_balance(user_address, expected_user_current_balance + expected_refund);
        // // Check registry balance
        check_account_balance(ar.registry_fee_address, expected_registry_current_balance - expected_refund);
    }

    #[test(framework = @supra_framework, user = @0x1cafb)]
    fun check_automation_task_fee_refund_id_done_with_old_config(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        config_buffer::initialize(framework);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 11_000_000;

        let t1 = register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let t2 = register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, CANCELLED);
        let t3 = register_with_state(
            framework,
            user,
            t3_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, PENDING);
        // Set some locked fee which is enough to pay refund if necessary
        set_locked_fee(framework, 100_000_000);
        update_config(framework,
            TTL_UPPER_BOUND_TEST,
            AUTOMATION_MAX_GAS_TEST,
            AUTOMATION_BASE_FEE_TEST / 2,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST / 2,
            CONGESTION_BASE_FEE_TEST / 2,
            CONGESTION_EXPONENT_TEST - 1,
            TASK_CAPACITY_TEST,
        );
        // Disable feature in order to avoid charges and check only refunds.
        toggle_feature_flag(framework, false);

        // 3 task has been registered
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * FLAT_REGISTRATION_FEE_TEST;
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * FLAT_REGISTRATION_FEE_TEST;


        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        // Epoch cut short 2 times

        // Refund is expected
        timestamp::update_global_time_for_test_secs(3600);
        on_new_epoch();
        let balance = coin::balance<SupraCoin>(signer::address_of(user));
        // as account have 2 tasks with same automation and congestion fees then refund is double
        let expected_refund = expected_congestion_fee_per_task + expected_automation_fee_per_task;
        assert!(balance == expected_user_current_balance + expected_refund, 12);
        // Checke registry balance
        check_account_balance(get_registry_fee_address(), expected_registry_current_balance - expected_refund);

        // Check that config is updated even if feature is disabled.
        let fwk_address = address_of(framework);
        let arc = borrow_global<ActiveAutomationRegistryConfig>(fwk_address);
        assert!(arc.main_config.registry_max_gas_cap == AUTOMATION_MAX_GAS_TEST, 14);
        assert!(arc.main_config.automation_base_fee_in_quants_per_sec == AUTOMATION_BASE_FEE_TEST / 2, 14);
        assert!(arc.main_config.congestion_threshold_percentage == CONGESTION_THRESHOLD_TEST / 2, 14);
        assert!(arc.main_config.congestion_base_fee_in_quants_per_sec == CONGESTION_BASE_FEE_TEST / 2, 14);
        assert!(arc.main_config.congestion_exponent == CONGESTION_EXPONENT_TEST - 1, 14);
        // Check that if feature is disabled, cleanup happens and no task is available in the registry.
        assert!(!has_task_with_id(t1), 15);
        assert!(!has_task_with_id(t2), 15);
        assert!(!has_task_with_id(t3), 15);
        assert!(get_task_count() == 0, 15);
        let ar = borrow_global<AutomationRegistry>(fwk_address);
        // Check that committed gas for this epoch is sum of active tasks max-gass
        assert!(ar.gas_committed_for_this_epoch == 0, 16);
        // Check locked fee is 0 as feature is disabled and no charges have been done.
        assert!(ar.epoch_locked_fees == 0, 17);
        assert!(ar.gas_committed_for_next_epoch == 0, 17);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_calculation(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 11_000_000;

        register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, CANCELLED);
        register_with_state(
            framework,
            user,
            t3_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, PENDING);

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        // Epoch cut short 2 times
        let fwk_address = address_of(framework);

        // if epoch length matches or greater the expected epoch interval then no refund is expected
        // event if there is a locked fee.
        let ar = borrow_global<AutomationRegistry>(fwk_address);
        let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
        let tcmg = ((2 * t1_t2_max_gas) as u256);
        // Take into account CANCELLED tasks as well
        // Tasks are still valid
        let results = calculate_tasks_automation_fees(
            ar,
            arc,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            0,
            tcmg,
            true);
        // Pending task is ignored
        assert!(vector::length(&results) == 2, 2);

        let expected_fee = expected_automation_fee_per_task + expected_congestion_fee_per_task;
        let r1 = vector::borrow(&results, 0);
        let r2 = vector::borrow(&results, 1);
        assert!(r1.fee == expected_fee, 3);
        assert!(r2.fee == expected_fee, 4);

        // Take into account CANCELLED tasks as well
        // Tasks are still valid but for the half of the epoch, as current time is 3600
        let current_time = EPOCH_INTERVAL_FOR_TEST_IN_SECS + EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2;
        let results = calculate_tasks_automation_fees(
            ar,
            arc,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            current_time,
            tcmg,
            true
        );
        // Pending task is ignored
        assert!(vector::length(&results) == 2, 2);

        let expected_fee = (expected_automation_fee_per_task + expected_congestion_fee_per_task) / 2;
        let r1 = vector::borrow(&results, 0);
        let r2 = vector::borrow(&results, 1);
        assert!(r1.fee == expected_fee, 3);
        assert!(r2.fee == expected_fee, 4);

        // Take into account CANCELLED tasks as well
        // Tasks are considered as expired
        let current_time = 2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        let results = calculate_tasks_automation_fees(
            ar,
            arc,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            current_time,
            tcmg,
            true
        );
        // Pending task is ignored
        assert!(vector::length(&results) == 2, 2);

        let r1 = vector::borrow(&results, 0);
        let r2 = vector::borrow(&results, 1);
        assert!(r1.fee == 0, 5);
        assert!(r2.fee == 0, 6);

        // Take into account only ACTIVE tasks
        // Tasks are considered as expired
        let results = calculate_tasks_automation_fees(
            ar,
            arc,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            0,
            tcmg,
            false);
        // Pending task is ignored
        assert!(vector::length(&results) == 1, 2);

        let expected_fee = (expected_automation_fee_per_task + expected_congestion_fee_per_task);
        let r1 = vector::borrow(&results, 0);
        assert!(r1.fee == expected_fee, 7);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_calculation_with_zero_multipliers(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 11_000_000;

        register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, CANCELLED);
        register_with_state(
            framework,
            user,
            t3_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, PENDING);

        // 44/100 * 1000 = 440 - automation_epoch_fee_per_second, 7200 epoch duration
        let expected_automation_fee_per_task = EPOCH_INTERVAL_FOR_TEST_IN_SECS * 440;
        // 8% surpasses the threshold, ((1+(8/100))^exponent-1) * 100 = 58 congestion base fee, occupancy 44/100, 7200 epoch duration
        let expected_congestion_fee_per_task = 58 * 44 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        // Epoch cut short 2 times
        let fwk_address = address_of(framework);

        // Update config with 0 automation base fee
        update_config_for_tests(framework,
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
            let ar = borrow_global<AutomationRegistry>(fwk_address);
            let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
            let results = calculate_tasks_automation_fees(ar, arc,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            0,
            tcmg,
            true);
            assert!(vector::length(&results) == 2, 10);

            let expected_fee = expected_congestion_fee_per_task;
            let r1 = vector::borrow(&results, 0);
            let r2 = vector::borrow(&results, 1);
            assert!(r1.fee == expected_fee, 1);
            assert!(r2.fee == expected_fee, 2);
        };

        // Update config with 100% congestion treshold, no congestion fee is expected
        update_config_for_tests(framework,
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
            let ar = borrow_global<AutomationRegistry>(fwk_address);
            let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
            let results = calculate_tasks_automation_fees(ar, arc,
                EPOCH_INTERVAL_FOR_TEST_IN_SECS,
                0,
                tcmg,
                true);
            assert!(vector::length(&results) == 2, 11);

            let expected_fee = expected_automation_fee_per_task;
            let r1 = vector::borrow(&results, 0);
            let r2 = vector::borrow(&results, 1);
            assert!(r1.fee == expected_fee, 3);
            assert!(r2.fee == expected_fee, 4);
        };

        // Update config with 0 congestion base fee, no congestion fee is expected
        update_config_for_tests(framework,
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
            let ar = borrow_global<AutomationRegistry>(fwk_address);
            let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
            let results = calculate_tasks_automation_fees(ar, arc,
                EPOCH_INTERVAL_FOR_TEST_IN_SECS,
                0,
                tcmg,
                true);
            assert!(vector::length(&results) == 2, 12);

            let expected_fee = expected_automation_fee_per_task;
            let r1 = vector::borrow(&results, 0);
            let r2 = vector::borrow(&results, 1);
            assert!(r1.fee == expected_fee, 3);
            assert!(r2.fee == expected_fee, 4);
        };

        // Update config with 0 congestion  and automation base fee, no fee calculation is expected at all
        update_config_for_tests(framework,
            EPOCH_INTERVAL_FOR_TEST_IN_SECS,
            AUTOMATION_MAX_GAS_TEST,
            0,
            FLAT_REGISTRATION_FEE_TEST,
            CONGESTION_THRESHOLD_TEST,
            0,
            CONGESTION_EXPONENT_TEST,
            TASK_CAPACITY_TEST,
        );

        let tcmg = ((2 * t1_t2_max_gas) as u256);

        {
            let ar = borrow_global<AutomationRegistry>(fwk_address);
            let arc = &borrow_global<ActiveAutomationRegistryConfig>(fwk_address).main_config;
            let results = calculate_tasks_automation_fees(ar, arc,
                EPOCH_INTERVAL_FOR_TEST_IN_SECS,
                0,
                tcmg,
                true);
            assert!(vector::length(&results) == 0, 13);
        }

    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_automation_task_fee_withdrawal_on_new_epoch(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        let t1_t2_max_gas = 44_000_000;
        let t3_max_gas = 10_000_000;

        // Automation fee cap overflow
        let t1 = register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            10_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        let t2 = register_with_state(
            framework,
            user,
            t1_t2_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);
        // Not enough balance to pay fee
        let t3 = register_with_state(
            framework,
            user,
            t3_max_gas,
            100_000_000,
            2 * EPOCH_INTERVAL_FOR_TEST_IN_SECS, ACTIVE);

        // Update config to cause automation fee cap overflow for the 1st task
        update_config_for_tests(framework,
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
        let expected_user_current_balance = ACCOUNT_BALANCE - 3 * FLAT_REGISTRATION_FEE_TEST;
        let expected_registry_current_balance = REGISTRY_DEFAULT_BALANCE + 3 * FLAT_REGISTRATION_FEE_TEST;
        // Make sure that user account has only enough balance for task 2 automation fee
        let withdraw_amount = expected_user_current_balance - expected_epoch_fee_for_t1_2;
        coin::transfer<SupraCoin>(
            user,
            get_registry_fee_address(),
            withdraw_amount);
        let expected_registry_current_balance = expected_registry_current_balance + withdraw_amount;

        timestamp::update_global_time_for_test_secs(EPOCH_INTERVAL_FOR_TEST_IN_SECS / 2);
        on_new_epoch();

        let tcmg = ((2 * t1_t2_max_gas + t3_max_gas) as u256);
        let user_address = address_of(user);
        let fwk_address = address_of(framework);

        // TASK 1 and 3 are removed/cancelled.
        assert!(!has_task_with_id(t1), 1);
        assert!(!has_task_with_id(t3), 2);
        assert!(has_sender_active_task_with_id(user_address, t2), 3);
        // only one task is charged as the other 2 are cancelled/removed.
        check_account_balance(
            get_registry_fee_address(),
            expected_registry_current_balance + expected_epoch_fee_for_t1_2
        );
        check_account_balance(address_of(user), 0);

        let ar = borrow_global<AutomationRegistry>(fwk_address);
        assert!(ar.gas_committed_for_this_epoch == tcmg, 4);
        assert!(ar.gas_committed_for_next_epoch == t1_t2_max_gas, 5);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_estimate_api(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        let fwk_address = address_of(framework);
        let task_max_gas = 10_000_000;
        // 10/100  * 1000 = 100 - automation_epoch_fee_per_sec, 7200 epoch duration. no congestion fee as threshold is not crossed.
        let expected_automation_fee = 100 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        let result = estimate_automation_fee(task_max_gas);
        assert!(result == expected_automation_fee, 1);

        // expected congestion fee with 85 % congestion
        // 5% surpass, ((1+(5/100))^exponent-1) * 100 = 34 (acf), task occupancy 10% epoch interval 7200
        let expected_congestion_fee = 34 * 10 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let result = estimate_automation_fee_with_committed_occupancy(task_max_gas, 75_000_000);
        assert!(result == expected_automation_fee + expected_congestion_fee, 2);

        // update next epoch committed max gas to cause the same congestion
        {
            let registry = borrow_global_mut<AutomationRegistry>(fwk_address);
            registry.gas_committed_for_next_epoch = 75_000_000;
        };
        let result = estimate_automation_fee(task_max_gas);
        assert!(result == expected_automation_fee + expected_congestion_fee, 2);

        // update next epoch registry max gas cap to resolve the congestion
        {
            let active_config = borrow_global_mut<ActiveAutomationRegistryConfig>(address_of(framework));
            active_config.next_epoch_registry_max_gas_cap = 200_000_000;
        };

        // 10/200 * 1000 occupancy - 50 - automation_epoch_fee_per_sec, 7200 epoch duration. no congestion fee as threshold is not crossed.
        let expected_automation_fee = 50 * EPOCH_INTERVAL_FOR_TEST_IN_SECS;
        let result = estimate_automation_fee(task_max_gas);
        assert!(result == expected_automation_fee, 2);

        // expected congestion fee with 86 % congestion
        // 6% surpass, ((1+(6/100))^exponent-1) * 100 = 41 (acf), task occupancy 5% epoch interval 7200
        let expected_congestion_fee = 41 * 5 * EPOCH_INTERVAL_FOR_TEST_IN_SECS / 100;
        let result = estimate_automation_fee_with_committed_occupancy(task_max_gas, 162_000_000);
        assert!(result == expected_automation_fee + expected_congestion_fee, 2);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun check_registry_fee_success_withdrawal(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry {
        initialize_registry_test(framework, user);
        set_locked_fee(framework, 100_000_000);
        let withdraw_amount = 99_999_999;
        let expected_registry_balance = REGISTRY_DEFAULT_BALANCE - withdraw_amount;
        let expected_user_balance = ACCOUNT_BALANCE + withdraw_amount;
        withdraw_automation_task_fees(framework, address_of(user), withdraw_amount);
        check_account_balance(get_registry_fee_address(), expected_registry_balance);
        check_account_balance(address_of(user), expected_user_balance);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EREQUEST_EXCEEDS_LOCKED_BALANCE, location = Self)]
    fun check_registry_fee_failed_withdrawal_locked_balance(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry {
        initialize_registry_test(framework, user);
        set_locked_fee(framework, 100_000_000);
        let withdraw_amount = REGISTRY_DEFAULT_BALANCE - 80_000_000;
        withdraw_automation_task_fees(framework, address_of(user), withdraw_amount);
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    #[expected_failure(abort_code = EINSUFFICIENT_BALANCE, location = Self)]
    fun check_registry_fee_failed_withdrawal_insufficient_balance(
        framework: &signer,
        user: &signer
    ) acquires AutomationRegistry {
        initialize_registry_test(framework, user);
        let withdraw_amount = REGISTRY_DEFAULT_BALANCE + 1;
        withdraw_automation_task_fees(framework, address_of(user), withdraw_amount);
    }

    #[test]
    fun check_sort_by_task_index() {
        let t1 = AutomationTaskFee {
            task_index: 1,
            owner: @0x0123456,
            fee: 10,
        };
        let t2 = AutomationTaskFee {
            task_index: 2,
            owner: @0x0123456,
            fee: 5,
        };
        let t3 = AutomationTaskFee {
            task_index: 3,
            owner: @0x0123456,
            fee: 30,
        };
        let t4 = AutomationTaskFee {
            task_index: 4,
            owner: @0x0123456,
            fee: 10,
        };
        let t5 = AutomationTaskFee {
            task_index: 5,
            owner: @0x0123456,
            fee: 1,
        };
        let task_fee_vec = vector[t5, t3, t1, t4, t2];
        sort_by_task_index(&mut task_fee_vec);
        let i = 0;
        while (i < 5) {
            let item = vector::borrow(&task_fee_vec, i);
            assert!(i + 1 == item.task_index, i);
            i = i + 1;
        };
    }

    #[test]
    fun check_calculate_exponentiation() {
        // 5% threshould which means (5/100) * DECIMAL
        let result = calculate_exponentiation(5 * DECIMAL / 100, CONGESTION_EXPONENT_TEST);
        assert!(result == 34009563, 11); // ~0.34

        // 28% threshould which means (28/100) * DECIMAL
        let result = calculate_exponentiation(28 * DECIMAL / 100, CONGESTION_EXPONENT_TEST);
        assert!(result == 339804650, 12); // ~3.39

        // 50% threshould which means (50/100) * DECIMAL
        let result = calculate_exponentiation(50 * DECIMAL / 100, CONGESTION_EXPONENT_TEST);
        assert!(result == 1039062500, 13); // ~10.39
    }

    #[test(framework = @supra_framework, user = @0x1cafa)]
    fun test_registration_enable_disable(framework: &signer, user: &signer) acquires ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);
        assert!(is_registration_enabled(), 14);

        disable_registration(framework);
        assert!(!is_registration_enabled(), 15);

        enable_registration(framework);
        assert!(is_registration_enabled(), 16);
    }

    #[test(framework = @supra_framework, user = @0x1cafe)]
    #[expected_failure(abort_code = ETASK_REGISTRATION_DISABLED, location = Self)]
    fun test_register_fails_when_registration_disabled(
        framework: &signer, user: &signer
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig {
        initialize_registry_test(framework, user);

        disable_registration(framework);
        assert!(!is_registration_enabled(), 17);

        register(user,
            PAYLOAD,
            86400,
            50,
            20,
            1000,
            PARENT_HASH,
            AUX_DATA
        );
    }
}
