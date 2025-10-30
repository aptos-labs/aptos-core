/// Copywrite (c) -- 2025 Supra
/// Supra Automation Registry
///
/// This contract is part of the Supra Framework and is designed to manage automated task entries
module supra_framework::automation_registry {

    use std::bcs;
    use std::features;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_std::any::Any;
    use aptos_std::from_bcs;
    use aptos_std::math64;
    use aptos_std::simple_map;
    use aptos_std::simple_map::SimpleMap;
    use supra_framework::multisig_account;
    use supra_framework::system_addresses::assert_supra_framework;
    use supra_framework::coin::Coin;

    use supra_std::enumerable_map::{Self, EnumerableMap};
    use supra_std::vector_utils;

    use supra_framework::account::{Self, SignerCapability};
    use supra_framework::coin;
    use supra_framework::config_buffer;
    use supra_framework::create_signer::create_signer;
    use supra_framework::event;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::system_addresses;
    use supra_framework::timestamp;

    use supra_std::vector_utils::sort_vector_u64;
    #[test_only]
    use std::option;

    friend supra_framework::block;
    friend supra_framework::genesis;
    friend supra_framework::reconfiguration_with_dkg;

    #[test_only]
    friend supra_framework::automation_registry_tests;

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

    /// The length of the transaction hash.
    const TXN_HASH_LENGTH: u64 = 32;
    /// Conversion factor between microseconds and second
    const MICROSECS_CONVERSION_FACTOR: u64 = 1_000_000;
    /// Registry resource creation seed
    const REGISTRY_RESOURCE_SEED: vector<u8> = b"supra_framework::automation_registry";
    /// Max U64 value
    const MAX_U64: u128 = 18446744073709551615;
    /// Decimal place to make
    // 10^8 Power
    const DECIMAL: u256 = 100_000_000;
    /// 100 Percentage
    const MAX_PERCENTAGE: u8 = 100;
    const REFUND_FRACTION: u64 = 2;

    /// Constants describing task state.
    const PENDING: u8 = 0;
    const ACTIVE: u8 = 1;
    const CANCELLED: u8 = 2;

    /// Constants decribing the task type, USER SUBMITTED TASK (UST - 1), GOVERNANCE SUBMITTED TASK(GST - 2)
    const UST: u8 = 1;
    const GST: u8 = 2;

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

    /// Constants describing REFUND TYPE
    const DEPOSIT_EPOCH_FEE: u8 = 0;
    const EPOCH_FEE: u8 = 1;

    /// Defines divisor for refunds of deposit fees with penalty
    /// Factor of `2` suggests that `1/2` of the deposit will be refunded.
    const REFUND_FACTOR: u64 = 2;

    /// Constants defining single task processing maximum limits
    /// Single task processing execution gas.
    /// max_execution_gas is defined 920_000_000, where scaling factor is 1_000_000.
    const TASK_EXECUTION_GAS: u64 = 4_000_000;
    /// Single task processing IO gas.
    const TASK_IO_GAS: u64 = 10_000_000;
    /// Max storage fee per task.
    const TASK_STORAGE_FEE: u64 = 1000;
    /// Max write operation per task.
    const TASK_WRITE_OPS: u64 = 10;
    /// Task support factor in percentage. It should not exceed 100.
    const TASK_SUPPORT_FACTOR: u64 = 80;

    /// Supported aux data count
    const SUPPORTED_AUX_DATA_COUNT_MAX: u64 = 2;
    /// Index of the aux data holding type value
    const TYPE_AUX_DATA_INDEX: u64 = 0;
    /// Index of the aux data holding task priority value
    const PRIORITY_AUX_DATA_INDEX: u64 = 1;

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
    /// Registry active configuration parameters for the current cycle.
    struct ActiveAutomationRegistryConfigV2 has key {
        main_config: AutomationRegistryConfig,
        /// Will be the same as main_config.registry_max_gas_cap, unless updated during the cycle transiation.
        next_cycle_registry_max_gas_cap: u64,
        /// Flag indicating whether the task registration is enabled or paused.
        /// If paused a new task registration will fail.
        registration_enabled: bool,
        /// Configuration parameters for system tasks
        system_task_config: RegistryConfigForSystemTasks,
        /// Will be the same as system_task_config.registry_max_gas_cap, unless updated during the cycle transition.
        next_cycle_sys_registry_max_gas_cap: u64,
        /// Auxiliary configurations to support future expansions.
        aux_configs: SimpleMap<String, Any>
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    #[event]
    /// Automation registry configuration parameters
    struct AutomationRegistryConfig has key, store, drop, copy {
        /// Maximum allowable duration (in seconds) from the registration time that an automation task can run.
        /// If the expiration time exceeds this duration, the task registration will fail.
        task_duration_cap_in_secs: u64,
        /// Maximum gas allocation for automation tasks per cycle
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

    /// Automation registry configuration parameters for governance/system submitted tasks
    struct RegistryConfigForSystemTasks has store, drop, copy {
        /// Maximum allowable duration (in seconds) from the registration time that an system automation task can run.
        /// If the expiration time exceeds this duration, the task registration will fail.
        task_duration_cap_in_secs: u64,
        /// Maximum gas allocation for system automation tasks per cycle
        /// Exceeding this limit during task registration will cause failure and is used in fee calculation.
        registry_max_gas_cap: u64,
        /// Maximum number of system tasks that registry can hold.
        task_capacity: u16,
        /// Auxiliary configuration properties to easy expansion after release if required.
        aux_properties: SimpleMap<String, u64>
    }

    #[event]
    /// Automation registry configuration parameters
    struct AutomationRegistryConfigV2 has store, drop, copy {
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
        /// Automation cycle duration in secods
        cycle_duration_secs: u64,
        /// Maximum allowable duration (in seconds) from the registration time that an system automation task can run.
        /// If the expiration time exceeds this duration, the task registration will fail.
        sys_task_duration_cap_in_secs: u64,
        /// Maximum gas allocation for system automation tasks per cycle
        /// Exceeding this limit during task registration will cause failure and is used in fee calculation.
        sys_registry_max_gas_cap: u64,
        /// Maximum number of system tasks that registry can hold.
        sys_task_capacity: u16,
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

    /// It tracks entries both pending and completed, organized by unique indices.
    struct RegistryStateForSystemTasks has store {
        /// Gas committed for next cycle
        gas_committed_for_next_cycle: u64,
        /// Total committed max gas amount at the beginning of the current cycle.
        gas_committed_for_this_cycle: u64,
        /// Cached system task indexes
        task_ids: vector<u64>,
        /// Authorized accounts to registry system tasks
        authorized_accounts: vector<address>,
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    /// It tracks entries both pending and active for user and system automation tasks, organized by unique indices.
    struct AutomationRegistryV2 has key, store {
        main: AutomationRegistry,
        system_tasks_state: RegistryStateForSystemTasks,
    }


    /// It tracks entries both pending and completed, organized by unique indices.
    /// Holds intermediate state data of the automation cycle transition from END->STARTED, or SUSPENDED->READY
    struct TransitionState has copy, drop, store {
        /// Refund duration of automation fees when automation feature/cycle is suspended.
        refund_duration: u64,
        /// Duration of the new cycle to charge fees for.
        new_cycle_duration: u64,
        /// Calculated automation fee per second for a new cycle or for refund period.
        automation_fee_per_sec: u64,
        /// Gas committed for the new cycle being transitioned.
        gas_committed_for_new_cycle: u64,
        /// Gas committed for the next cycle.
        gas_committed_for_next_cycle: u64,
        /// Gas committed by system tasks for the next cycle.
        sys_gas_committed_for_next_cycle: u64,
        /// Total fee charged from users for the new cycle, which is not withdrawable.
        locked_fees: u64,
        /// List of the tasks to be processed during transition.
        /// This list is sorted in ascending order.
        /// The requirement is that all tasks are processed in the order of their registration. Which should be true
        /// especially for cycle fee charges before new cycle start.
        expected_tasks_to_be_processed: vector<u64>,
        /// Position of the task index in the expected_tasks_to_be_processed to be processed next.
        /// It is incremented when an expected task is successfully processed.
        next_task_index_position: u64
    }

    fun is_transition_finalized(state: &TransitionState): bool {
        vector::length(&state.expected_tasks_to_be_processed) == state.next_task_index_position
    }
    fun is_transition_in_progress(state: &TransitionState): bool {
        state.next_task_index_position != 0
    }

    fun mark_task_processed(state: &mut TransitionState, task_index: u64) {
        assert!(state.next_task_index_position < vector::length(&state.expected_tasks_to_be_processed), EINCONSISTENT_TRANSITION_STATE);
        let expected_task = vector::borrow(&state.expected_tasks_to_be_processed, state.next_task_index_position);
        assert!(expected_task == &task_index, EOUT_OF_ORDER_TASK_PROCESSING_REQUEST);
        state.next_task_index_position = state.next_task_index_position + 1;
    }

    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    /// Epoch state. Deprecated since SUPRA_AUTOMATION_V2 version.
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

    /// Provides information of the current cycle state.
    struct AutomationCycleInfo has copy, drop, store {
        /// Current cycle id. Incremented when a start of a new cycle is given.
        index: u64,
        /// State of the current cycle.
        state: u8,
        /// Current cycle start time which is updated with the current chain time when a cycle is incremented.
        start_time: u64,
        /// Automation cycle duration in seconds.
        duration_secs: u64,
    }

    #[event]
    /// Event emitted for cycle state transition.
    struct AutomationCycleEvent has copy, drop, store {
        /// Updated cycle state information.
        cycle_state_info: AutomationCycleInfo,
        /// The state transitioned from
        old_state: u8,
    }

    // Unless we provide view API to get the details, it should not be part of any resouce group to be
    // able to fetch via OnChainConfig API
    /// Cycle state.
    struct AutomationCycleDetails has key, copy, drop {
        /// Cycle index corresponding to the current state. Incremented when a transition to the new cycle is finalized.
        index: u64,
        /// State of the current cycle.
        state: u8,
        /// Current cycle start time which is updated with the current chain time when a cycle is incremented.
        start_time: u64,
        /// Automation cycle duration in seconds for the current cycle.
        duration_secs: u64,
        /// Intermediate state of cycle transition to next one or suspended state.
        transition_state: std::option::Option<TransitionState>,
    }


    #[resource_group_member(group = supra_framework::object::ObjectGroup)]
    /// Automation Deposited fee bookkeeping configs
    struct AutomationRefundBookkeeping has key, copy {
        /// Total deposited fee so far which is locked in resource account unless refund of it (fully or partially) is done.
        /// Regardless of the refunded amount the actual deposited amount is deduced to unlock it from the resource account.
        total_deposited_automation_fee: u64
        // TODO here we can have also configuration parameter like REFUND_FACTOR
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
        /// Deposit fee locked for the task equal to the automation-fee-cap for epoch specified for it.
        /// It will be refunded fully when active task is expired or cancelled by user
        /// and partially if a pending task is cancelled by user or an active task is cancelled by the system due to
        /// insufficient balance to  pay the automation fee for the epoch
        locked_fee_for_next_epoch: u64,
    }

    public fun is_of_type(task: &AutomationTaskMetaData, type: u8): bool {
        assert!(vector::length(&task.aux_data) == SUPPORTED_AUX_DATA_COUNT_MAX, EINVALID_AUX_DATA_LENGTH);
        let type_data = vector::borrow(&task.aux_data, TYPE_AUX_DATA_INDEX);
        assert!(vector::length(type_data) == 1, EINVALID_TASK_TYPE_LENGTH);
        let type_value = vector::borrow(type_data, 0);
        *type_value == type
    }

    #[event]
    /// Event on task registration fee withdrawal from owner account upon registration.
    struct TaskRegistrationFeeWithdraw has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    #[event]
    /// Event on task registration fee withdrawal from owner account upon registration.
    struct TaskRegistrationDepositFeeWithdraw has drop, store {
        task_index: u64,
        owner: address,
        registration_fee: u64,
        locked_deposit_fee: u64,
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
    /// Event emitted when a deposit fee is refunded for an automation task.
    struct TaskDepositFeeRefund has drop, store {
        task_index: u64,
        owner: address,
        amount: u64,
    }

    #[event]
    /// Event emitted when an automation fee is being refunded but inner state bookkeeping total locked deposits is less than
    /// potential locked deposit for the task.
    struct ErrorUnlockTaskDepositFee has drop, store {
        task_index: u64,
        total_registered_deposit: u64,
        locked_deposit: u64
    }

    #[event]
    /// Event emitted when a task epoch fee is being refunded but locked epoch fees is less than
    /// potential requested refund.
    struct ErrorUnlockTaskEpochFee has drop, store {
        task_index: u64,
        locked_epoch_fees: u64,
        refund: u64
    }

    #[event]
    /// Event emitted on automation task cancellation by owner.
    struct TaskCancelled has drop, store {
        task_index: u64,
        owner: address,
    }

    #[event]
    /// Event emitted on automation task cancellation by owner.
    struct TaskCancelledV2 has drop, store {
        task_index: u64,
        owner: address,
        registration_hash: vector<u8>
    }

    #[event]
    /// Event emitted on automation tasks stopped by owner.
    struct TasksStopped has drop, store {
        tasks: vector<TaskStopped>,
        owner: address,
    }

    struct TaskStopped has drop, store {
        task_index: u64,
        deposit_refund: u64,
        epoch_fee_refund: u64,
    }

    #[event]
    /// Event emitted on automation tasks stopped by owner.
    struct TasksStoppedV2 has drop, store {
        tasks: vector<TaskStoppedV2>,
        owner: address,
    }

    struct TaskStoppedV2 has drop, store {
        task_index: u64,
        deposit_refund: u64,
        epoch_fee_refund: u64,
        registration_hash: vector<u8>
    }

    #[event]
    /// Event emitted when an automation task is cancelled due to insufficient balance.
    struct TaskCancelledInsufficentBalance has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    #[event]
    /// Event emitted when an automation task is cancelled due to insufficient balance.
    struct TaskCancelledInsufficentBalanceV2 has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
        balance: u64,
        registration_hash: vector<u8>
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
    /// Event emitted when an automation task is cancelled due to automation fee capacity surpass.
    struct TaskCancelledCapacitySurpassedV2 has drop, store {
        task_index: u64,
        owner: address,
        fee: u64,
        automation_fee_cap: u64,
        registration_hash: vector<u8>
    }

    #[event]
    /// Event emitted on epoch transition containing removed task indexes.
    struct RemovedTasks has drop, store {
        task_indexes: vector<u64>
    }

    #[event]
    /// Event emitted on epoch transition containing active task indexes for the new epoch.
    struct ActiveTasks has drop, store {
        task_indexes: vector<u64>
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
    /// Event emitted during epoch transition when refunds to be paid is not possible due to insufficient resource account balance.
    /// Type of the refund can be related either to the deposit paid during registration (0), or to epoch-fee caused by
    /// the shortening of the epoch (1)
    struct ErrorInsufficientBalanceToRefund has drop, store {
        refund_type: u8,
        task_index: u64,
        owner: address,
        amount: u64,
    }

    #[event]
    /// Event emitted when on new epoch inconsistent state of the registry has been identified.
    /// When automation is in suspended state, there are no tasks expected.
    struct ErrorInconsistentSuspendedState has drop, store {}

    #[event]
    /// Emitted when the registration in the automation registry is enabled.
    struct EnabledRegistrationEvent has drop, store {}

    #[event]
    /// Emitted when the registration in the automation registry is disabled.
    struct DisabledRegistrationEvent has drop, store {}

    #[event]
    /// Emitted when the account is authorized to submit system automation tasks
    struct AuthorizationGranted has drop, store {
        account: address
    }

    #[event]
    /// Emitted when the account authorization is revoked to submit system automation tasks
    struct AuthorizationRevoked has drop, store {
        account: address
    }

    /// Represents the fee charged for an automation task execution and some additional information.
    struct AutomationTaskFeeMeta has drop {
        task_index: u64,
        owner: address,
        fee: u64,
        automation_fee_cap: u64,
        expiry_time: u64,
        max_gas_amount: u64,
        locked_deposit_fee: u64
    }

    /// Represents intermediate state of the registry on epoch change.
    /// Deprecated in production, substituted with `IntermediateStateOfEpochChange`.
    /// Kept for backward compatible framework upgrade.
    struct IntermediateState has drop {
        active_task_ids: vector<u64>,
        gas_committed_for_next_epoch: u64,
        epoch_locked_fees: u64,
    }

    /// Represents intermediate state of the registry on epoch change.
    /// Deprecated in production, substituted with `IntermediateStateOfCycleChange`.
    /// Kept for backward compatible framework upgrade.
    struct IntermediateStateOfEpochChange {
        removed_tasks: vector<u64>,
        gas_committed_for_new_epoch: u64,
        gas_committed_for_next_epoch: u64,
        epoch_locked_fees: Coin<SupraCoin>,
    }

    /// Represents intermediate state of the registry on cycle change.
    struct IntermediateStateOfCycleChange {
        removed_tasks: vector<u64>,
        gas_committed_for_next_cycle: u64,
        sys_gas_committed_for_next_cycle: u64,
        epoch_locked_fees: Coin<SupraCoin>,
    }

    #[view]
    /// Checks whether all required resources are created.
    public fun is_initialized(): bool {
        exists<AutomationRegistryV2>(@supra_framework)
            && exists<AutomationRefundBookkeeping>(@supra_framework)
            && exists<ActiveAutomationRegistryConfigV2>(@supra_framework)
            && exists<AutomationCycleDetails>(@supra_framework)
    }

    #[view]
    /// Means to query by user whether the automation registry has been properly initialized and ready to be utilized.
    public fun is_feature_enabled_and_initialized(): bool {
        features::supra_native_automation_enabled() && is_initialized()
    }

    #[view]
    /// Returns next task index in registry
    public fun get_next_task_index(): u64 acquires AutomationRegistryV2 {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        automation_registry.main.current_index
    }

    #[view]
    /// Returns number of available tasks.
    public fun get_task_count(): u64 acquires AutomationRegistryV2 {
        let state = borrow_global<AutomationRegistryV2>(@supra_framework);
        enumerable_map::length(&state.main.tasks)
    }

    #[view]
    /// Returns number of available system tasks.
    public fun get_system_task_count(): u64 acquires AutomationRegistryV2 {
        let state = borrow_global<AutomationRegistryV2>(@supra_framework);
        vector::length(&state.system_tasks_state.task_ids)
    }

    #[view]
    /// List all automation task ids available in register.
    public fun get_task_ids(): vector<u64> acquires AutomationRegistryV2 {
        let state = borrow_global<AutomationRegistryV2>(@supra_framework);
        enumerable_map::get_map_list(&state.main.tasks)
    }

    #[view]
    /// Get locked balance of the resource account in terms of epoch-fees
    public fun get_epoch_locked_balance(): u64 acquires AutomationRegistryV2 {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        automation_registry.main.epoch_locked_fees
    }

    #[view]
    /// Get locked balance of the resource account in terms of deposited automation fees.
    public fun get_locked_deposit_balance(): u64 acquires AutomationRefundBookkeeping {
        let refund_bookkeeping = borrow_global<AutomationRefundBookkeeping>(@supra_framework);
        refund_bookkeeping.total_deposited_automation_fee
    }

    #[view]
    /// Get total locked balance of the resource account.
    public fun get_registry_total_locked_balance(): u64 acquires AutomationRefundBookkeeping, AutomationRegistryV2 {
        get_epoch_locked_balance() + get_locked_deposit_balance()
    }

    #[view]
    /// List all active automation task ids for the current epoch.
    /// Note that the tasks with CANCELLED state are still considered active for the current epoch,
    /// as cancellation takes effect in the next epoch only.
    public fun get_active_task_ids(): vector<u64> acquires AutomationRegistryV2 {
        let state = borrow_global<AutomationRegistryV2>(@supra_framework);
        state.main.epoch_active_task_ids
    }

    #[view]
    /// Retrieves the details of a automation task entry by its task index.
    /// Error will be returned if entry with specified task index does not exist.
    public fun get_task_details(task_index: u64): AutomationTaskMetaData acquires AutomationRegistryV2 {
        let registry_state = borrow_global<AutomationRegistryV2>(@supra_framework);
        assert!(enumerable_map::contains(&registry_state.main.tasks, task_index), EAUTOMATION_TASK_NOT_FOUND);
        enumerable_map::get_value(&registry_state.main.tasks, task_index)
    }

    /// Retrieves specific metadata details of an automation task entry by its task index.
    ///
    /// 1. `address`                 - The owner of the task.
    /// 2. `vector<u8>`              - The payload transaction (encoded).
    /// 3. `u64`                     - The expiry time of the task (timestamp).
    /// 4. `vector<u8>`              - The hash of the transaction.
    /// 5. `u64`                     - The maximum gas amount allowed for the task.
    /// 6. `u64`                     - The gas price cap for executing the task.
    /// 7. `u64`                     - The automation fee cap for the current epoch.
    /// 8. `vector<vector<u8>>`      - Auxiliary data related to the task (can be multiple items).
    /// 9. `u64`                     - The time at which the task was registered (timestamp).
    /// 10. `u8`                     - The state of the task (e.g., active, cancelled, completed).
    /// 11. `u64`                    - The locked fee reserved for the next epoch execution.
    public fun deconstruct_task_metadata(
        task_metadata: &AutomationTaskMetaData
    ): (address, vector<u8>, u64, vector<u8>, u64, u64, u64, vector<vector<u8>>, u64, u8, u64) {
        (
            task_metadata.owner,
            task_metadata.payload_tx,
            task_metadata.expiry_time,
            task_metadata.tx_hash,
            task_metadata.max_gas_amount,
            task_metadata.gas_price_cap,
            task_metadata.automation_fee_cap_for_epoch,
            task_metadata.aux_data,
            task_metadata.registration_time,
            task_metadata.state,
            task_metadata.locked_fee_for_next_epoch
        )
    }

    #[view]
    /// Retrieves the owner address of a task by its task index.
    public fun get_task_owner(task_index: u64): address acquires AutomationRegistry {
        let automation_task_metadata = borrow_global<AutomationRegistry>(@supra_framework);
        assert!(enumerable_map::contains(&automation_task_metadata.tasks, task_index), EAUTOMATION_TASK_NOT_FOUND);
        let task_metadata = enumerable_map::get_value(&automation_task_metadata.tasks, task_index);
        task_metadata.owner
    }

    #[view]
    /// Retrieves the details of a automation tasks entry by their task index.
    /// If a task does not exist, it is not included in the result, and no error is reported
    public fun get_task_details_bulk(task_indexes: vector<u64>): vector<AutomationTaskMetaData> acquires AutomationRegistryV2 {
        let registry_state = borrow_global<AutomationRegistryV2>(@supra_framework);
        let task_details = vector[];
        vector::for_each(task_indexes, |task_index| {
            if (enumerable_map::contains(&registry_state.main.tasks, task_index)) {
                vector::push_back(&mut task_details, enumerable_map::get_value(&registry_state.main.tasks, task_index))
            }
        });
        task_details
    }

    #[view]
    /// Checks whether there is an active task in registry with specified input task index.
    public fun has_sender_active_task_with_id(sender: address, task_index: u64): bool acquires AutomationRegistryV2 {
        has_sender_active_task_with_id_and_type(sender, task_index, UST)
    }

    #[view]
    /// Checks whether there is an active system task in registry with specified input task index.
    public fun has_sender_active_system_task_with_id(sender: address, task_index: u64): bool acquires AutomationRegistryV2 {
        has_sender_active_task_with_id_and_type(sender, task_index, GST)
    }

    #[view]
    /// Checks whether there is an active task in registry with specified input task index of the input type.
    /// The type can be either 1 for user submitted tasks, and 2 for governance authorized tasks.
    public fun has_sender_active_task_with_id_and_type(sender: address, task_index: u64, type: u8): bool acquires AutomationRegistryV2 {
        let registry_state = borrow_global<AutomationRegistryV2>(@supra_framework);
        if (enumerable_map::contains(&registry_state.main.tasks, task_index)) {
            let value = enumerable_map::get_value_ref(&registry_state.main.tasks, task_index);
            value.state != PENDING && value.owner == sender && is_of_type(value, type)
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
    public fun get_gas_committed_for_next_epoch(): u64 acquires AutomationRegistryV2 {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        automation_registry.main.gas_committed_for_next_epoch
    }

    #[view]
    /// Get gas committed for the current epoch at the beginning of the epoch.
    public fun get_gas_committed_for_current_epoch(): u64 acquires AutomationRegistryV2 {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        (automation_registry.main.gas_committed_for_this_epoch as u64)
    }

    #[view]
    /// Get automation registry configuration
    public fun get_automation_registry_config(): AutomationRegistryConfig acquires ActiveAutomationRegistryConfigV2 {
        borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework).main_config
    }

    #[view]
    /// Get automation registry configuration for system tasks
    public fun get_automation_registry_config_for_system_tasks(): RegistryConfigForSystemTasks acquires ActiveAutomationRegistryConfigV2 {
        borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework).system_task_config
    }

    #[view]
    /// Get automation registry maximum gas capacity for the next epoch
    public fun get_next_epoch_registry_max_gas_cap(): u64 acquires ActiveAutomationRegistryConfigV2 {
        borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework).next_cycle_registry_max_gas_cap
    }

    #[view]
    /// Get automation epoch info
    public fun get_automation_epoch_info(): AutomationEpochInfo {
        assert!(false, EDEPRECATED_SINCE_V2);
        AutomationEpochInfo {
            expected_epoch_duration: 0,
            epoch_interval: 0,
            start_time: 0,

        }
    }

    #[view]
    /// Estimates automation fee for the next epoch for specified task occupancy for the configured epoch-interval
    /// referencing the current automation registry fee parameters, current total occupancy and registry maximum allowed
    /// occupancy for the next epoch.
    public fun estimate_automation_fee(
        task_occupancy: u64
    ): u64 acquires AutomationRegistryV2, AutomationCycleDetails, ActiveAutomationRegistryConfigV2 {
        let registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        estimate_automation_fee_with_committed_occupancy(task_occupancy, registry.main.gas_committed_for_next_epoch)
    }

    #[view]
    /// Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
    /// referencing the current automation registry fee parameters, specified total/committed occupancy and registry
    /// maximum allowed occupancy for the next epoch.
    public fun estimate_automation_fee_with_committed_occupancy(
        task_occupancy: u64,
        committed_occupancy: u64
    ): u64 acquires AutomationCycleDetails, ActiveAutomationRegistryConfigV2 {
        let cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        let config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        estimate_automation_fee_with_committed_occupancy_internal(
            task_occupancy,
            committed_occupancy,
            cycle_info.duration_secs,
            config
        )
    }

    #[view]
    /// Calculates automation fee per second for the specified task occupancy
    /// referencing the current automation registry fee parameters, specified total/committed occupancy and current registry
    /// maximum allowed occupancy.
    public fun calculate_automation_fee_multiplier_for_committed_occupancy(
        total_committed_max_gas: u64
    ): u64 acquires ActiveAutomationRegistryConfigV2 {
        // Compute the automation fee multiplier for cycle
        let active_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let automation_fee_per_sec = calculate_automation_fee_multiplier_for_epoch(
            &active_config.main_config,
            (total_committed_max_gas as u256),
            active_config.main_config.registry_max_gas_cap);
        (automation_fee_per_sec as u64)
    }

    #[view]
    /// Calculates automation fee per second for the current cycle
    /// referencing the current automation registry fee parameters, and committed gas for this cycle stored in
    /// the automation registry and current maximum allowed occupancy.
    public fun calculate_automation_fee_multiplier_for_current_cycle(): u64 acquires ActiveAutomationRegistryConfigV2, AutomationRegistryV2 {
        // Compute the automation fee multiplier for this cycle
        let active_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        calculate_automation_fee_multiplier_for_current_cycle_internal(active_config, &automation_registry.main)
    }

    #[view]
    /// Returns the current status of the registration in the automation registry.
    public fun is_registration_enabled(): bool acquires ActiveAutomationRegistryConfigV2 {
        borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework).registration_enabled
    }

    #[view]
    /// Returns the current duration of the automation cycle.
    public fun get_cycle_duration(): u64 acquires AutomationCycleDetails {
        borrow_global<AutomationCycleDetails>(@supra_framework).duration_secs
    }

    #[view]
    /// Returns the current cycle info.
    public fun get_cycle_info(): AutomationCycleInfo acquires AutomationCycleDetails {
        let details = borrow_global<AutomationCycleDetails>(@supra_framework);
        into_automation_cycle_info(details)
    }

    #[view]
    /// Returns the maximum number of the tasks that can be processed in scope of single bookkeeping transaction.
    fun get_record_max_task_count(max_execution_gas: u64, max_io_gas: u64, max_storage_fee: u64, max_write_op: u64): u64 {
        let task_count_by_exec_gas = max_execution_gas / TASK_EXECUTION_GAS;
        let task_count_by_io_gas = max_io_gas / TASK_IO_GAS;
        let task_count_by_storage_fee = max_storage_fee / TASK_STORAGE_FEE;
        let task_count_by_write_op = max_write_op / TASK_WRITE_OPS;

        let task_count = math64::min(task_count_by_exec_gas, task_count_by_io_gas);
        task_count = math64::min(task_count, task_count_by_storage_fee);
        task_count = math64::min(task_count, task_count_by_write_op);
        task_count * TASK_SUPPORT_FACTOR / 100
    }

    #[view]
    /// List of system registered tasks
    public fun get_system_task_indexes(): vector<u64> acquires AutomationRegistryV2 {
        let registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        registry.system_tasks_state.task_ids
    }

    #[view]
    /// Get committed gas for the next cycle by system tasks.
    public fun get_system_gas_committed_for_next_cycle(): u64 acquires AutomationRegistryV2 {
        let registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        registry.system_tasks_state.gas_committed_for_next_cycle
    }

    #[view]
    /// Get committed gas for the current cycle by system tasks.
    public fun get_system_gas_committed_for_current_cycle(): u64 acquires AutomationRegistryV2 {
        let registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        registry.system_tasks_state.gas_committed_for_this_cycle
    }

    #[view]
    /// Checks whether the input account address is authorized.
    public fun is_authorized_account(account: address): bool acquires AutomationRegistryV2 {
        let registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        vector::contains(&registry.system_tasks_state.authorized_accounts, &account)
    }

    // Public entry functions

    /// Withdraw accumulated automation task fees from the resource account - access by admin
    public fun withdraw_automation_task_fees(
        supra_framework: &signer,
        to: address,
        amount: u64
    ) acquires AutomationRegistryV2 , AutomationRefundBookkeeping {
        system_addresses::assert_supra_framework(supra_framework);
        transfer_fee_to_account_internal(to, amount);
        event::emit(RegistryFeeWithdraw { to, amount });
    }

    /// Update Automation Registry Config
    public fun update_config(
        _supra_framework: &signer,
        _task_duration_cap_in_secs: u64,
        _registry_max_gas_cap: u64,
        _automation_base_fee_in_quants_per_sec: u64,
        _flat_registration_fee_in_quants: u64,
        _congestion_threshold_percentage: u8,
        _congestion_base_fee_in_quants_per_sec: u64,
        _congestion_exponent: u8,
        _task_capacity: u16,
    ) {
        assert!(false, EDEPRECATED_SINCE_V2);
    }

    /// Update Automation Registry Config along with cycle duration.
    public fun update_config_v2(
        supra_framework: &signer,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
        cycle_duration_secs: u64,
        sys_task_duration_cap_in_secs: u64,
        sys_registry_max_gas_cap: u64,
        sys_task_capacity: u16,
    ) acquires AutomationRegistryV2, ActiveAutomationRegistryConfigV2 {
        system_addresses::assert_supra_framework(supra_framework);

        validate_configuration_parameters_common(
            cycle_duration_secs,
            task_duration_cap_in_secs,
            sys_task_duration_cap_in_secs,
            registry_max_gas_cap,
            sys_registry_max_gas_cap,
            congestion_threshold_percentage,
            congestion_exponent);

        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);

        assert!(
            automation_registry.main.gas_committed_for_next_epoch <= registry_max_gas_cap,
            EUNACCEPTABLE_AUTOMATION_GAS_LIMIT
        );

        assert!(
            automation_registry.system_tasks_state.gas_committed_for_next_cycle <= sys_registry_max_gas_cap,
            EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT
        );

        let new_automation_registry_config = AutomationRegistryConfigV2 {
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            automation_base_fee_in_quants_per_sec,
            flat_registration_fee_in_quants,
            congestion_threshold_percentage,
            congestion_base_fee_in_quants_per_sec,
            congestion_exponent,
            task_capacity,
            cycle_duration_secs,
            sys_task_duration_cap_in_secs,
            sys_registry_max_gas_cap,
            sys_task_capacity
        };
        config_buffer::upsert(copy new_automation_registry_config);

        // next cyle registry max gas cap will be update instantly
        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfigV2>(@supra_framework);
        automation_registry_config.next_cycle_registry_max_gas_cap = registry_max_gas_cap;
        automation_registry_config.next_cycle_sys_registry_max_gas_cap = sys_registry_max_gas_cap;

        event::emit(new_automation_registry_config);
    }

    /// Enables the registration process in the automation registry.
    public fun enable_registration(supra_framework: &signer) acquires ActiveAutomationRegistryConfigV2 {
        system_addresses::assert_supra_framework(supra_framework);
        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfigV2>(@supra_framework);
        automation_registry_config.registration_enabled = true;
        event::emit(EnabledRegistrationEvent {});
    }

    /// Disables the registration process in the automation registry.
    public fun disable_registration(supra_framework: &signer) acquires ActiveAutomationRegistryConfigV2 {
        system_addresses::assert_supra_framework(supra_framework);
        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfigV2>(@supra_framework);
        automation_registry_config.registration_enabled = false;
        event::emit(DisabledRegistrationEvent {});
    }

    /// Grants authorization to the input account to submit system automation tasks.
    public fun grant_authorization(supra_framework: &signer, account: address) acquires AutomationRegistryV2 {
        system_addresses::assert_supra_framework(supra_framework);
        let system_tasks_state = &mut borrow_global_mut<AutomationRegistryV2>(@supra_framework).system_tasks_state;
        if (vector::contains(&system_tasks_state.authorized_accounts, &account)) {
            return
        };
        assert!(multisig_account::account_exists(account), EUNKNOWN_MULTISIG_ADDRESS);
        vector::push_back(&mut system_tasks_state.authorized_accounts, account);
        event::emit(AuthorizationGranted {
            account
        })
    }

    /// Revoke authorization from the input account to submit system automation tasks.
    public fun revoke_authorization(supra_framework: &signer, account: address) acquires AutomationRegistryV2 {
        system_addresses::assert_supra_framework(supra_framework);
        let system_tasks_state = &mut borrow_global_mut<AutomationRegistryV2>(@supra_framework).system_tasks_state;
        if (!vector::contains(&system_tasks_state.authorized_accounts, &account)) {
            return
        };
        vector::remove_value(&mut system_tasks_state.authorized_accounts, &account);
        event::emit(AuthorizationRevoked {
            account
        })
    }

    /// Cancel Automation task with specified task_index.
    /// Only existing task, which is PENDING or ACTIVE, can be cancelled and only by task owner.
    /// If the task is
    ///   - active, its state is updated to be CANCELLED.
    ///   - pending, it is removed form the list.
    ///   - cancelled, an error is reported
    /// Committed gas-limit is updated by reducing it with the max-gas-amount of the cancelled task.
    public entry fun cancel_task(
        owner_signer: &signer,
        task_index: u64
    ) acquires AutomationRegistryV2, AutomationCycleDetails, AutomationRefundBookkeeping{
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        let cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_info.state == CYCLE_STARTED, ECYCLE_TRANSITION_IN_PROGRESS);
        let automation_registry = &mut borrow_global_mut<AutomationRegistryV2>(@supra_framework).main;
        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);
        assert!(enumerable_map::contains(&automation_registry.tasks, task_index), EAUTOMATION_TASK_NOT_FOUND);

        let automation_task_metadata = enumerable_map::get_value(&mut automation_registry.tasks, task_index);
        let owner = signer::address_of(owner_signer);
        assert!(is_of_type(&automation_task_metadata, UST), EUNSUPPORTED_TASK_OPERATION);
        assert!(automation_task_metadata.owner == owner, EUNAUTHORIZED_TASK_OWNER);
        assert!(automation_task_metadata.state != CANCELLED, EALREADY_CANCELLED);
        if (automation_task_metadata.state == PENDING) {
            let resource_signer = account::create_signer_with_capability(
                &automation_registry.registry_fee_address_signer_cap
            );
            // When Pending tasks are cancelled, refund of the deposit fee is done with penalty
            let result = safe_deposit_refund(
                refund_bookkeeping,
                &resource_signer,
                automation_registry.registry_fee_address,
                automation_task_metadata.task_index,
                owner,
                automation_task_metadata.locked_fee_for_next_epoch / REFUND_FACTOR,
                automation_task_metadata.locked_fee_for_next_epoch);
            assert!(result, EDEPOSIT_REFUND);
            enumerable_map::remove_value(&mut automation_registry.tasks, task_index);
        } else { // it is safe not to check the state as above, the cancelled tasks are already rejected.
            // Active tasks will be refunded the deposited amount fully at the end of the epoch
            let automation_task_metadata_mut = enumerable_map::get_value_mut(
                &mut automation_registry.tasks,
                task_index
            );
            automation_task_metadata_mut.state = CANCELLED;
        };

        // This check means the task was expected to be executed in the next cycle, but it has been cancelled.
        // We need to remove its gas commitment from `gas_committed_for_next_epoch` for this particular task.
        if (automation_task_metadata.expiry_time > (cycle_info.start_time + cycle_info.duration_secs)) {
            assert!(
                automation_registry.gas_committed_for_next_epoch >= automation_task_metadata.max_gas_amount,
                EGAS_COMMITTEED_VALUE_UNDERFLOW
            );
            // Adjust the gas committed for the next epoch by subtracting the gas amount of the cancelled task
            automation_registry.gas_committed_for_next_epoch = automation_registry.gas_committed_for_next_epoch - automation_task_metadata.max_gas_amount;
        };

        event::emit(TaskCancelledV2 { task_index: automation_task_metadata.task_index, owner, registration_hash: automation_task_metadata.tx_hash });
    }

    /// Immediately stops automation tasks for the specified `task_indexes`.
    /// Only tasks that exist and are owned by the sender can be stopped.
    /// If any of the specified tasks are not owned by the sender, the transaction will abort.
    /// When a task is stopped, the committed gas for the next epoch is reduced
    /// by the max gas amount of the stopped task. Half of the remaining task fee is refunded.
    public entry fun stop_tasks(
        owner_signer: &signer,
        task_indexes: vector<u64>
    ) acquires AutomationRegistryV2, ActiveAutomationRegistryConfigV2, AutomationCycleDetails, AutomationRefundBookkeeping {
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        let cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_info.state == CYCLE_STARTED, ECYCLE_TRANSITION_IN_PROGRESS);
        // Ensure that task indexes are provided
        assert!(!vector::is_empty(&task_indexes), EEMPTY_TASK_INDEXES);

        let owner = signer::address_of(owner_signer);
        let automation_registry = &mut borrow_global_mut<AutomationRegistryV2>(@supra_framework).main;
        let arc = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework).main_config;
        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);

        let tcmg = automation_registry.gas_committed_for_this_epoch;

        // Compute the automation fee multiplier for epoch
        let automation_fee_per_sec = calculate_automation_fee_multiplier_for_epoch(&arc, tcmg, arc.registry_max_gas_cap);

        let stopped_task_details = vector[];
        let total_refund_fee = 0;
        let epoch_locked_fees = automation_registry.epoch_locked_fees;

        // Calculate refundable fee for this remaining time task in current epoch
        let current_time = timestamp::now_seconds();
        let cycle_end_time = cycle_info.duration_secs + cycle_info.start_time;
        let residual_interval = if (cycle_end_time <= current_time) {
            0
        } else {
            cycle_end_time - current_time
        };

        // Loop through each task index to validate and stop the task
        vector::for_each(task_indexes, |task_index| {
            if (enumerable_map::contains(&automation_registry.tasks, task_index)) {
                // Remove task from registry
                let task = enumerable_map::remove_value(&mut automation_registry.tasks, task_index);
                assert!(is_of_type(&task, UST), EUNSUPPORTED_TASK_OPERATION);

                // Ensure only the task owner can stop it
                assert!(task.owner == owner, EUNAUTHORIZED_TASK_OWNER);

                vector::remove_value(&mut automation_registry.epoch_active_task_ids, &task_index);

                // This check means the task was expected to be executed in the next cycle, but it has been stopped.
                // We need to remove its gas commitment from `gas_committed_for_next_cycle` for this particular task.
                // Also it checks that task should not be cancelled.
                if (task.state != CANCELLED && task.expiry_time > cycle_end_time) {
                    // Prevent underflow in gas committed
                    assert!(
                        automation_registry.gas_committed_for_next_epoch >= task.max_gas_amount,
                        EGAS_COMMITTEED_VALUE_UNDERFLOW
                    );

                    // Reduce committed gas by the stopped task's max gas
                    automation_registry.gas_committed_for_next_epoch = automation_registry.gas_committed_for_next_epoch - task.max_gas_amount;
                };

                let (epoch_fee_refund, deposit_refund) = if (task.state != PENDING) {
                    let task_fee = calculate_task_fee(
                        &arc,
                        &task,
                        residual_interval,
                        current_time,
                        automation_fee_per_sec
                    );
                    // Refund full deposit and the half of the remaining run-time fee when task is active or cancelled stage
                    (task_fee / REFUND_FRACTION, task.locked_fee_for_next_epoch)
                } else {
                    (0, (task.locked_fee_for_next_epoch / REFUND_FRACTION))
                };
                let result = safe_unlock_locked_deposit(
                    refund_bookkeeping,
                    task.locked_fee_for_next_epoch,
                    task.task_index);
                assert!(result, EDEPOSIT_REFUND);
                let (result, remaining_epoch_locked_fees) = safe_unlock_locked_epoch_fee(
                    epoch_locked_fees,
                    epoch_fee_refund,
                    task.task_index);
                assert!(result, EEPOCH_FEE_REFUND);
                epoch_locked_fees = remaining_epoch_locked_fees;

                total_refund_fee = total_refund_fee + (epoch_fee_refund + deposit_refund);

                vector::push_back(
                    &mut stopped_task_details,
                    TaskStoppedV2 { task_index, deposit_refund, epoch_fee_refund, registration_hash: task.tx_hash }
                );
            }
        });

        // Refund and emit event if any tasks were stopped
        if (!vector::is_empty(&stopped_task_details)) {
            let resource_signer = account::create_signer_with_capability(
                &automation_registry.registry_fee_address_signer_cap
            );

            let resource_account_balance = coin::balance<SupraCoin>(automation_registry.registry_fee_address);
            assert!(resource_account_balance >= total_refund_fee, EINSUFFICIENT_BALANCE_FOR_REFUND);
            coin::transfer<SupraCoin>(&resource_signer, owner, total_refund_fee);

            // Emit task stopped event
            event::emit(TasksStoppedV2 {
                tasks: stopped_task_details,
                owner
            });
        };
    }

    /// Immediately stops system automation tasks for the specified `task_indexes`.
    /// Only tasks that exist and are owned by the sender can be stopped.
    /// If any of the specified tasks are not owned by the sender, the transaction will abort.
    /// When a task is stopped, the committed gas for the next epoch is reduced
    /// by the max gas amount of the stopped task.
    public entry fun stop_system_tasks(
        owner_signer: &signer,
        task_indexes: vector<u64>
    ) acquires AutomationRegistryV2, AutomationCycleDetails {
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        let cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_info.state == CYCLE_STARTED, ECYCLE_TRANSITION_IN_PROGRESS);
        // Ensure that task indexes are provided
        assert!(!vector::is_empty(&task_indexes), EEMPTY_TASK_INDEXES);

        let owner = signer::address_of(owner_signer);
        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);

        let stopped_task_details = vector[];

        // Calculate refundable fee for this remaining time task in current epoch
        let cycle_end_time = cycle_info.duration_secs + cycle_info.start_time;

        // Loop through each task index to validate and stop the task
        vector::for_each(task_indexes, |task_index| {
            if (enumerable_map::contains(&automation_registry.main.tasks, task_index)) {
                // Remove task from registry
                let task = enumerable_map::remove_value(&mut automation_registry.main.tasks, task_index);

                // Ensure only the task owner can stop it
                assert!(task.owner == owner, EUNAUTHORIZED_TASK_OWNER);
                assert!(is_of_type(&task, GST), EUNSUPPORTED_TASK_OPERATION);

                vector::remove_value(&mut automation_registry.main.epoch_active_task_ids, &task_index);
                vector::remove_value(&mut automation_registry.system_tasks_state.task_ids, &task_index);

                // This check means the task was expected to be executed in the next cycle, but it has been stopped.
                // We need to remove its gas commitment from `gas_committed_for_next_cycle` for this particular task.
                // Also it checks that task should not be cancelled.
                if (task.state != CANCELLED && task.expiry_time > cycle_end_time) {
                    // Prevent underflow in gas committed
                    assert!(
                        automation_registry.system_tasks_state.gas_committed_for_next_cycle >= task.max_gas_amount,
                        EGAS_COMMITTEED_VALUE_UNDERFLOW
                    );

                    // Reduce committed gas by the stopped task's max gas
                    automation_registry.system_tasks_state.gas_committed_for_next_cycle =
                        automation_registry.system_tasks_state.gas_committed_for_next_cycle - task.max_gas_amount;
                };


                vector::push_back(
                    &mut stopped_task_details,
                    TaskStoppedV2 { task_index, deposit_refund: 0, epoch_fee_refund: 0, registration_hash: task.tx_hash }
                );
            }
        });

        // Refund and emit event if any tasks were stopped
        if (!vector::is_empty(&stopped_task_details)) {
            // Emit task stopped event
            event::emit(TasksStoppedV2 {
                tasks: stopped_task_details,
                owner
            });
        };
    }

    /// Cancel System automation task with specified task_index.
    /// Only existing task, which is PENDING or ACTIVE, can be cancelled and only by task owner.
    /// If the task is
    ///   - active, its state is updated to be CANCELLED.
    ///   - pending, it is removed form the list.
    ///   - cancelled, an error is reported
    /// Committed gas-limit is updated by reducing it with the max-gas-amount of the cancelled task.
    public entry fun cancel_system_task(
        owner_signer: &signer,
        task_index: u64
    ) acquires AutomationRegistryV2, AutomationCycleDetails {
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        let cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_info.state == CYCLE_STARTED, ECYCLE_TRANSITION_IN_PROGRESS);
        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);

        assert!(vector::contains(&automation_registry.system_tasks_state.task_ids, &task_index), ESYSTEM_AUTOMATION_TASK_NOT_FOUND);
        assert!(enumerable_map::contains(&automation_registry.main.tasks, task_index), EAUTOMATION_TASK_NOT_FOUND);

        let automation_task_metadata = enumerable_map::get_value(&mut automation_registry.main.tasks, task_index);
        assert!(is_of_type(&automation_task_metadata, GST), EUNSUPPORTED_TASK_OPERATION);

        let owner = signer::address_of(owner_signer);
        assert!(automation_task_metadata.owner == owner, EUNAUTHORIZED_TASK_OWNER);
        assert!(automation_task_metadata.state != CANCELLED, EALREADY_CANCELLED);
        if (automation_task_metadata.state == PENDING) {
            enumerable_map::remove_value(&mut automation_registry.main.tasks, task_index);
        } else { // it is safe not to check the state as above, the cancelled tasks are already rejected.
            let automation_task_metadata_mut = enumerable_map::get_value_mut(
                &mut automation_registry.main.tasks,
                task_index
            );
            automation_task_metadata_mut.state = CANCELLED;
        };

        // This check means the task was expected to be executed in the next cycle, but it has been cancelled.
        // We need to remove its gas commitment from `gas_committed_for_next_epoch` for this particular task.
        if (automation_task_metadata.expiry_time > (cycle_info.start_time + cycle_info.duration_secs)) {
            assert!(
                automation_registry.system_tasks_state.gas_committed_for_next_cycle >= automation_task_metadata.max_gas_amount,
                EGAS_COMMITTEED_VALUE_UNDERFLOW
            );
            // Adjust the gas committed for the next epoch by subtracting the gas amount of the cancelled task
            automation_registry.system_tasks_state.gas_committed_for_next_cycle =
                automation_registry.system_tasks_state.gas_committed_for_next_cycle - automation_task_metadata.max_gas_amount;
        };

        event::emit(TaskCancelledV2 { task_index: automation_task_metadata.task_index, owner, registration_hash: automation_task_metadata.tx_hash });
    }

    // Public functions facilitating transitions from version to version

    /// Public entry function to initialize bookeeping resource when feature enabling automation deposit fee charges is released.
    public fun initialize_refund_bookkeeping_resource(supra_framework: &signer) {
        system_addresses::assert_supra_framework(supra_framework);
        move_to(supra_framework, AutomationRefundBookkeeping {
            total_deposited_automation_fee: 0
        });
    }

    /// API to gracfully migrate from automation feature v1 inplementation to v2 where bookkeeping of the tasks is
    /// detached from epoch-change and cycle based lifecycle of the automation registry is enabled and
    /// tasks are updated to have UST task-type.
    /// IMPORTANT: Should always be followed by `SUPRA_AUTOMATION_V2` feature flag being enabled and
    /// supra_governance::reconfiguration otherwise registry/chain will end-up in inconsistent state.
    ///
    /// monitor_cycle_end (block_prologue->automation_registry::monitor_cycle_end) which will lead to panic and node will stop
    /// thus not causing any inconcistensy in the chain
    ///
    public fun migrate_v2(supra_framework: &signer, cycle_duration_secs: u64,
        sys_task_duration_cap_in_secs: u64,
        sys_registry_max_gas_cap: u64,
        sys_task_capacity: u16
    ) acquires AutomationRegistry, AutomationEpochInfo, ActiveAutomationRegistryConfig, ActiveAutomationRegistryConfigV2, AutomationCycleDetails, AutomationRegistryV2{
        assert_supra_framework(supra_framework);
        assert!(!features::supra_automation_v2_enabled(), EINVALID_MIGRATION_ACTION);
        assert!(exists<AutomationEpochInfo>(@supra_framework), EINVALID_MIGRATION_ACTION);
        validate_system_configuration_parameters_common(cycle_duration_secs, sys_task_duration_cap_in_secs, sys_registry_max_gas_cap);

        // Prepare the state for migration
        let automation_registry = move_from<AutomationRegistry>(@supra_framework);
        let automation_epoch_info = move_from<AutomationEpochInfo>(@supra_framework);

        let automation_registry_config = borrow_global<ActiveAutomationRegistryConfig>(
            @supra_framework
        ).main_config;

        let current_time = timestamp::now_seconds();
        // Refund the epoch fees as epoch will be cut short and on_new_epoch will be dummy due to migration,
        // so this is the only place to do the refunds
        update_state_for_migration(
            &mut automation_registry,
            &automation_registry_config,
            automation_epoch_info,
            current_time
        );

        // Initializing the cycle releated resouces
        let id = 0;
        move_to(supra_framework, AutomationCycleDetails {
            start_time: current_time,
            index: id,
            duration_secs: cycle_duration_secs,
            state: CYCLE_READY,
            transition_state: std::option::none()
        });

        migrate_registry_config(supra_framework, sys_task_duration_cap_in_secs,  sys_registry_max_gas_cap, sys_task_capacity);

        // Initialize registry state for system tasks and new AutomtionRegistryV2 holding both system task and general registry state
        migrate_registry_state(supra_framework, automation_registry);

        // Remain in CYCLE_READY state if feature is not enabled or registry is not fully initialized
        if (!is_feature_enabled_and_initialized()) {
            return
        };
        // Emit cycle end which will lead the native layer to start preparation to the new cycle.
        let cycle_info = borrow_global_mut<AutomationCycleDetails>(@supra_framework);
        // Update the config to start the cycle with new config.
        update_config_from_buffer_for_migration(cycle_info);
        on_cycle_end_internal(cycle_info);
    }

    // Public friend api

    /// Initialization of Automation Registry with configuration parameters for SUPRA_AUTOMATION_V2 version.
    /// Expected to have this function call either at genesis startup or as part of the SUPRA_FRAMEWORK upgrade where
    /// automation feature is being introduced very first time utilizing `genesis::initialize_supra_native_automation_v2`.
    /// In case if framework upgrade is happening on the chain where automation feature with epoch based lifecycle is
    /// already released and is in ongoing state, then `migrate_v2` function should be utilized instead.
    public(friend) fun initialize(
        supra_framework: &signer,
        cycle_duration_secs: u64,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
        sys_task_duration_cap_in_secs: u64,
        sys_registry_max_gas_cap: u64,
        sys_task_capacity: u16,
    ) {
        system_addresses::assert_supra_framework(supra_framework);
        validate_configuration_parameters_common(
            cycle_duration_secs,
            task_duration_cap_in_secs,
            sys_task_duration_cap_in_secs,
            registry_max_gas_cap,
            sys_registry_max_gas_cap,
            congestion_threshold_percentage,
            congestion_exponent);

        let (registry_fee_resource_signer, registry_fee_address_signer_cap) = create_registry_resource_account(
            supra_framework
        );

        let system_tasks_state =  RegistryStateForSystemTasks {
            gas_committed_for_this_cycle: 0,
            gas_committed_for_next_cycle: 0,
            authorized_accounts: vector[],
            task_ids: vector[],
        };

        let general_registry_state = AutomationRegistry {
            tasks: enumerable_map::new_map(),
            current_index: 0,
            gas_committed_for_next_epoch: 0,
            epoch_locked_fees: 0,
            gas_committed_for_this_epoch: 0,
            registry_fee_address: signer::address_of(&registry_fee_resource_signer),
            registry_fee_address_signer_cap,
            epoch_active_task_ids: vector[],
        };

        move_to(supra_framework, AutomationRegistryV2 {
            main: general_registry_state,
            system_tasks_state
        });

        let system_task_config =  RegistryConfigForSystemTasks {
            task_duration_cap_in_secs: sys_task_duration_cap_in_secs,
            registry_max_gas_cap: sys_registry_max_gas_cap,
            task_capacity: sys_task_capacity,
            aux_properties: simple_map::new(),
        };
        move_to(supra_framework, ActiveAutomationRegistryConfigV2 {
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
            next_cycle_registry_max_gas_cap: registry_max_gas_cap,
            next_cycle_sys_registry_max_gas_cap: sys_registry_max_gas_cap,
            registration_enabled: true,
            system_task_config,
            aux_configs: simple_map::new()
        });

        let (cycle_state, cycle_id) =
            if (features::supra_automation_v2_enabled() && features::supra_native_automation_enabled()) {
                (CYCLE_STARTED, 1)
            } else {
                (CYCLE_READY, 0)
            };

        move_to(supra_framework, AutomationCycleDetails {
            start_time: timestamp::now_seconds(),
            index: cycle_id,
            duration_secs: cycle_duration_secs,
            state: cycle_state,
            transition_state: std::option::none<TransitionState>(),
        });

        initialize_refund_bookkeeping_resource(supra_framework);

    }

    /// Checks the cycle end and emit an event on it.
    /// Does nothing if SUPRA_NATIVE_AUTOMATION or SUPRA_AUTOMATION_V2 is disabled.
    public(friend) fun monitor_cycle_end() acquires AutomationCycleDetails, ActiveAutomationRegistryConfigV2, AutomationRegistryV2 {
        if (!is_feature_enabled_and_initialized() || !features::supra_automation_v2_enabled()) {
            return
        };
        assert_automation_cycle_management_support();
        let cycle_info = borrow_global_mut<AutomationCycleDetails>(@supra_framework);
        if (cycle_info.state != CYCLE_STARTED
            || cycle_info.start_time + cycle_info.duration_secs > timestamp::now_seconds()) {
            return
        };
        on_cycle_end_internal(cycle_info)
    }

    /// On new epoch will be triggered for automation registry caused by `supra_governance::reconfiguration` or DKG finalization
    /// to update the automation registry state depending on SUPRA_NATIVE_AUTOMATION feature flag state.
    ///
    /// If registry is not fully initialized nothing is done.
    ///
    /// If native automation feature is disabled and automation cycle in CYCLE_STARTED state,
    /// then automation lifecycle is suspended immediately. And detached managment will
    /// initiate reprocessing of the available tasks which will end up in refund and cealnup actions.
    ///
    /// Otherwise suspention is postponed untill the end of the transition state.
    ///
    /// Nothing will be done if automation cycle was already suspneded, i.e. in CYCLE_READY state.
    ///
    /// If native automation feature is enabled and automation lifecycle has been in CYCLE_READY state,
    /// then lifecycle is restarted.
    public(friend) fun on_new_epoch() acquires AutomationCycleDetails, ActiveAutomationRegistryConfigV2, AutomationRegistryV2 {
        if (!is_initialized() || !features::supra_automation_v2_enabled()) {
            return
        };
        let cycle_info = borrow_global_mut<AutomationCycleDetails>(@supra_framework);
        let general_registry_data = &borrow_global<AutomationRegistryV2>(@supra_framework).main;
        if (features::supra_native_automation_enabled()) {
            // If the lifecycle has been suspended and we are recovering from it, then we update config from buffer and
            // then start a new cycle directly.
            // Unless we are in CYCLE_READY state, the feature flag being enabled will not have any effect.
            // All the other states mean that we are in the middle of previous transition, which should end
            // before reenabling the feature.
            if (cycle_info.state == CYCLE_READY) {
                if (enumerable_map::length(&general_registry_data.tasks) != 0) {
                    event::emit(ErrorInconsistentSuspendedState {});
                    return
                };
                update_config_from_buffer(cycle_info);
                move_to_started_state(cycle_info);
            };
            return
        };

        // We do not update config here, as due to feature being disabled, cycle ends early so it is expected
        // that the current fee-parameters will be used to calculate automation-fee for refund for a cycle
        // that has been kept short.
        // So the confing should remain intact.
        if (cycle_info.state == CYCLE_STARTED) {
            try_move_to_suspended_state(general_registry_data, cycle_info);
        } else if (cycle_info.state == CYCLE_FINISHED && std::option::is_some(&cycle_info.transition_state)) {
            let trasition_state = std::option::borrow(&cycle_info.transition_state);
            if (!is_transition_in_progress(trasition_state)) {
                // Just entered cycle-end phase, and meanwhile also feature has been disabled so it is safe to move to suspended state.
                try_move_to_suspended_state(general_registry_data, cycle_info);
            }
            // Otherwise wait of the cycle transition to end and then feature flag value will be taken into account.
        }
        // If in already SUSPENED state or in READY state then do nothing.
    }

    // Private Native VM referenced api

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
    ) acquires AutomationRegistryV2, AutomationCycleDetails, ActiveAutomationRegistryConfigV2, AutomationRefundBookkeeping {
        // Guarding registration if feature is not enabled.
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        let has_no_priority = check_and_validate_aux_data(&aux_data, UST);

        let automation_registry_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let automation_cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(automation_registry_config.registration_enabled, ETASK_REGISTRATION_DISABLED);
        assert!(automation_cycle_info.state == CYCLE_STARTED, ECYCLE_TRANSITION_IN_PROGRESS);

        // If registry is full, reject task registration
        assert!((get_task_count() as u16) < automation_registry_config.main_config.task_capacity, EREGISTRY_IS_FULL);

        let owner = signer::address_of(owner_signer);
        let automation_registry = &mut borrow_global_mut<AutomationRegistryV2>(@supra_framework).main;

        //Well-formedness check of payload_tx is done in native layer beforehand.

        let registration_time = timestamp::now_seconds();
        validate_task_duration(
            expiry_time,
            registration_time,
            automation_registry_config,
            automation_cycle_info,
            UST
        );

        assert!(gas_price_cap > 0, EINVALID_GAS_PRICE);
        assert!(max_gas_amount > 0, EINVALID_MAX_GAS_AMOUNT);
        assert!(vector::length(&tx_hash) == TXN_HASH_LENGTH, EINVALID_TXN_HASH);

        let committed_gas = (automation_registry.gas_committed_for_next_epoch as u128) + (max_gas_amount as u128);
        assert!(committed_gas <= MAX_U64, EGAS_COMMITTEED_VALUE_OVERFLOW);

        let committed_gas = (committed_gas as u64);
        assert!(committed_gas <= automation_registry_config.next_cycle_registry_max_gas_cap, EGAS_AMOUNT_UPPER);

        // Check the automation fee capacity
        let estimated_automation_fee_for_epoch = estimate_automation_fee_with_committed_occupancy_internal(
            max_gas_amount,
            automation_registry.gas_committed_for_next_epoch,
            automation_cycle_info.duration_secs,
            automation_registry_config);
        assert!(automation_fee_cap_for_epoch >= estimated_automation_fee_for_epoch,
            EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH
        );

        automation_registry.gas_committed_for_next_epoch = committed_gas;
        let task_index = automation_registry.current_index;

        if (has_no_priority) {
            let priority = std::bcs::to_bytes(&task_index);
            vector_utils::replace(&mut aux_data, PRIORITY_AUX_DATA_INDEX, priority);
        };

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
            locked_fee_for_next_epoch: automation_fee_cap_for_epoch
        };

        enumerable_map::add_value(&mut automation_registry.tasks, task_index, automation_task_metadata);
        automation_registry.current_index = automation_registry.current_index + 1;

        // Charge flat registration fee from the user at the time of registration and deposit for automation_fee for epoch.
        let fee = automation_registry_config.main_config.flat_registration_fee_in_quants + automation_fee_cap_for_epoch;

        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);
        refund_bookkeeping.total_deposited_automation_fee = refund_bookkeeping.total_deposited_automation_fee + automation_fee_cap_for_epoch;

        coin::transfer<SupraCoin>(owner_signer, automation_registry.registry_fee_address, fee);

        event::emit(TaskRegistrationDepositFeeWithdraw {
            task_index,
            owner,
            registration_fee: automation_registry_config.main_config.flat_registration_fee_in_quants ,
            locked_deposit_fee: automation_fee_cap_for_epoch
        });
        event::emit(automation_task_metadata);
    }

    /// Registers a new system automation task entry.
    /// Note, system tasks are not charged registration and deposit fee.
    fun register_system_task(
        owner_signer: &signer,
        payload_tx: vector<u8>,
        expiry_time: u64,
        max_gas_amount: u64,
        tx_hash: vector<u8>,
        aux_data: vector<vector<u8>>
    ) acquires AutomationRegistryV2, AutomationCycleDetails, ActiveAutomationRegistryConfigV2 {
        // Guarding registration if feature is not enabled.
        assert!(features::supra_native_automation_enabled(), EDISABLED_AUTOMATION_FEATURE);
        let has_no_priority = check_and_validate_aux_data(&aux_data, GST);

        let automation_registry_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let automation_cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(automation_registry_config.registration_enabled, ETASK_REGISTRATION_DISABLED);
        assert!(automation_cycle_info.state == CYCLE_STARTED, ECYCLE_TRANSITION_IN_PROGRESS);

        // If registry is full, reject task registration
        assert!((get_system_task_count() as u16) < automation_registry_config.system_task_config.task_capacity, EREGISTRY_IS_FULL);

        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);

        let owner = signer::address_of(owner_signer);
        assert!(vector::contains(&automation_registry.system_tasks_state.authorized_accounts, &owner),
            EUNAUTHORIZED_SYSTEM_ACCOUNT
        );

        //Well-formedness check of payload_tx is done in native layer beforehand.

        let registration_time = timestamp::now_seconds();
        validate_task_duration(
            expiry_time,
            registration_time,
            automation_registry_config,
            automation_cycle_info,
            GST
        );

        assert!(max_gas_amount > 0, EINVALID_MAX_GAS_AMOUNT);
        assert!(vector::length(&tx_hash) == TXN_HASH_LENGTH, EINVALID_TXN_HASH);

        let committed_gas = (automation_registry.system_tasks_state.gas_committed_for_next_cycle as u128) + (max_gas_amount as u128);
        assert!(committed_gas <= MAX_U64, EGAS_COMMITTEED_VALUE_OVERFLOW);

        let committed_gas = (committed_gas as u64);
        assert!(committed_gas <= automation_registry_config.next_cycle_sys_registry_max_gas_cap, EGAS_AMOUNT_UPPER);

        automation_registry.system_tasks_state.gas_committed_for_next_cycle = committed_gas;
        let task_index = automation_registry.main.current_index;
        if (has_no_priority) {
            let priority = std::bcs::to_bytes(&task_index);
            vector_utils::replace(&mut aux_data, PRIORITY_AUX_DATA_INDEX, priority);
        };

        let automation_task_metadata = AutomationTaskMetaData {
            task_index,
            owner,
            payload_tx,
            expiry_time,
            max_gas_amount,
            // No max gas price, as system tasks are not charged
            gas_price_cap: 0,
            // No Automation fee cap, as system tasks are not charged
            automation_fee_cap_for_epoch: 0,
            aux_data,
            state: PENDING,
            registration_time,
            tx_hash,
            // No deposit fee as system tasks are not charged
            locked_fee_for_next_epoch: 0
        };

        enumerable_map::add_value(&mut automation_registry.main.tasks, task_index, automation_task_metadata);
        automation_registry.main.current_index = automation_registry.main.current_index + 1;
        vector::push_back(&mut automation_registry.system_tasks_state.task_ids, task_index);

        event::emit(automation_task_metadata);
    }


    /// Called by MoveVm on `AutomationBookkeepingAction::Process` action emitted by native layer ahead of cycle transition
    fun process_tasks(
        vm: signer,
        cycle_index: u64,
        task_indexes: vector<u64>
    ) acquires AutomationCycleDetails, AutomationRegistryV2, AutomationRefundBookkeeping, ActiveAutomationRegistryConfigV2 {
        // Operational constraint: can only be invoked by the VM
        system_addresses::assert_vm(&vm);
        let cycle_info = borrow_global<AutomationCycleDetails>(@supra_framework);
        if (cycle_info.state == CYCLE_FINISHED) {
            on_cycle_transition(cycle_index, task_indexes);
            return
        };
        assert!(cycle_info.state == CYCLE_SUSPENDED, EINVALID_REGISTRY_STATE);
        on_cycle_suspend(cycle_index, task_indexes);
    }

    // Private helper functions

    /// Traverses the list of the tasks and based on the task state and expiry information either charges or drops
    /// the task after refunding eligable fees.
    ///
    /// Input cycle index corresponds to the new cycle to which the transition is being done.
    ///
    /// Tasks are cheked not to be processed more than once.
    /// This function should be called only if registry is in CYCLE_FINISHED state, meaning a normal cycle transition is
    /// happening.
    ///
    /// After processing all input tasks, intermediate transition state is updated and transition end is check
    /// (whether all expected tasks has been processed already).
    ///
    /// In case if transition end is detected a start of the new cycle is given
    /// (if during trasition period suspention is not requested) and corresponding event is emitted.
    fun on_cycle_transition(cycle_index: u64, task_indexes: vector<u64>)
    acquires AutomationCycleDetails, AutomationRefundBookkeeping, AutomationRegistryV2, ActiveAutomationRegistryConfigV2 {
        if (vector::is_empty(&task_indexes)) {
            return
        };

        let cycle_info = borrow_global_mut<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_info.state == CYCLE_FINISHED, EINVALID_REGISTRY_STATE);
        assert!(std::option::is_some(&cycle_info.transition_state), EINVALID_REGISTRY_STATE);
        assert!(cycle_info.index + 1 == cycle_index, EINVALID_INPUT_CYCLE_INDEX);

        let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);

        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);
        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);
        let automation_registry_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let intermedate_result = IntermediateStateOfCycleChange {
            removed_tasks: vector[],
            gas_committed_for_next_cycle: 0,
            sys_gas_committed_for_next_cycle: 0,
            epoch_locked_fees: coin::zero()
        };

        drop_or_charge_tasks(
            task_indexes,
            automation_registry,
            refund_bookkeeping,
            transition_state,
            &automation_registry_config.main_config,
            timestamp::now_seconds(),
            &mut intermedate_result
        );
        let IntermediateStateOfCycleChange {
            removed_tasks,
            gas_committed_for_next_cycle,
            sys_gas_committed_for_next_cycle,
            epoch_locked_fees
        } = intermedate_result;

        transition_state.locked_fees = transition_state.locked_fees + coin::value(&epoch_locked_fees);
        transition_state.gas_committed_for_next_cycle = transition_state.gas_committed_for_next_cycle + gas_committed_for_next_cycle;
        transition_state.sys_gas_committed_for_next_cycle = transition_state.sys_gas_committed_for_next_cycle + sys_gas_committed_for_next_cycle;
        coin::deposit(automation_registry.main.registry_fee_address, epoch_locked_fees);

        update_cycle_transition_state_from_finished(automation_registry, cycle_info);

        if (!vector::is_empty(&removed_tasks)) {
            event::emit(RemovedTasks{
                task_indexes: removed_tasks
            })
        }
    }

    /// Traverses the list of the tasks and refunds automation(if not PENDING) and depoist fees for all tasks
    /// and removes from registry.
    ///
    /// Input cycle index corresponds to the cycle being suspended.
    ///
    /// This function is called only if automation feature is disabled, i.e. CYCLE_SUSPENDED state.
    ///
    /// After processing input set of tasks the end of suspention process is checked(i.e. all expected tasks has been processed).
    /// In case if end is identified the registry state is update to CYCLE_READY and corresponding event is emitted.
    fun on_cycle_suspend(cycle_index: u64, task_indexes: vector<u64> )
    acquires AutomationCycleDetails, AutomationRefundBookkeeping, AutomationRegistryV2, ActiveAutomationRegistryConfigV2 {

        if (vector::is_empty(&task_indexes)) {
            return
        };

        let cycle_info = borrow_global_mut<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_info.state == CYCLE_SUSPENDED, EINVALID_REGISTRY_STATE);
        assert!(std::option::is_some(&cycle_info.transition_state), EINVALID_REGISTRY_STATE);
        assert!(cycle_info.index == cycle_index, EINVALID_INPUT_CYCLE_INDEX);
        let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);


        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);
        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);
        let arc = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let current_time = timestamp::now_seconds();

        let resource_signer = account::create_signer_with_capability(
            &automation_registry.main.registry_fee_address_signer_cap
        );
        let removed_tasks = vector[];
        let epoch_locked_fees = automation_registry.main.epoch_locked_fees;
        // Sort task indexes as order is important
        task_indexes = sort_vector_u64(task_indexes);
        vector::for_each(task_indexes, |task_index| {
            if (enumerable_map::contains(&automation_registry.main.tasks, task_index)) {
                let task = enumerable_map::remove_value(&mut automation_registry.main.tasks, task_index);
                mark_task_processed(transition_state, task_index);
                // Nothing to refund for GST tasks
                if (is_of_type(&task, UST)) {
                    epoch_locked_fees = refund_task_fees(
                        task,
                        &mut automation_registry.main,
                        refund_bookkeeping,
                        arc,
                        transition_state,
                        &resource_signer,
                        epoch_locked_fees,
                        current_time,
                        &mut removed_tasks
                    )
                };
            }
        });

        update_cycle_transition_state_from_suspended(automation_registry, cycle_info);
        event::emit(RemovedTasks {
            task_indexes: removed_tasks
        });
    }

    /// Traverses all input task indexes and either drops or tries to charge automation fee if possible.
    fun drop_or_charge_tasks(
        task_ids: vector<u64>,
        automation_registry: &mut AutomationRegistryV2,
        refund_bookkeeping: &mut AutomationRefundBookkeeping,
        transition_state: &mut TransitionState,
        arc: &AutomationRegistryConfig,
        current_time: u64,
        intermediate_state: &mut IntermediateStateOfCycleChange,
    ) {

        let resource_signer = account::create_signer_with_capability(
            &automation_registry.main.registry_fee_address_signer_cap
        );
        let current_cycle_end_time = current_time + transition_state.new_cycle_duration;

        // Sort task indexes to charge automation fees in the tasks chronological order
        task_ids = sort_vector_u64(task_ids);

        // Process each active task and calculate fee for the epoch for the tasks
        vector::for_each(task_ids, |task_index| {
            drop_or_charge_task(
                task_index,
                automation_registry,
                refund_bookkeeping,
                arc,
                transition_state,
                &resource_signer,
                current_time,
                current_cycle_end_time,
                intermediate_state
            )
        });
    }

    /// Drops or charges the input task.
    /// If the task is already processed or missing from the registry then nothing is done.
    fun drop_or_charge_task(
        task_index: u64,
        automation_registry: &mut AutomationRegistryV2,
        refund_bookkeeping: &mut AutomationRefundBookkeeping,
        arc: &AutomationRegistryConfig,
        transition_state: &mut TransitionState,
        resource_signer: &signer,
        current_time: u64,
        current_cycle_end_time: u64,
        intermediate_state: &mut IntermediateStateOfCycleChange,
    )
    {
        if (!enumerable_map::contains(&automation_registry.main.tasks, task_index)) {
            return
        };
        mark_task_processed(transition_state, task_index);
        let task_meta = enumerable_map::get_value_mut(&mut automation_registry.main.tasks, task_index);
        if (task_meta.state == CANCELLED || task_meta.expiry_time <= current_time) {
            if (is_of_type(task_meta, UST)) {
                refund_deposit_and_drop(task_index, automation_registry, refund_bookkeeping, resource_signer, &mut intermediate_state.removed_tasks);
            } else {
                drop_system_task(task_index, automation_registry, &mut intermediate_state.removed_tasks)
            };
            return
        } else if (is_of_type(task_meta, GST)) {
            // Governance submitted tasks are not charged
            intermediate_state.sys_gas_committed_for_next_cycle = intermediate_state.sys_gas_committed_for_next_cycle + task_meta.max_gas_amount;
            task_meta.state = ACTIVE;
            return
        };

        let fee= calculate_task_fee(
            arc,
            task_meta,
            transition_state.new_cycle_duration,
            current_time,
            (transition_state.automation_fee_per_sec as u256));
        // If the task reached this phase that means it is valid active task for the new epoch.
        // During cleanup all expired tasks has been removed from the registry but the state of the tasks is not updated.
        // As here we need to distinguish new tasks from already existing active tasks,
        // as the fee calculation for them will be different based on their active duration in the epoch.
        // For more details see calculate_task_fee function.
        task_meta.state = ACTIVE;
        let task = AutomationTaskFeeMeta {
            task_index,
            owner: task_meta.owner,
            fee,
            expiry_time: task_meta.expiry_time,
            automation_fee_cap: task_meta.automation_fee_cap_for_epoch,
            max_gas_amount: task_meta.max_gas_amount,
            locked_deposit_fee: task_meta.locked_fee_for_next_epoch,
        };
        try_withdraw_task_automation_fee(
            automation_registry,
            refund_bookkeeping,
            resource_signer,
            task,
            current_cycle_end_time,
            intermediate_state
        );
    }

    /// Refunds the deposit fee of the task and removes from registry.
    fun refund_deposit_and_drop(
        task_index: u64,
        automation_registry: &mut AutomationRegistryV2,
        refund_bookkeeping: &mut AutomationRefundBookkeeping,
        resource_signer: &signer,
        removed_tasks: &mut vector<u64> ) : AutomationTaskMetaData {
        let task = enumerable_map::remove_value(&mut automation_registry.main.tasks, task_index);
        assert!(is_of_type(&task, UST), EREGISTERED_TASK_INVALID_TYPE);
        safe_deposit_refund(
            refund_bookkeeping,
            resource_signer,
            automation_registry.main.registry_fee_address,
            task_index,
            task.owner,
            task.locked_fee_for_next_epoch,
            task.locked_fee_for_next_epoch);
        vector::push_back(removed_tasks, task_index);
        task
    }

    /// Removes system task from registry state.
    fun drop_system_task(
        task_index: u64,
        automation_registry: &mut AutomationRegistryV2,
        removed_tasks: &mut vector<u64>
    )  {
        let task = enumerable_map::remove_value(&mut automation_registry.main.tasks, task_index);
        assert!(is_of_type(&task, GST), EREGISTERED_TASK_INVALID_TYPE);
        vector::remove_value(&mut automation_registry.system_tasks_state.task_ids, &task.task_index);
        vector::push_back(removed_tasks, task_index);
    }

    /// Refunds the deposit fee and any autoamtion fees of the task.
    fun refund_task_fees(
        task: AutomationTaskMetaData,
        automation_registry: &mut AutomationRegistry,
        refund_bookkeeping: &mut AutomationRefundBookkeeping,
        arc: &ActiveAutomationRegistryConfigV2,
        transition_state: &mut TransitionState,
        resource_signer: &signer,
        epoch_locked_fees: u64,
        current_time: u64,
        removed_tasks: &mut vector<u64>

    ) : u64 {
        assert!(is_of_type(&task, UST), EREGISTERED_TASK_INVALID_TYPE);
        // Do not attempt fee refund if remaining duration is 0
        if (task.state != PENDING && transition_state.refund_duration != 0) {
            let refund = calculate_task_fee(
                &arc.main_config,
                &task,
                transition_state.refund_duration,
                current_time,
                (transition_state.automation_fee_per_sec as u256));
            let (_, remaining_epoch_locked_fees) = safe_fee_refund(
                epoch_locked_fees,
                resource_signer,
                automation_registry.registry_fee_address,
                task.task_index,
                task.owner,
                refund);
            epoch_locked_fees = remaining_epoch_locked_fees;
        };

        safe_deposit_refund(
            refund_bookkeeping,
            resource_signer,
            automation_registry.registry_fee_address,
            task.task_index,
            task.owner,
            task.locked_fee_for_next_epoch,
            task.locked_fee_for_next_epoch);
        vector::push_back(removed_tasks, task.task_index);
        epoch_locked_fees
    }

    fun into_automation_cycle_info(details: &AutomationCycleDetails): AutomationCycleInfo {
        AutomationCycleInfo {
            index: details.index,
            state: details.state,
            start_time: details.start_time,
            duration_secs: details.duration_secs
        }
    }

    /// Updates the cycle state if the transition is identified to be finalized.
    ///
    /// As transition happens from suspended state and while transition was in progress
    ///    - if the feature was enabled back, then the transition will happen direclty to starated state,
    ///    - otherwise the transition will be done to the ready state.
    ///
    /// In both cases config will be updated. In this case we will make sure to keep the consistency of state
    /// when transition to ready state happens through paths
    ///  - Started -> Suspended -> Ready
    ///  - or Started-> {Finished, Suspended} -> Ready
    ///  - or Started -> Finished -> {Started, Suspended}
    fun update_cycle_transition_state_from_suspended(
        automation_registry: &mut AutomationRegistryV2,
        cycle_info: &mut AutomationCycleDetails,
    ) acquires  ActiveAutomationRegistryConfigV2  {
        assert!(std::option::is_some(&cycle_info.transition_state), EINVALID_REGISTRY_STATE);
        let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);

        if (!is_transition_finalized(transition_state)) {
            return
        };

        automation_registry.system_tasks_state.gas_committed_for_next_cycle = 0;
        automation_registry.system_tasks_state.gas_committed_for_this_cycle = 0;
        automation_registry.system_tasks_state.task_ids = vector[];

        automation_registry.main.gas_committed_for_next_epoch = 0;
        automation_registry.main.gas_committed_for_this_epoch = 0;
        automation_registry.main.epoch_active_task_ids = vector[];
        automation_registry.main.epoch_locked_fees = 0;

        if (features::supra_native_automation_enabled()) {
            // Update the config in case if transition flow is STARTED -> SUSPENDED-> STARTED.
            // to reflect new configs for the new cycle if it has been updated during SUSPENDED state processing
            update_config_from_buffer(cycle_info);
            move_to_started_state(cycle_info)
        } else {
            move_to_ready_state(cycle_info)
        }
    }

    /// Updates the cycle state if the transition is identified to be finalized.
    ///
    /// From CYCLE_FINALIZED state we always move to the next cycle and in CYCLE_STARTED state.
    ///
    /// But if it happened so that there was a suspension during cycle transition which was ignored,
    /// then immediately cycle state is updated to suspended.
    ///
    /// Expectation will be that native layer catches this double transition and issues refunds for the new cycle fees
    /// which will not proceeded farther in any case.
    fun update_cycle_transition_state_from_finished(
        automation_registry: &mut AutomationRegistryV2,
        cycle_info: &mut AutomationCycleDetails,
    ) acquires ActiveAutomationRegistryConfigV2 {
        assert!(std::option::is_some(&cycle_info.transition_state), EINVALID_REGISTRY_STATE);

        let transition_state = std::option::borrow(&cycle_info.transition_state);
        let transition_finalized = is_transition_finalized(transition_state);

        if (!transition_finalized) {
            return
        };

        automation_registry.system_tasks_state.gas_committed_for_next_cycle = transition_state.sys_gas_committed_for_next_cycle;
        automation_registry.system_tasks_state.gas_committed_for_this_cycle = transition_state.sys_gas_committed_for_next_cycle;

        automation_registry.main.gas_committed_for_next_epoch = transition_state.gas_committed_for_next_cycle;
        automation_registry.main.gas_committed_for_this_epoch = (transition_state.gas_committed_for_new_cycle as u256);
        automation_registry.main.epoch_active_task_ids = enumerable_map::get_map_list(&automation_registry.main.tasks);
        automation_registry.main.epoch_locked_fees = transition_state.locked_fees;

        // Set current timestamp as cycle start_time
        // Increase cycle and update the state to Started
        move_to_started_state(cycle_info);
        if (!vector::is_empty(&automation_registry.main.epoch_active_task_ids)) {
            event::emit(ActiveTasks {
                task_indexes: automation_registry.main.epoch_active_task_ids
            });
        };
        if (!features::supra_native_automation_enabled()) {
            try_move_to_suspended_state(&automation_registry.main, cycle_info)
        }
    }

    /// Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
    /// referencing the current automation registry fee parameters, specified total/committed occupancy and registry
    /// maximum allowed occupancy for the next epoch.
    /// Note it is expected that committed_occupancy does not include currnet task's occupancy.
    fun estimate_automation_fee_with_committed_occupancy_internal(
        task_occupancy: u64,
        committed_occupancy: u64,
        duration: u64,
        active_config: &ActiveAutomationRegistryConfigV2
    ): u64 {
        let total_committed_max_gas = committed_occupancy + task_occupancy;

        // Compute the automation fee multiplier for epoch
        let automation_fee_per_sec = calculate_automation_fee_multiplier_for_epoch(
            &active_config.main_config,
            (total_committed_max_gas as u256),
            active_config.next_cycle_registry_max_gas_cap);

        if (automation_fee_per_sec == 0) {
            return 0
        };

        calculate_automation_fee_for_interval(
            duration,
            task_occupancy,
            automation_fee_per_sec,
            active_config.next_cycle_registry_max_gas_cap)
    }

    fun validate_configuration_parameters_common(
        cycle_duration_secs: u64,
        task_duration_cap_in_secs: u64,
        sys_task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        sys_registry_max_gas_cap: u64,
        congestion_threshold_percentage: u8,
        congestion_exponent: u8,
    ) {
        assert!(cycle_duration_secs > 0, ECYCLE_DURATION_NON_ZERO);
        assert!(congestion_threshold_percentage <= MAX_PERCENTAGE, EMAX_CONGESTION_THRESHOLD);
        assert!(congestion_exponent > 0, ECONGESTION_EXP_NON_ZERO);
        assert!(task_duration_cap_in_secs > cycle_duration_secs, EUNACCEPTABLE_TASK_DURATION_CAP);
        assert!(sys_task_duration_cap_in_secs > cycle_duration_secs, EUNACCEPTABLE_SYS_TASK_DURATION_CAP);
        assert!(registry_max_gas_cap > 0, EREGISTRY_MAX_GAS_CAP_NON_ZERO);
        assert!(sys_registry_max_gas_cap > 0, EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO);
    }

    fun validate_system_configuration_parameters_common(
        cycle_duration_secs: u64,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
    ) {
        assert!(task_duration_cap_in_secs > cycle_duration_secs, EUNACCEPTABLE_SYS_TASK_DURATION_CAP);
        assert!(registry_max_gas_cap > 0, EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS);
    }

    fun create_registry_resource_account(supra_framework: &signer): (signer, SignerCapability) {
        let (registry_fee_resource_signer, registry_fee_address_signer_cap) = account::create_resource_account(
            supra_framework,
            REGISTRY_RESOURCE_SEED
        );
        coin::register<SupraCoin>(&registry_fee_resource_signer);
        (registry_fee_resource_signer, registry_fee_address_signer_cap)
    }

    fun on_cycle_end_internal(cycle_info: &mut AutomationCycleDetails) acquires ActiveAutomationRegistryConfigV2, AutomationRegistryV2 {
        let automation_registry = &borrow_global<AutomationRegistryV2>(@supra_framework).main;
        if (enumerable_map::length(&automation_registry.tasks) == 0) {
            // Registry is empty update config-buffer and move to started state directly
            update_config_from_buffer(cycle_info);
            move_to_started_state(cycle_info);
            return
        };
        let expected_tasks_to_be_processed = enumerable_map::get_map_list(&automation_registry.tasks);
        expected_tasks_to_be_processed = sort_vector_u64(expected_tasks_to_be_processed);
        let transition_state = TransitionState {
            refund_duration: 0,
            new_cycle_duration: cycle_info.duration_secs,
            automation_fee_per_sec: 0,
            gas_committed_for_new_cycle: automation_registry.gas_committed_for_next_epoch,
            gas_committed_for_next_cycle: 0,
            sys_gas_committed_for_next_cycle: 0,
            locked_fees: 0,
            expected_tasks_to_be_processed,
            next_task_index_position: 0
        };
        cycle_info.transition_state = std::option::some(transition_state);
        // During cycle transition we update config only after transition state is created in order to have new cycle
        // duration as transition state parameter.
        update_config_from_buffer(cycle_info);
        // Calculate automation fee per second for the new epoch only after configuration is updated.
        let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);
        // As we already know the committed gas for the new cycle it is being calculated using updated fee-parameters
        // and will be used to charge tasks during transition process.
        transition_state.automation_fee_per_sec =
            calculate_automation_fee_multiplier_for_committed_occupancy(transition_state.gas_committed_for_new_cycle);
        update_cycle_state_to(cycle_info, CYCLE_FINISHED);
    }

    fun update_cycle_state_to(cycle_info: &mut AutomationCycleDetails, state: u8) {
        let old_state = cycle_info.state;
        cycle_info.state = state;
        let event = AutomationCycleEvent {
            cycle_state_info: into_automation_cycle_info(cycle_info),
            old_state,
        };
        event::emit(event)
    }

    fun move_to_ready_state(cycle_info: &mut AutomationCycleDetails) {
        // If the cycle duration updated has been identified during transtion, then the transition state is kept
        // with reset values except new cycle duration to have it properly set for the next new cycle.
        // This may happen in case of cycle was ended and feature-flag has been disbaled before any task has
        // been processed for the cycle transition.
        // Note that we want to have consistent data in ready state which says that the cycle pointed in the ready state
        // has been finished/summerized, and we are ready to start the next new cycle. and all the cycle inforamation should
        // match the finalized/summerized cycle since its start, including cycle duration
        if (std::option::is_some(&cycle_info.transition_state)) {
            let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);
            if (transition_state.new_cycle_duration == cycle_info.duration_secs) {
                cycle_info.transition_state = std::option::none<TransitionState>();
            } else {
                // Reset all except new cycle duration
                transition_state.refund_duration = 0;
                transition_state.automation_fee_per_sec = 0;
                transition_state.gas_committed_for_new_cycle = 0;
                transition_state.gas_committed_for_next_cycle = 0;
                transition_state.sys_gas_committed_for_next_cycle = 0;
                transition_state.locked_fees = 0;
                transition_state.expected_tasks_to_be_processed = vector[];
                transition_state.next_task_index_position = 0;
            }
        };
        update_cycle_state_to(cycle_info, CYCLE_READY)
    }

    fun move_to_started_state(cycle_info: &mut AutomationCycleDetails) {
        cycle_info.index = cycle_info.index + 1;
        cycle_info.start_time = timestamp::now_seconds();
        if (std::option::is_some(&cycle_info.transition_state)) {
            let transition_state = std::option::extract(&mut cycle_info.transition_state);
            cycle_info.duration_secs = transition_state.new_cycle_duration;
        };
        update_cycle_state_to(cycle_info, CYCLE_STARTED)
    }

    /// Transition to suspended state is expected to be called
    ///   a) when cycle is active and in progress
    ///     - here we simply move to suspended state so native layer can start requesting tasks processing
    ///       which will end up in  refunds and cleanup. Note that refund will be done based on total gas-committed
    ///       for the current cycle defined at the begining for the cycle, and using current automation fee parameters
    ///   b) when cycle has just finished and there was another transaction causing feature suspension
    ///     - as this both events happen in scope of the same block, then we will simply update the state to suspended
    ///       and the native layer should identify the transition and request processing of the all available tasks.
    ///       Note that in this case automation fee refund will not be expected and suspention and cycle end matched and
    ///       no fee was yet charged to be refunded.
    ///       So the duration for refund and automation-fee-per-second for refund will be 0
    ///   c) when cycle transition was in progress and there was a feature suspension, but it could not be applied,
    ///      and postponed till the cycle transition concludes
    /// In all cases if there are no tasks in registry the state will be updated directly to CYCLE_READY state.
    fun try_move_to_suspended_state(automation_registry: &AutomationRegistry, cycle_info: &mut AutomationCycleDetails
    ) acquires  ActiveAutomationRegistryConfigV2 {
        if (enumerable_map::length(&automation_registry.tasks) == 0) {
            // Registry is empty move to ready state directly
            // move_to_ready_state(cycle_info);
            update_cycle_state_to(cycle_info, CYCLE_READY);
            return
        };
        if (std::option::is_none(&cycle_info.transition_state)) {
            // Indicates that cycle was in STARTED state when suspention has been identified.
            // It is safe to assert that cycle_end_time will always be greater than current chain time as
            // the cycle end is check in the block metadata txn execution which proceeds any other transaction in the block.
            // Including the transaction which caused transition to suspended state.
            // So in case if cycle_end_time < current_time then cycle end would have been identified
            // and we would have enterend else branch instead.
            // This holds true even if we identified suspention when moving from FINALIZED->STARTED state.
            // As in this case we will first transition to the STARTED state and only then to SUSPENDED.
            // And when transition to STARTED state we update the cycle start-time to be the current-chain-time.
            let current_time = timestamp::now_seconds();
            let cycle_end_time = cycle_info.start_time + cycle_info.duration_secs;
            assert!(current_time >= cycle_info.start_time, EINVALID_REGISTRY_STATE);
            assert!(current_time < cycle_end_time, EINVALID_REGISTRY_STATE);
            assert!(cycle_info.state == CYCLE_STARTED, EINVALID_REGISTRY_STATE);
            let active_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
            let expected_tasks_to_be_processed = enumerable_map::get_map_list(&automation_registry.tasks);
            expected_tasks_to_be_processed = sort_vector_u64(expected_tasks_to_be_processed);
            let transition_state = TransitionState {
                refund_duration: cycle_end_time - current_time,
                new_cycle_duration: cycle_info.duration_secs,
                automation_fee_per_sec: calculate_automation_fee_multiplier_for_current_cycle_internal(active_config, automation_registry),
                gas_committed_for_new_cycle: 0,
                gas_committed_for_next_cycle: 0,
                sys_gas_committed_for_next_cycle: 0,
                locked_fees: 0,
                expected_tasks_to_be_processed,
                next_task_index_position: 0
            };
            cycle_info.transition_state = std::option::some(transition_state);
        } else {
            assert!(cycle_info.state == CYCLE_FINISHED, EINVALID_REGISTRY_STATE);
            let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);
            assert!(!is_transition_in_progress(transition_state), EINVALID_REGISTRY_STATE);
            // Did not manage to charge cycle fee, so automation_fee_per_sec be 0 along with remaining duration
            // So the tasks sent for refund, will get only deposit refunded.
            transition_state.refund_duration = 0;
            transition_state.automation_fee_per_sec = 0;
            transition_state.gas_committed_for_new_cycle = 0;
        };
        update_cycle_state_to(cycle_info, CYCLE_SUSPENDED)
    }

    /// Refunds automation fee for epoch for all eligible tasks and clears automation registry state in terms of
    /// fee primitives.
    fun update_state_for_migration(
        automation_registry: &mut AutomationRegistry,
        arc: &AutomationRegistryConfig,
        aei: AutomationEpochInfo,
        current_time: u64
    ) {
        let AutomationEpochInfo {
            start_time,
            epoch_interval: _,
            expected_epoch_duration,

        } = aei;
        let previous_epoch_duration = current_time - start_time;
        let refund_interval = 0;
        let refund_automation_fee_per_sec = 0;

        // If epoch actual duration is greater or equal to expected epoch-duration then there is nothing to refund.
        if (automation_registry.epoch_locked_fees != 0 && previous_epoch_duration < expected_epoch_duration) {
            let previous_tcmg = automation_registry.gas_committed_for_this_epoch;
            refund_interval = expected_epoch_duration - previous_epoch_duration;
            // Compute the automation fee multiplier for ended epoch
            refund_automation_fee_per_sec = calculate_automation_fee_multiplier_for_epoch(arc, previous_tcmg, arc.registry_max_gas_cap);
        };
        refund_fees_and_update_tasks(automation_registry, arc, refund_automation_fee_per_sec, refund_interval, current_time);

        automation_registry.epoch_locked_fees = 0;
        automation_registry.gas_committed_for_this_epoch = 0;
    }

    /// Refunds automation fee for epoch for all eligible tasks during migration.
    fun refund_fees_and_update_tasks(
        automation_registry: &mut AutomationRegistry,
        arc: &AutomationRegistryConfig,
        refund_automation_fee_per_sec: u256,
        refund_interval: u64,
        current_time: u64)
    {
        let ids = enumerable_map::get_map_list(&automation_registry.tasks);

        let resource_signer = account::create_signer_with_capability(
            &automation_registry.registry_fee_address_signer_cap
        );
        let epoch_locked_fees = automation_registry.epoch_locked_fees;

        vector::for_each(ids, |task_index| {
            let task = enumerable_map::get_value_mut(&mut automation_registry.tasks, task_index);
            // Defult type before migration is UST and the priority is the task index
            task.aux_data = vector[vector[UST], bcs::to_bytes(&task_index)];
            if (refund_automation_fee_per_sec != 0 && task.state != PENDING) {
                let refund = calculate_task_fee(
                    arc,
                    task,
                    refund_interval,
                    current_time,
                    refund_automation_fee_per_sec);
                let (_, remaining_epoch_locked_fees) = safe_fee_refund(
                    epoch_locked_fees,
                    &resource_signer,
                    automation_registry.registry_fee_address,
                    task.task_index,
                    task.owner,
                    refund);
                epoch_locked_fees = remaining_epoch_locked_fees;
            };
        });
    }

    /// Refunds specified amount of deposit to the task owner and unlocks full deposit from registry resource account.
    /// Error events are emitted
    ///   - if the registry resource account does not have enough balance for refund.
    ///   - if the full deposit can not be unlocked.
    fun safe_deposit_refund(
        rb: &mut AutomationRefundBookkeeping,
        resource_signer: &signer,
        resource_address: address,
        task_index: u64,
        task_owner: address,
        refundable_deposit: u64,
        locked_deposit: u64
    ):  bool {
        // This check will make sure that no more than totally locked deposited will be refunded.
        // If there is an attempt then it means implementation bug.
        let result = safe_unlock_locked_deposit(rb, locked_deposit, task_index);
        if (!result) {
            return result
        };

        let result = safe_refund(
            resource_signer,
            resource_address,
            task_index,
            task_owner,
            refundable_deposit,
            DEPOSIT_EPOCH_FEE);

        if (result) {
            event::emit(
                TaskDepositFeeRefund { task_index, owner: task_owner, amount: refundable_deposit }
            );
        };
        result
    }

    /// Unlocks the deposit paid by the task from internal deposit refund bookkeeping state.
    /// Error event is emitted if the deposit refund bookkeeping state is inconsistent with the requested unlock amount.
    fun safe_unlock_locked_deposit(
        rb: &mut AutomationRefundBookkeeping,
        locked_deposit: u64,
        task_index: u64
    ): bool {
        let has_locked_deposit = rb.total_deposited_automation_fee >= locked_deposit;
        if (has_locked_deposit) {
            rb.total_deposited_automation_fee = rb.total_deposited_automation_fee - locked_deposit;
        } else {
            event::emit(
                ErrorUnlockTaskDepositFee { total_registered_deposit: rb.total_deposited_automation_fee, locked_deposit, task_index }
            );
        };
        has_locked_deposit
    }

    /// Unlocks the locked fee paid by the task for epoch.
    /// Error event is emitted if the epoch locked fee amount is inconsistent with the requested unlock amount.
    fun safe_unlock_locked_epoch_fee(
        epoch_locked_fees: u64,
        refundable_fee: u64,
        task_index: u64
    ): (bool, u64) {
        // This check makes sure that more than locked amount of the fees will be not be refunded.
        // Any attempt means internal bug.
        let has_locked_fee = epoch_locked_fees >= refundable_fee;
        if (has_locked_fee) {
            // unlock the refunded amount
            epoch_locked_fees = epoch_locked_fees - refundable_fee;
        } else {
            event::emit(
                ErrorUnlockTaskEpochFee { locked_epoch_fees: epoch_locked_fees, task_index, refund: refundable_fee}
            );
        };
        (has_locked_fee, epoch_locked_fees)
    }

    /// Refunds fee paid by the task for the epoch to the task owner.
    /// Note that here we do not unlock the fee, as on epoch change locked epoch-fees for the ended epoch are
    /// automatically unlocked.
    fun safe_fee_refund(
        epoch_locked_fees: u64,
        resource_signer: &signer,
        resource_address: address,
        task_index: u64,
        task_owner: address,
        refundable_fee: u64
    ):  (bool, u64) {
        let (result, remaining_locked_fees) = safe_unlock_locked_epoch_fee(epoch_locked_fees, refundable_fee, task_index);
        if (!result) {
            return (result, remaining_locked_fees)
        };
        let result = safe_refund(
            resource_signer,
            resource_address,
            task_index,
            task_owner,
            refundable_fee,
            EPOCH_FEE);
        if (result) {
            event::emit(
                TaskFeeRefund { task_index, owner: task_owner, amount: refundable_fee }
            );
        };
        (result, remaining_locked_fees)
    }

    /// Refunds specified amount to the task owner.
    /// Error event is emitted if the resource account does not have enough balance.
    fun safe_refund(
        resource_signer: &signer,
        resource_address: address,
        task_index: u64,
        task_owner: address,
        refundable_amount: u64,
        refund_type: u8
    ):  bool {
        let balance = coin::balance<SupraCoin>(resource_address);
        if (balance < refundable_amount) {
            event::emit(
                ErrorInsufficientBalanceToRefund { refund_type, task_index, owner: task_owner, amount: refundable_amount }
            );
            return false
        };

        coin::transfer<SupraCoin>(resource_signer, task_owner, refundable_amount);
        return true
    }


    /// Calculates automation task fees for a single task at the time of new epoch.
    /// This is supposed to be called only after removing expired task and must not be called for expired task.
    /// It returns calculated task fee for the interval the task will be active.
    fun calculate_task_fee(
        arc: &AutomationRegistryConfig,
        task: &AutomationTaskMetaData,
        potential_fee_timeframe: u64,
        current_time: u64,
        automation_fee_per_sec: u256
    ): u64 {
        if (automation_fee_per_sec == 0) { return 0 };
        if (task.expiry_time <= current_time) { return 0 };
        // Subtraction is safe here, as we already excluded expired tasks
        let task_active_timeframe = task.expiry_time - current_time;
        // If the task is a new task i.e. in Pending state, then it is charged always for
        // the input potential_fee_timeframe(which is epoch-interval),
        // For the new tasks which active-timeframe is less than epoch-interval
        // it would mean it is their first and only epoch and we charge the fee for entire epoch.
        // Note that although the new short tasks are charged for entire epoch, the refunding logic remains the same for
        // them as for the long tasks.
        // This way bad-actors will be discourged to submit small and short tasks with big occupancy by blocking other
        // good-actors register tasks.
        let actual_fee_timeframe = if (task.state == PENDING) {
            potential_fee_timeframe
        } else {
            math64::min(task_active_timeframe, potential_fee_timeframe)
        };
        calculate_automation_fee_for_interval(
            actual_fee_timeframe,
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

    fun calculate_automation_fee_multiplier_for_current_cycle_internal(
        active_config: &ActiveAutomationRegistryConfigV2,
        automation_registry: &AutomationRegistry
    ): u64 {
        // Compute the automation fee multiplier for this cycle
        let multiplier = calculate_automation_fee_multiplier_for_epoch(
            &active_config.main_config,
            automation_registry.gas_committed_for_this_epoch,
            active_config.main_config.registry_max_gas_cap);
        (multiplier as u64)
    }

    /// Calculate automation fee multiplier for epoch. It is measured in quants/sec.
    fun calculate_automation_fee_multiplier_for_epoch(
        arc: &AutomationRegistryConfig,
        tcmg: u256,
        registry_max_gas_cap: u64
    ): u256 {
        let acf = calculate_automation_congestion_fee(arc, tcmg, registry_max_gas_cap);
        acf + (arc.automation_base_fee_in_quants_per_sec as u256)
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
        if (threshold_usage <= threshold_percentage) 0
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

    fun try_withdraw_task_automation_fee(
        automation_registry: &mut AutomationRegistryV2,
        refund_bookkeeping: &mut AutomationRefundBookkeeping,
        resource_signer: &signer,
        task: AutomationTaskFeeMeta,
        current_cycle_end_time: u64,
        intermediate_state: &mut IntermediateStateOfCycleChange) {
        // Remove the automation task if the epoch fee cap is exceeded
        // It might happen that task has been expired by the time charging is being done.
        // This may be caused by the fact that bookkeeping transactions has been withheld due to epoch transition.
        if (task.fee > task.automation_fee_cap) {
            let task_meta = refund_deposit_and_drop(
                task.task_index,
                automation_registry,
                refund_bookkeeping,
                resource_signer,
                &mut intermediate_state.removed_tasks
            );
            event::emit(TaskCancelledCapacitySurpassedV2 {
                task_index: task.task_index,
                owner: task.owner,
                fee: task.fee,
                automation_fee_cap: task.automation_fee_cap,
                registration_hash: task_meta.tx_hash,
            });
            return
        };
        let user_balance = coin::balance<SupraCoin>(task.owner);
        if (user_balance < task.fee) {
            // If the user does not have enough balance, remove the task, DON'T refund the locked deposit, but simply unlock it
            // and emit an event
            safe_unlock_locked_deposit(refund_bookkeeping, task.locked_deposit_fee, task.task_index);
            let task_meta = enumerable_map::remove_value(&mut automation_registry.main.tasks, task.task_index);
            vector::push_back(&mut intermediate_state.removed_tasks, task.task_index);
            event::emit(TaskCancelledInsufficentBalanceV2 {
                task_index: task.task_index,
                owner: task.owner,
                fee: task.fee,
                balance: user_balance,
                registration_hash: task_meta.tx_hash
            });
            return
        };
        if (task.fee != 0) {
            // Charge the fee and emit a success event
            let withdrawn_coins = coin::withdraw<SupraCoin>(
                &create_signer(task.owner),
                task.fee
            );
            // Merge to total task fees deducted from the users account
            coin::merge(&mut intermediate_state.epoch_locked_fees, withdrawn_coins);
        };
        event::emit(TaskEpochFeeWithdraw {
            task_index: task.task_index,
            owner: task.owner,
            fee: task.fee,
        });

        // Calculate gas commitment for the next epoch only for valid active tasks
        if (task.expiry_time > current_cycle_end_time) {
            intermediate_state.gas_committed_for_next_cycle = intermediate_state.gas_committed_for_next_cycle+ task.max_gas_amount;
        };
    }

    /// The function updates the ActiveAutomationRegistryConfig structure with values extracted from the buffer, if the buffer exists.
    /// This function will be called only during migration and can be removed in subsequent releases
    /// Note this function should be called in scope of migrate_v2 after automation configuration has been migrated V2 as well
    fun update_config_from_buffer_for_migration(cycle_info: &mut AutomationCycleDetails) acquires ActiveAutomationRegistryConfigV2 {
        if (config_buffer::does_exist<AutomationRegistryConfig>()) {
            let buffer = config_buffer::extract<AutomationRegistryConfig>();
            let automation_registry_config = &mut borrow_global_mut<ActiveAutomationRegistryConfigV2>(
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
        // In case if between supra-framework update and migration step the config has been updated using the new v2 API.
        update_config_from_buffer(cycle_info)
    }

    /// The function updates the ActiveAutomationRegistryConfig structure with values extracted from the buffer, if the buffer exists.
    fun update_config_from_buffer(cycle_info: &mut AutomationCycleDetails) acquires ActiveAutomationRegistryConfigV2 {
        if (!config_buffer::does_exist<AutomationRegistryConfigV2>()) {
            return
        };
        let buffer = config_buffer::extract<AutomationRegistryConfigV2>();
        let active_config = borrow_global_mut<ActiveAutomationRegistryConfigV2>(
            @supra_framework
        );
        {
            let automation_registry_config = &mut active_config.main_config;
            automation_registry_config.task_duration_cap_in_secs = buffer.task_duration_cap_in_secs;
            automation_registry_config.registry_max_gas_cap = buffer.registry_max_gas_cap;
            automation_registry_config.automation_base_fee_in_quants_per_sec = buffer.automation_base_fee_in_quants_per_sec;
            automation_registry_config.flat_registration_fee_in_quants = buffer.flat_registration_fee_in_quants;
            automation_registry_config.congestion_threshold_percentage = buffer.congestion_threshold_percentage;
            automation_registry_config.congestion_base_fee_in_quants_per_sec = buffer.congestion_base_fee_in_quants_per_sec;
            automation_registry_config.congestion_exponent = buffer.congestion_exponent;
            automation_registry_config.task_capacity = buffer.task_capacity;
        };

        if (std::option::is_some(&cycle_info.transition_state)) {
            let transition_state = std::option::borrow_mut(&mut cycle_info.transition_state);
            transition_state.new_cycle_duration = buffer.cycle_duration_secs;
        } else {
            cycle_info.duration_secs = buffer.cycle_duration_secs;
        };

        {
            let system_task_config = &mut active_config.system_task_config;
            system_task_config.task_capacity = buffer.sys_task_capacity;
            system_task_config.registry_max_gas_cap = buffer.sys_registry_max_gas_cap;
            system_task_config.task_duration_cap_in_secs = buffer.sys_task_duration_cap_in_secs;

        }
    }

    /// Transfers the specified fee amount from the resource account to the target account.
    fun transfer_fee_to_account_internal(to: address, amount: u64) acquires AutomationRegistryV2, AutomationRefundBookkeeping {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        let refund_bookkeeping = borrow_global<AutomationRefundBookkeeping>(@supra_framework);
        let resource_balance = coin::balance<SupraCoin>(automation_registry.main.registry_fee_address);

        assert!(resource_balance >= amount, EINSUFFICIENT_BALANCE);

        assert!((resource_balance - amount)
            >= automation_registry.main.epoch_locked_fees + refund_bookkeeping.total_deposited_automation_fee,
            EREQUEST_EXCEEDS_LOCKED_BALANCE);

        let resource_signer = account::create_signer_with_capability(
            &automation_registry.main.registry_fee_address_signer_cap
        );
        coin::transfer<SupraCoin>(&resource_signer, to, amount);
    }

    fun validate_task_duration(
        expiry_time: u64,
        registration_time: u64,
        automation_registry_config: &ActiveAutomationRegistryConfigV2,
        automation_cycle_info: &AutomationCycleDetails,
        task_type: u8
    ) {
        assert!(expiry_time > registration_time, EINVALID_EXPIRY_TIME);
        let task_duration = expiry_time - registration_time;
        if (task_type == UST) {
            assert!(task_duration <= automation_registry_config.main_config.task_duration_cap_in_secs, EEXPIRY_TIME_UPPER);
        } else {
            assert!(task_type == GST, EINVALID_TASK_TYPE);
            assert!(task_duration <= automation_registry_config.system_task_config.task_duration_cap_in_secs, EEXPIRY_TIME_UPPER);
        };
        // Check that task is valid at least in the next cycle
        assert!(
            expiry_time > (automation_cycle_info.start_time + automation_cycle_info.duration_secs),
            EEXPIRY_BEFORE_NEXT_CYCLE
        );
    }

    /// Validates auxiliary data , by checking task type and priority if any specified.
    /// Returns true if priority is not specify, false if specified.
    fun check_and_validate_aux_data(aux_data: &vector<vector<u8>>, task_type: u8) : bool {
        assert!(vector::length(aux_data) == SUPPORTED_AUX_DATA_COUNT_MAX, EINVALID_AUX_DATA_LENGTH);

        // Check task type
        let maybe_task_type = vector::borrow(aux_data, TYPE_AUX_DATA_INDEX);
        assert!(vector::length(maybe_task_type) == 1, EINVALID_TASK_TYPE_LENGTH);
        let type_value = vector::borrow(maybe_task_type, 0);
        assert!(*type_value == task_type, EINVALID_TASK_TYPE);

        // Check priority existence
        let maybe_task_priority = vector::borrow(aux_data, PRIORITY_AUX_DATA_INDEX);
        let has_no_priority = vector::is_empty(maybe_task_priority);
        if (!has_no_priority) {
            // If there is a value specified validate that it can be converted to u64 successfully.
            // This will allow to avoid invalid task registration
            let _ = from_bcs::to_u64(*maybe_task_priority);
        };
        has_no_priority
    }

    // Precondition: `ActiveAutomationRegistryConfig` must exist at `@supra_framework`
    // Postcondition: `ActiveAutomationRegistryConfig` must not exist at `@supra_framework`
    // AND `ActiveAutomationRegistryConfigV2` must exist at `@supra_framework`
    fun migrate_registry_config(
        supra_framework: &signer,
        sys_task_duration_cap_in_secs: u64,
        sys_registry_max_gas_cap: u64,
        sys_task_capacity: u16

    ) acquires ActiveAutomationRegistryConfig {
        let current_active_config = move_from<ActiveAutomationRegistryConfig>(@supra_framework);
        let ActiveAutomationRegistryConfig {
            main_config,
            next_epoch_registry_max_gas_cap,
            registration_enabled
        } = current_active_config;
        let system_task_config =  RegistryConfigForSystemTasks {
            task_duration_cap_in_secs: sys_task_duration_cap_in_secs,
            registry_max_gas_cap: sys_registry_max_gas_cap,
            task_capacity: sys_task_capacity,
            aux_properties: simple_map::new()
        };
        let new_active_config = ActiveAutomationRegistryConfigV2 {
            main_config,
            next_cycle_registry_max_gas_cap: next_epoch_registry_max_gas_cap,
            next_cycle_sys_registry_max_gas_cap: sys_registry_max_gas_cap,
            registration_enabled,
            system_task_config,
            aux_configs: simple_map::new(),
        };
        move_to<ActiveAutomationRegistryConfigV2>(supra_framework, new_active_config);
    }

    /// Initializes registry state for system tasks
    fun migrate_registry_state(supra_framework: &signer, automation_registry: AutomationRegistry) {
        system_addresses::assert_supra_framework(supra_framework);
        let system_tasks_state =  RegistryStateForSystemTasks {
            gas_committed_for_this_cycle: 0,
            gas_committed_for_next_cycle: 0,
            authorized_accounts: vector[],
            task_ids: vector[],
        };

        move_to(supra_framework, AutomationRegistryV2 {
            main: automation_registry,
            system_tasks_state,
        });
    }


    fun upscale_from_u8(value: u8): u256 { (value as u256) * DECIMAL }

    fun upscale_from_u64(value: u64): u256 { (value as u256) * DECIMAL }

    fun upscale_from_u256(value: u256): u256 { value * DECIMAL }

    fun downscale_to_u64(value: u256): u64 { ((value / DECIMAL) as u64) }

    fun downscale_to_u256(value: u256): u256 { value / DECIMAL }

    /// If SUPRA_AUTOMATION_V2 is enabled then call native function to assert full support of cycle based
    /// automation registry management.
    fun assert_automation_cycle_management_support() {
        native_automation_cycle_management_support();
    }

    native fun native_automation_cycle_management_support(): bool;

    #[test_only]
    const PARENT_HASH: vector<u8> = x"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    #[test_only]
    const PAYLOAD: vector<u8> = x"0102030405060708090a0b0c0d0e0f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20101112131415161718191a1b1c1d1e1f20";
    #[test_only]
    const AUX_DATA: vector<vector<u8>> = vector[vector[1], vector[]];
    #[test_only]
    const SYS_AUX_DATA: vector<vector<u8>> = vector[vector[2], vector[]];

    #[test_only]
    public(friend) fun update_config_for_tests(
        supra_framework: &signer,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
    ) acquires ActiveAutomationRegistryConfigV2 {
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

        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfigV2>(@supra_framework);
        automation_registry_config.main_config = new_automation_registry_config;
        automation_registry_config.next_cycle_registry_max_gas_cap = registry_max_gas_cap;

        event::emit(new_automation_registry_config);
    }

    #[test_only]
    public(friend) fun update_system_task_config_for_tests(
        supra_framework: &signer,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        task_capacity: u16,
    ) acquires ActiveAutomationRegistryConfigV2 {
        system_addresses::assert_supra_framework(supra_framework);

        let new_system_registry_config = RegistryConfigForSystemTasks {
            task_duration_cap_in_secs,
            registry_max_gas_cap,
            task_capacity,
            aux_properties: simple_map::new(),
        };

        let automation_registry_config = borrow_global_mut<ActiveAutomationRegistryConfigV2>(@supra_framework);
        automation_registry_config.system_task_config = new_system_registry_config;
        automation_registry_config.next_cycle_sys_registry_max_gas_cap = registry_max_gas_cap;
    }

    #[test_only]
    public(friend) fun has_task_with_id(task_index: u64): bool acquires AutomationRegistryV2 {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        enumerable_map::contains(&automation_registry.main.tasks, task_index)
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun register_with_state(
        user: &signer,
        max_gas_amount: u64,
        automation_fee_cap: u64,
        expiry_time: u64,
        state: u8,
    ): u64 acquires AutomationRegistryV2, ActiveAutomationRegistryConfigV2, AutomationCycleDetails, AutomationRefundBookkeeping {
        register_with_custom_input(
            user,
            max_gas_amount,
            automation_fee_cap,
            20,
            expiry_time,
            PARENT_HASH,
            AUX_DATA,
            state,
        )
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun register_with_custom_input(
        user: &signer,
        max_gas_amount: u64,
        automation_fee_cap: u64,
        gas_price: u64,
        expiry_time: u64,
        parent_hash: vector<u8>,
        aux_data: vector<vector<u8>>,
        state: u8,
    ): u64 acquires AutomationRegistryV2, ActiveAutomationRegistryConfigV2, AutomationCycleDetails, AutomationRefundBookkeeping {
        register(user,
            PAYLOAD,
            expiry_time,
            max_gas_amount,
            gas_price,
            automation_fee_cap,
            parent_hash,
            aux_data
        );
        let automation_registry = &mut borrow_global_mut<AutomationRegistryV2>(@supra_framework).main;
        let task_index = automation_registry.current_index - 1;
        let task_details = enumerable_map::get_value_mut(&mut automation_registry.tasks, task_index);
        if (state != PENDING) {
            automation_registry.gas_committed_for_this_epoch = automation_registry.gas_committed_for_this_epoch + (max_gas_amount as u256);
            vector::push_back(&mut  automation_registry.epoch_active_task_ids, task_index)
        };
        if (state == CANCELLED) {
            automation_registry.gas_committed_for_next_epoch = automation_registry.gas_committed_for_next_epoch - max_gas_amount;
        };
        task_details.state = state;
        task_index
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun register_system_task_with_state(
        user: &signer,
        max_gas_amount: u64,
        expiry_time: u64,
        state: u8,
    ): u64 acquires AutomationRegistryV2, ActiveAutomationRegistryConfigV2, AutomationCycleDetails {
        register_system_task_with_custom_input(
            user,
            max_gas_amount,
            expiry_time,
            PARENT_HASH,
            SYS_AUX_DATA,
            state
        )
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun register_system_task_with_custom_input(
        user: &signer,
        max_gas_amount: u64,
        expiry_time: u64,
        parent_hash: vector<u8>,
        aux_data: vector<vector<u8>>,
        state: u8,
    ): u64 acquires AutomationRegistryV2, ActiveAutomationRegistryConfigV2, AutomationCycleDetails {
        register_system_task(user,
            PAYLOAD,
            expiry_time,
            max_gas_amount,
            parent_hash,
            aux_data
        );
        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);
        let task_index = automation_registry.main.current_index - 1;
        let task_details = enumerable_map::get_value_mut(&mut automation_registry.main.tasks, task_index);
        if (state != PENDING) {
            automation_registry.system_tasks_state.gas_committed_for_this_cycle = automation_registry.system_tasks_state.gas_committed_for_this_cycle + max_gas_amount;
            vector::push_back(&mut  automation_registry.main.epoch_active_task_ids, task_index)
        };
        if (state == CANCELLED) {
            automation_registry.system_tasks_state.gas_committed_for_next_cycle = automation_registry.system_tasks_state.gas_committed_for_next_cycle + max_gas_amount;
        };
        task_details.state = state;
        task_index
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun update_task_state(
        task_index: u64,
        state: u8,
    ) acquires AutomationRegistryV2 {
        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);
        let task_details = enumerable_map::get_value_mut(&mut automation_registry.main.tasks, task_index);
        task_details.state = state;
    }


    #[test_only]
    public(friend) fun set_locked_fee(
        locked_fee: u64,
    ) acquires AutomationRegistryV2 {
        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);
        automation_registry.main.epoch_locked_fees = locked_fee;
    }

    #[test_only]
    public(friend) fun set_total_deposited_automation_fee(
        fee: u64,
    ) acquires AutomationRefundBookkeeping {
        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);
        refund_bookkeeping.total_deposited_automation_fee = fee;
    }

    #[test_only]
    public(friend) fun set_gas_committed_for_next_cycle(
        gas: u64,
    ) acquires AutomationRegistryV2 {
        let automation_registry = borrow_global_mut<AutomationRegistryV2>(@supra_framework);
        automation_registry.main.gas_committed_for_next_epoch = gas;
    }

    /// Represents the fee charged for an automation task execution and some additional information.
    /// Used only in tests, substituted with AutomationTaskFeeMeta in production code.
    /// Kept for backward compatible framework upgrade.
    struct AutomationTaskFee has drop {
        task_index: u64,
        owner: address,
        fee: u64,
    }

    #[test_only]
    /// Calculates automation task fees for the active tasks for the provided interval with provided tcmg occupancy.
    public(friend) fun calculate_tasks_automation_fees(
        interval: u64,
        current_time: u64,
        tcmg: u256,
    ): vector<AutomationTaskFee> acquires  AutomationRegistryV2, ActiveAutomationRegistryConfigV2{
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        let arc = &borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework).main_config;
        let task_with_fees = vector[];

        // Compute the automation fee multiplier for epoch
        let automation_fee_per_sec = calculate_automation_fee_multiplier_for_epoch(arc, tcmg, arc.registry_max_gas_cap);

        enumerable_map::for_each_value_ref(&automation_registry.main.tasks, |task| {
            let task: &AutomationTaskMetaData = task;
            let task_fee = calculate_task_fee(arc, task, interval, current_time, automation_fee_per_sec);
            vector::push_back(&mut task_with_fees, AutomationTaskFee {
                task_index: task.task_index,
                owner: task.owner,
                fee: task_fee,
            });
        });
        task_with_fees
    }

    #[test_only]
    public(friend) fun process_tasks_for_tests(
        cycle_index: u64,
        task_indexes: vector<u64>
    ) acquires AutomationCycleDetails, AutomationRegistryV2, AutomationRefundBookkeeping, ActiveAutomationRegistryConfigV2 {
        process_tasks(create_signer(@vm_reserved), cycle_index, task_indexes);
    }

    #[test_only]
    public(friend) fun safe_deposit_refund_for_tests(
        task_index: u64,
        task_owner: address,
        refundable_deposit: u64,
        locked_deposit: u64
    ): bool acquires AutomationRefundBookkeeping, AutomationRegistryV2 {

        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        let refund_bookkeeping = borrow_global_mut<AutomationRefundBookkeeping>(@supra_framework);

        let resource_address = automation_registry.main.registry_fee_address;
        let resource_signer = account::create_signer_with_capability(
            &automation_registry.main.registry_fee_address_signer_cap
        );
        safe_deposit_refund(
            refund_bookkeeping,
            &resource_signer,
            resource_address,
            task_index,
            task_owner,
            refundable_deposit,
            locked_deposit)
    }

    #[test_only]
    public(friend) fun safe_fee_refund_for_tests(
        locked_fee: u64,
        task_index: u64,
        task_owner: address,
        refundable_fee: u64,
    ): (bool, u64) acquires AutomationRegistryV2 {

        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);

        let resource_address = automation_registry.main.registry_fee_address;
        let resource_signer = account::create_signer_with_capability(
            &automation_registry.main.registry_fee_address_signer_cap
        );
        safe_fee_refund(
            locked_fee,
            &resource_signer,
            resource_address,
            task_index,
            task_owner,
            refundable_fee)
    }

    #[test_only]
    public(friend) fun prepare_state_for_migration(framework: &signer) acquires AutomationCycleDetails, ActiveAutomationRegistryConfigV2, AutomationRegistryV2 {
        let cycle_details = move_from<AutomationCycleDetails>(@supra_framework);
        let epoch_info = AutomationEpochInfo {
            expected_epoch_duration: cycle_details.duration_secs,
            epoch_interval: cycle_details.duration_secs,
            start_time: cycle_details.start_time,
        };
        move_to(framework, epoch_info);

        let ActiveAutomationRegistryConfigV2 {
            main_config,
            next_cycle_registry_max_gas_cap,
            next_cycle_sys_registry_max_gas_cap: _,
            registration_enabled,
            system_task_config: _,
            aux_configs: _
        } = move_from<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let config = ActiveAutomationRegistryConfig {
            main_config,
            next_epoch_registry_max_gas_cap: next_cycle_registry_max_gas_cap,
            registration_enabled,
        };
        move_to(framework, config);
        // Drop system task state
        let AutomationRegistryV2 {
            main,
            system_tasks_state
        } = move_from<AutomationRegistryV2>(@supra_framework);
        let RegistryStateForSystemTasks {
            gas_committed_for_next_cycle: _,
            gas_committed_for_this_cycle: _,
            task_ids: _,
            authorized_accounts: _,
        } = system_tasks_state;
        move_to(framework, main);
    }
    #[test_only]
    public(friend) fun check_cycle_state_and_duration(expected_state: u8, expected_duration: u64, has_transition_state: bool): u64 acquires AutomationCycleDetails {
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_details.state == expected_state, 101);
        assert!(cycle_details.duration_secs == expected_duration, 102);
        assert!(std::option::is_some(&cycle_details.transition_state) == has_transition_state, 103);
        cycle_details.duration_secs
    }

    #[test_only]
    public(friend) fun check_cycle_new_duration(expected_duration: u64) acquires AutomationCycleDetails {
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(std::option::is_some(&cycle_details.transition_state), 104);

        let transition_state = std::option::borrow(&cycle_details.transition_state);
        assert!(transition_state.new_cycle_duration  == expected_duration, 105);
    }

    #[test_only]
    public(friend) fun check_next_task_index_to_be_processed(expected_state: u8, expected_index_position: u64) acquires AutomationCycleDetails {
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_details.state == expected_state, 106);
        assert!(std::option::is_some(&cycle_details.transition_state), 107);

        let transition_state = std::option::borrow(&cycle_details.transition_state);
        assert!(transition_state.next_task_index_position  == expected_index_position, 108);
    }

    #[test_only]
    public(friend) fun check_suspended_cycle_state(expected_refund_duration: u64, expected_automation_fee_multiplier: u64) acquires AutomationCycleDetails {
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_details.state == CYCLE_SUSPENDED, 109);
        let transition_state = std::option::borrow(&cycle_details.transition_state);
        assert!(transition_state.refund_duration == expected_refund_duration, 110);
        assert!(transition_state.automation_fee_per_sec == expected_automation_fee_multiplier, 111);
        assert!(transition_state.gas_committed_for_new_cycle == 0, 112);
        assert!(transition_state.gas_committed_for_next_cycle  == 0, 113);
        assert!(transition_state.next_task_index_position == 0, 114);
    }

    #[test_only]
    public(friend) fun check_cycle_state(state: u8, index: u64, start_time: u64, cycle_duration: u64) acquires  AutomationCycleDetails {
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        // Check that we are still in finished state and processed-task are only task2 and task3
        assert!(cycle_details.state == state, (cycle_details.state as u64));
        assert!(cycle_details.index == index, cycle_details.index);
        assert!(cycle_details.start_time == start_time, cycle_details.start_time);
        assert!(cycle_details.duration_secs == cycle_duration, cycle_details.duration_secs);
        if (state == CYCLE_FINISHED || state == CYCLE_SUSPENDED) {
            assert!(std::option::is_some(&cycle_details.transition_state), (state as  u64));
        }
    }

    #[test_only]
    public(friend) fun check_cycle_transition_state(
        expected_state: u8,
        expected_refund_duration: u64,
        expected_new_cycle_duration: u64,
        expected_automation_fee_per_sec: u64,
        expected_gas_committed_for_new_cycle: u64,
        expected_gas_committed_for_next_cycle: u64,
        expected_sys_gas_committed_for_next_cycle: u64,
        expected_locked_fees: u64,
        expected_tasks_to_be_processed_len: u64,
        expected_next_task_index_position: u64
        ) acquires AutomationCycleDetails {
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        assert!(cycle_details.state == expected_state, 120);
        let transition_state = std::option::borrow(&cycle_details.transition_state);
        assert!(transition_state.refund_duration == expected_refund_duration, 121);
        assert!(transition_state.automation_fee_per_sec == expected_automation_fee_per_sec, 122);
        assert!(transition_state.gas_committed_for_new_cycle == expected_gas_committed_for_new_cycle, 123);
        assert!(transition_state.gas_committed_for_next_cycle  == expected_gas_committed_for_next_cycle, 124);
        assert!(transition_state.sys_gas_committed_for_next_cycle  == expected_sys_gas_committed_for_next_cycle, 125);
        assert!(transition_state.next_task_index_position == expected_next_task_index_position, 114);
        assert!(transition_state.locked_fees == expected_locked_fees, 115);
        assert!(transition_state.new_cycle_duration == expected_new_cycle_duration, 116);
        assert!(vector::length(&transition_state.expected_tasks_to_be_processed) == expected_tasks_to_be_processed_len, 117);
    }

    #[test_only]
    public(friend) fun check_automation_configuration(
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
        cycle_duration_secs: u64,
        sys_task_duration_cap_in_secs: u64,
        sys_registry_max_gas_cap: u64,
        sys_task_capacity: u16,
        ) acquires ActiveAutomationRegistryConfigV2, AutomationCycleDetails {
        let registry_config = borrow_global<ActiveAutomationRegistryConfigV2>(@supra_framework);
        let main_config = &registry_config.main_config;
        let system_config = &registry_config.system_task_config;
        assert!(main_config.registry_max_gas_cap == registry_max_gas_cap, 201);
        assert!(main_config.task_duration_cap_in_secs == task_duration_cap_in_secs, 202);
        assert!(main_config.automation_base_fee_in_quants_per_sec == automation_base_fee_in_quants_per_sec, 203);
        assert!(main_config.flat_registration_fee_in_quants == flat_registration_fee_in_quants, 204);
        assert!(main_config.congestion_threshold_percentage == congestion_threshold_percentage, 205);
        assert!(main_config.congestion_base_fee_in_quants_per_sec == congestion_base_fee_in_quants_per_sec, 206);
        assert!(main_config.congestion_exponent == congestion_exponent, 207);
        assert!(main_config.task_capacity == task_capacity, 208);
        assert!(system_config.registry_max_gas_cap == sys_registry_max_gas_cap, 209);
        assert!(system_config.task_duration_cap_in_secs == sys_task_duration_cap_in_secs, 210);
        assert!(system_config.task_capacity == sys_task_capacity, 211);
        let cycle_details = borrow_global<AutomationCycleDetails>(@supra_framework);
        if (option::is_none(&cycle_details.transition_state)) {
            assert!(cycle_details.duration_secs == cycle_duration_secs, 212);
        } else {
            let transition_state = option::borrow(&cycle_details.transition_state);
            assert!(transition_state.new_cycle_duration == cycle_duration_secs, 213);
        }
    }

    #[test_only]
    public(friend) fun check_task_priority(task_details: &AutomationTaskMetaData, expected_priority: u64) {
        let priority_raw = vector::borrow(&task_details.aux_data,  PRIORITY_AUX_DATA_INDEX);
        let priority  = from_bcs::to_u64(*priority_raw);
        assert!(priority == expected_priority, 300);
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun check_task_state(
        task_index: u64,
        exists: bool,
        state: u8,
    ) acquires AutomationRegistryV2  {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        assert!(enumerable_map::contains(&automation_registry.main.tasks, task_index) == exists, 301);
        if (exists) {
            let task_details = enumerable_map::get_value_ref(&automation_registry.main.tasks, task_index);
            assert!(task_details.state == state, 302);
        }
    }

    #[test_only]
    /// Registers a task with specified state and returns the task index
    public(friend) fun check_gas_and_fees_for_cycle(
        expected_gas_for_this_cycle: u64,
        expected_gas_for_next_cycle: u64,
        expected_locked_fees: u64,
        expected_total_deposit_fees: u64
    ) acquires AutomationRegistryV2, AutomationRefundBookkeeping  {
        let automation_registry = borrow_global<AutomationRegistryV2>(@supra_framework);
        let refund_bookkeeping = borrow_global<AutomationRefundBookkeeping>(@supra_framework);
        assert!(automation_registry.main.gas_committed_for_next_epoch == expected_gas_for_next_cycle, 401);
        assert!(automation_registry.main.gas_committed_for_this_epoch == (expected_gas_for_this_cycle as u256), 402);
        assert!(automation_registry.main.epoch_locked_fees == expected_locked_fees, 403);
        assert!(refund_bookkeeping.total_deposited_automation_fee == expected_total_deposit_fees, 404);
    }

    #[test_only]
    public(friend) fun check_automation_fee(calculated_fee_data: &vector<AutomationTaskFee>, index: u64, expected_fee: u64) {
        let r1 = vector::borrow(calculated_fee_data, index);
        assert!(r1.fee == expected_fee, 501);
    }

    #[test_only]
    public(friend) fun check_epoch_and_cycle_resources(epoch_data_exists: bool, cycle_data_exists: bool) {
        assert!(exists<AutomationEpochInfo>(@supra_framework) == epoch_data_exists, 600);
        assert!(exists<AutomationCycleDetails>(@supra_framework) == cycle_data_exists, 601);
    }

    #[test]
    fun check_calculate_exponentiation() {
        let congestion_exponent: u8 = 6;
        // 5% threshould which means (5/100) * DECIMAL
        let result = calculate_exponentiation(5 * DECIMAL / 100, congestion_exponent);
        assert!(result == 34009563, 11); // ~0.34

        // 28% threshould which means (28/100) * DECIMAL
        let result = calculate_exponentiation(28 * DECIMAL / 100, congestion_exponent);
        assert!(result == 339804650, 12); // ~3.39

        // 50% threshould which means (50/100) * DECIMAL
        let result = calculate_exponentiation(50 * DECIMAL / 100, congestion_exponent);
        assert!(result == 1039062500, 13); // ~10.39
    }

    #[test]
    #[expected_failure(abort_code = 65537, location = from_bcs)]
    fun check_invalid_priority_aux_data(
    ) {
        let aux_data = vector[vector[UST], vector[4, 5]];
        check_and_validate_aux_data(&aux_data, UST);
    }

}
