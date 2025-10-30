
<a id="0x1_automation_registry"></a>

# Module `0x1::automation_registry`

Copywrite (c) -- 2025 Supra
Supra Automation Registry

This contract is part of the Supra Framework and is designed to manage automated task entries


-  [Resource `ActiveAutomationRegistryConfig`](#0x1_automation_registry_ActiveAutomationRegistryConfig)
-  [Resource `ActiveAutomationRegistryConfigV2`](#0x1_automation_registry_ActiveAutomationRegistryConfigV2)
-  [Resource `AutomationRegistryConfig`](#0x1_automation_registry_AutomationRegistryConfig)
-  [Struct `RegistryConfigForSystemTasks`](#0x1_automation_registry_RegistryConfigForSystemTasks)
-  [Struct `AutomationRegistryConfigV2`](#0x1_automation_registry_AutomationRegistryConfigV2)
-  [Resource `AutomationRegistry`](#0x1_automation_registry_AutomationRegistry)
-  [Struct `RegistryStateForSystemTasks`](#0x1_automation_registry_RegistryStateForSystemTasks)
-  [Resource `AutomationRegistryV2`](#0x1_automation_registry_AutomationRegistryV2)
-  [Struct `TransitionState`](#0x1_automation_registry_TransitionState)
-  [Resource `AutomationEpochInfo`](#0x1_automation_registry_AutomationEpochInfo)
-  [Struct `AutomationCycleInfo`](#0x1_automation_registry_AutomationCycleInfo)
-  [Struct `AutomationCycleEvent`](#0x1_automation_registry_AutomationCycleEvent)
-  [Resource `AutomationCycleDetails`](#0x1_automation_registry_AutomationCycleDetails)
-  [Resource `AutomationRefundBookkeeping`](#0x1_automation_registry_AutomationRefundBookkeeping)
-  [Resource `AutomationTaskMetaData`](#0x1_automation_registry_AutomationTaskMetaData)
-  [Struct `TaskRegistrationFeeWithdraw`](#0x1_automation_registry_TaskRegistrationFeeWithdraw)
-  [Struct `TaskRegistrationDepositFeeWithdraw`](#0x1_automation_registry_TaskRegistrationDepositFeeWithdraw)
-  [Struct `RegistryFeeWithdraw`](#0x1_automation_registry_RegistryFeeWithdraw)
-  [Struct `TaskEpochFeeWithdraw`](#0x1_automation_registry_TaskEpochFeeWithdraw)
-  [Struct `TaskFeeRefund`](#0x1_automation_registry_TaskFeeRefund)
-  [Struct `TaskDepositFeeRefund`](#0x1_automation_registry_TaskDepositFeeRefund)
-  [Struct `ErrorUnlockTaskDepositFee`](#0x1_automation_registry_ErrorUnlockTaskDepositFee)
-  [Struct `ErrorUnlockTaskEpochFee`](#0x1_automation_registry_ErrorUnlockTaskEpochFee)
-  [Struct `TaskCancelled`](#0x1_automation_registry_TaskCancelled)
-  [Struct `TaskCancelledV2`](#0x1_automation_registry_TaskCancelledV2)
-  [Struct `TasksStopped`](#0x1_automation_registry_TasksStopped)
-  [Struct `TaskStopped`](#0x1_automation_registry_TaskStopped)
-  [Struct `TasksStoppedV2`](#0x1_automation_registry_TasksStoppedV2)
-  [Struct `TaskStoppedV2`](#0x1_automation_registry_TaskStoppedV2)
-  [Struct `TaskCancelledInsufficentBalance`](#0x1_automation_registry_TaskCancelledInsufficentBalance)
-  [Struct `TaskCancelledInsufficentBalanceV2`](#0x1_automation_registry_TaskCancelledInsufficentBalanceV2)
-  [Struct `TaskCancelledCapacitySurpassed`](#0x1_automation_registry_TaskCancelledCapacitySurpassed)
-  [Struct `TaskCancelledCapacitySurpassedV2`](#0x1_automation_registry_TaskCancelledCapacitySurpassedV2)
-  [Struct `RemovedTasks`](#0x1_automation_registry_RemovedTasks)
-  [Struct `ActiveTasks`](#0x1_automation_registry_ActiveTasks)
-  [Struct `ErrorTaskDoesNotExist`](#0x1_automation_registry_ErrorTaskDoesNotExist)
-  [Struct `ErrorTaskDoesNotExistForWithdrawal`](#0x1_automation_registry_ErrorTaskDoesNotExistForWithdrawal)
-  [Struct `ErrorInsufficientBalanceToRefund`](#0x1_automation_registry_ErrorInsufficientBalanceToRefund)
-  [Struct `ErrorInconsistentSuspendedState`](#0x1_automation_registry_ErrorInconsistentSuspendedState)
-  [Struct `EnabledRegistrationEvent`](#0x1_automation_registry_EnabledRegistrationEvent)
-  [Struct `DisabledRegistrationEvent`](#0x1_automation_registry_DisabledRegistrationEvent)
-  [Struct `AuthorizationGranted`](#0x1_automation_registry_AuthorizationGranted)
-  [Struct `AuthorizationRevoked`](#0x1_automation_registry_AuthorizationRevoked)
-  [Struct `AutomationTaskFeeMeta`](#0x1_automation_registry_AutomationTaskFeeMeta)
-  [Struct `IntermediateState`](#0x1_automation_registry_IntermediateState)
-  [Struct `IntermediateStateOfEpochChange`](#0x1_automation_registry_IntermediateStateOfEpochChange)
-  [Struct `IntermediateStateOfCycleChange`](#0x1_automation_registry_IntermediateStateOfCycleChange)
-  [Struct `AutomationTaskFee`](#0x1_automation_registry_AutomationTaskFee)
-  [Constants](#@Constants_0)
-  [Function `is_transition_finalized`](#0x1_automation_registry_is_transition_finalized)
-  [Function `is_transition_in_progress`](#0x1_automation_registry_is_transition_in_progress)
-  [Function `mark_task_processed`](#0x1_automation_registry_mark_task_processed)
-  [Function `is_of_type`](#0x1_automation_registry_is_of_type)
-  [Function `is_initialized`](#0x1_automation_registry_is_initialized)
-  [Function `is_feature_enabled_and_initialized`](#0x1_automation_registry_is_feature_enabled_and_initialized)
-  [Function `get_next_task_index`](#0x1_automation_registry_get_next_task_index)
-  [Function `get_task_count`](#0x1_automation_registry_get_task_count)
-  [Function `get_system_task_count`](#0x1_automation_registry_get_system_task_count)
-  [Function `get_task_ids`](#0x1_automation_registry_get_task_ids)
-  [Function `get_epoch_locked_balance`](#0x1_automation_registry_get_epoch_locked_balance)
-  [Function `get_locked_deposit_balance`](#0x1_automation_registry_get_locked_deposit_balance)
-  [Function `get_registry_total_locked_balance`](#0x1_automation_registry_get_registry_total_locked_balance)
-  [Function `get_active_task_ids`](#0x1_automation_registry_get_active_task_ids)
-  [Function `get_task_details`](#0x1_automation_registry_get_task_details)
-  [Function `deconstruct_task_metadata`](#0x1_automation_registry_deconstruct_task_metadata)
-  [Function `get_task_owner`](#0x1_automation_registry_get_task_owner)
-  [Function `get_task_details_bulk`](#0x1_automation_registry_get_task_details_bulk)
-  [Function `has_sender_active_task_with_id`](#0x1_automation_registry_has_sender_active_task_with_id)
-  [Function `has_sender_active_system_task_with_id`](#0x1_automation_registry_has_sender_active_system_task_with_id)
-  [Function `has_sender_active_task_with_id_and_type`](#0x1_automation_registry_has_sender_active_task_with_id_and_type)
-  [Function `get_registry_fee_address`](#0x1_automation_registry_get_registry_fee_address)
-  [Function `get_gas_committed_for_next_epoch`](#0x1_automation_registry_get_gas_committed_for_next_epoch)
-  [Function `get_gas_committed_for_current_epoch`](#0x1_automation_registry_get_gas_committed_for_current_epoch)
-  [Function `get_automation_registry_config`](#0x1_automation_registry_get_automation_registry_config)
-  [Function `get_automation_registry_config_for_system_tasks`](#0x1_automation_registry_get_automation_registry_config_for_system_tasks)
-  [Function `get_next_epoch_registry_max_gas_cap`](#0x1_automation_registry_get_next_epoch_registry_max_gas_cap)
-  [Function `get_automation_epoch_info`](#0x1_automation_registry_get_automation_epoch_info)
-  [Function `estimate_automation_fee`](#0x1_automation_registry_estimate_automation_fee)
-  [Function `estimate_automation_fee_with_committed_occupancy`](#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy)
-  [Function `calculate_automation_fee_multiplier_for_committed_occupancy`](#0x1_automation_registry_calculate_automation_fee_multiplier_for_committed_occupancy)
-  [Function `calculate_automation_fee_multiplier_for_current_cycle`](#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle)
-  [Function `is_registration_enabled`](#0x1_automation_registry_is_registration_enabled)
-  [Function `get_cycle_duration`](#0x1_automation_registry_get_cycle_duration)
-  [Function `get_cycle_info`](#0x1_automation_registry_get_cycle_info)
-  [Function `get_record_max_task_count`](#0x1_automation_registry_get_record_max_task_count)
-  [Function `get_system_task_indexes`](#0x1_automation_registry_get_system_task_indexes)
-  [Function `get_system_gas_committed_for_next_cycle`](#0x1_automation_registry_get_system_gas_committed_for_next_cycle)
-  [Function `get_system_gas_committed_for_current_cycle`](#0x1_automation_registry_get_system_gas_committed_for_current_cycle)
-  [Function `is_authorized_account`](#0x1_automation_registry_is_authorized_account)
-  [Function `withdraw_automation_task_fees`](#0x1_automation_registry_withdraw_automation_task_fees)
-  [Function `update_config`](#0x1_automation_registry_update_config)
-  [Function `update_config_v2`](#0x1_automation_registry_update_config_v2)
-  [Function `enable_registration`](#0x1_automation_registry_enable_registration)
-  [Function `disable_registration`](#0x1_automation_registry_disable_registration)
-  [Function `grant_authorization`](#0x1_automation_registry_grant_authorization)
-  [Function `revoke_authorization`](#0x1_automation_registry_revoke_authorization)
-  [Function `cancel_task`](#0x1_automation_registry_cancel_task)
-  [Function `stop_tasks`](#0x1_automation_registry_stop_tasks)
-  [Function `stop_system_tasks`](#0x1_automation_registry_stop_system_tasks)
-  [Function `cancel_system_task`](#0x1_automation_registry_cancel_system_task)
-  [Function `initialize_refund_bookkeeping_resource`](#0x1_automation_registry_initialize_refund_bookkeeping_resource)
-  [Function `migrate_v2`](#0x1_automation_registry_migrate_v2)
-  [Function `initialize`](#0x1_automation_registry_initialize)
-  [Function `monitor_cycle_end`](#0x1_automation_registry_monitor_cycle_end)
-  [Function `on_new_epoch`](#0x1_automation_registry_on_new_epoch)
-  [Function `register`](#0x1_automation_registry_register)
-  [Function `register_system_task`](#0x1_automation_registry_register_system_task)
-  [Function `process_tasks`](#0x1_automation_registry_process_tasks)
-  [Function `on_cycle_transition`](#0x1_automation_registry_on_cycle_transition)
-  [Function `on_cycle_suspend`](#0x1_automation_registry_on_cycle_suspend)
-  [Function `drop_or_charge_tasks`](#0x1_automation_registry_drop_or_charge_tasks)
-  [Function `drop_or_charge_task`](#0x1_automation_registry_drop_or_charge_task)
-  [Function `refund_deposit_and_drop`](#0x1_automation_registry_refund_deposit_and_drop)
-  [Function `drop_system_task`](#0x1_automation_registry_drop_system_task)
-  [Function `refund_task_fees`](#0x1_automation_registry_refund_task_fees)
-  [Function `into_automation_cycle_info`](#0x1_automation_registry_into_automation_cycle_info)
-  [Function `update_cycle_transition_state_from_suspended`](#0x1_automation_registry_update_cycle_transition_state_from_suspended)
-  [Function `update_cycle_transition_state_from_finished`](#0x1_automation_registry_update_cycle_transition_state_from_finished)
-  [Function `estimate_automation_fee_with_committed_occupancy_internal`](#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal)
-  [Function `validate_configuration_parameters_common`](#0x1_automation_registry_validate_configuration_parameters_common)
-  [Function `validate_system_configuration_parameters_common`](#0x1_automation_registry_validate_system_configuration_parameters_common)
-  [Function `create_registry_resource_account`](#0x1_automation_registry_create_registry_resource_account)
-  [Function `on_cycle_end_internal`](#0x1_automation_registry_on_cycle_end_internal)
-  [Function `update_cycle_state_to`](#0x1_automation_registry_update_cycle_state_to)
-  [Function `move_to_ready_state`](#0x1_automation_registry_move_to_ready_state)
-  [Function `move_to_started_state`](#0x1_automation_registry_move_to_started_state)
-  [Function `try_move_to_suspended_state`](#0x1_automation_registry_try_move_to_suspended_state)
-  [Function `update_state_for_migration`](#0x1_automation_registry_update_state_for_migration)
-  [Function `refund_fees_and_update_tasks`](#0x1_automation_registry_refund_fees_and_update_tasks)
-  [Function `safe_deposit_refund`](#0x1_automation_registry_safe_deposit_refund)
-  [Function `safe_unlock_locked_deposit`](#0x1_automation_registry_safe_unlock_locked_deposit)
-  [Function `safe_unlock_locked_epoch_fee`](#0x1_automation_registry_safe_unlock_locked_epoch_fee)
-  [Function `safe_fee_refund`](#0x1_automation_registry_safe_fee_refund)
-  [Function `safe_refund`](#0x1_automation_registry_safe_refund)
-  [Function `calculate_task_fee`](#0x1_automation_registry_calculate_task_fee)
-  [Function `calculate_automation_fee_for_interval`](#0x1_automation_registry_calculate_automation_fee_for_interval)
-  [Function `calculate_automation_fee_multiplier_for_current_cycle_internal`](#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle_internal)
-  [Function `calculate_automation_fee_multiplier_for_epoch`](#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch)
-  [Function `calculate_automation_congestion_fee`](#0x1_automation_registry_calculate_automation_congestion_fee)
-  [Function `calculate_exponentiation`](#0x1_automation_registry_calculate_exponentiation)
-  [Function `try_withdraw_task_automation_fee`](#0x1_automation_registry_try_withdraw_task_automation_fee)
-  [Function `update_config_from_buffer_for_migration`](#0x1_automation_registry_update_config_from_buffer_for_migration)
-  [Function `update_config_from_buffer`](#0x1_automation_registry_update_config_from_buffer)
-  [Function `transfer_fee_to_account_internal`](#0x1_automation_registry_transfer_fee_to_account_internal)
-  [Function `validate_task_duration`](#0x1_automation_registry_validate_task_duration)
-  [Function `check_and_validate_aux_data`](#0x1_automation_registry_check_and_validate_aux_data)
-  [Function `migrate_registry_config`](#0x1_automation_registry_migrate_registry_config)
-  [Function `migrate_registry_state`](#0x1_automation_registry_migrate_registry_state)
-  [Function `upscale_from_u8`](#0x1_automation_registry_upscale_from_u8)
-  [Function `upscale_from_u64`](#0x1_automation_registry_upscale_from_u64)
-  [Function `upscale_from_u256`](#0x1_automation_registry_upscale_from_u256)
-  [Function `downscale_to_u64`](#0x1_automation_registry_downscale_to_u64)
-  [Function `downscale_to_u256`](#0x1_automation_registry_downscale_to_u256)
-  [Function `assert_automation_cycle_management_support`](#0x1_automation_registry_assert_automation_cycle_management_support)
-  [Function `native_automation_cycle_management_support`](#0x1_automation_registry_native_automation_cycle_management_support)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map">0x1::enumerable_map</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="multisig_account.md#0x1_multisig_account">0x1::multisig_account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="../../supra-stdlib/doc/vector_utils.md#0x1_vector_utils">0x1::vector_utils</a>;
</code></pre>



<a id="0x1_automation_registry_ActiveAutomationRegistryConfig"></a>

## Resource `ActiveAutomationRegistryConfig`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>main_config: <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next_epoch_registry_max_gas_cap: u64</code>
</dt>
<dd>
 Will be the same as main_config.registry_max_gas_cap, unless updated during the epoch.
</dd>
<dt>
<code>registration_enabled: bool</code>
</dt>
<dd>
 Flag indicating whether the task registration is enabled or paused.
 If paused a new task registration will fail.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_ActiveAutomationRegistryConfigV2"></a>

## Resource `ActiveAutomationRegistryConfigV2`

Registry active configuration parameters for the current cycle.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>main_config: <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next_cycle_registry_max_gas_cap: u64</code>
</dt>
<dd>
 Will be the same as main_config.registry_max_gas_cap, unless updated during the cycle transiation.
</dd>
<dt>
<code>registration_enabled: bool</code>
</dt>
<dd>
 Flag indicating whether the task registration is enabled or paused.
 If paused a new task registration will fail.
</dd>
<dt>
<code>system_task_config: <a href="automation_registry.md#0x1_automation_registry_RegistryConfigForSystemTasks">automation_registry::RegistryConfigForSystemTasks</a></code>
</dt>
<dd>
 Configuration parameters for system tasks
</dd>
<dt>
<code>next_cycle_sys_registry_max_gas_cap: u64</code>
</dt>
<dd>
 Will be the same as system_task_config.registry_max_gas_cap, unless updated during the cycle transition.
</dd>
<dt>
<code>aux_configs: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/doc/any.md#0x1_any_Any">any::Any</a>&gt;</code>
</dt>
<dd>
 Auxiliary configurations to support future expansions.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationRegistryConfig"></a>

## Resource `AutomationRegistryConfig`

Automation registry configuration parameters


<pre><code>#[<a href="event.md#0x1_event">event</a>]
#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_duration_cap_in_secs: u64</code>
</dt>
<dd>
 Maximum allowable duration (in seconds) from the registration time that an automation task can run.
 If the expiration time exceeds this duration, the task registration will fail.
</dd>
<dt>
<code>registry_max_gas_cap: u64</code>
</dt>
<dd>
 Maximum gas allocation for automation tasks per cycle
 Exceeding this limit during task registration will cause failure and is used in fee calculation.
</dd>
<dt>
<code>automation_base_fee_in_quants_per_sec: u64</code>
</dt>
<dd>
 Base fee per second for the full capacity of the automation registry, measured in quants/sec.
 The capacity is considered full if the total committed gas of all registered tasks equals registry_max_gas_cap.
</dd>
<dt>
<code>flat_registration_fee_in_quants: u64</code>
</dt>
<dd>
 Flat registration fee charged by default for each task.
</dd>
<dt>
<code>congestion_threshold_percentage: u8</code>
</dt>
<dd>
 Ratio (in the range [0;100]) representing the acceptable upper limit of committed gas amount
 relative to registry_max_gas_cap. Beyond this threshold, congestion fees apply.
</dd>
<dt>
<code>congestion_base_fee_in_quants_per_sec: u64</code>
</dt>
<dd>
 Base fee per second for the full capacity of the automation registry when the congestion threshold is exceeded.
</dd>
<dt>
<code>congestion_exponent: u8</code>
</dt>
<dd>
 The congestion fee increases exponentially based on this value, ensuring higher fees as the registry approaches full capacity.
</dd>
<dt>
<code>task_capacity: u16</code>
</dt>
<dd>
 Maximum number of tasks that registry can hold.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_RegistryConfigForSystemTasks"></a>

## Struct `RegistryConfigForSystemTasks`

Automation registry configuration parameters for governance/system submitted tasks


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_RegistryConfigForSystemTasks">RegistryConfigForSystemTasks</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_duration_cap_in_secs: u64</code>
</dt>
<dd>
 Maximum allowable duration (in seconds) from the registration time that an system automation task can run.
 If the expiration time exceeds this duration, the task registration will fail.
</dd>
<dt>
<code>registry_max_gas_cap: u64</code>
</dt>
<dd>
 Maximum gas allocation for system automation tasks per cycle
 Exceeding this limit during task registration will cause failure and is used in fee calculation.
</dd>
<dt>
<code>task_capacity: u16</code>
</dt>
<dd>
 Maximum number of system tasks that registry can hold.
</dd>
<dt>
<code>aux_properties: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, u64&gt;</code>
</dt>
<dd>
 Auxiliary configuration properties to easy expansion after release if required.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationRegistryConfigV2"></a>

## Struct `AutomationRegistryConfigV2`

Automation registry configuration parameters


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfigV2">AutomationRegistryConfigV2</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_duration_cap_in_secs: u64</code>
</dt>
<dd>
 Maximum allowable duration (in seconds) from the registration time that an automation task can run.
 If the expiration time exceeds this duration, the task registration will fail.
</dd>
<dt>
<code>registry_max_gas_cap: u64</code>
</dt>
<dd>
 Maximum gas allocation for automation tasks per epoch
 Exceeding this limit during task registration will cause failure and is used in fee calculation.
</dd>
<dt>
<code>automation_base_fee_in_quants_per_sec: u64</code>
</dt>
<dd>
 Base fee per second for the full capacity of the automation registry, measured in quants/sec.
 The capacity is considered full if the total committed gas of all registered tasks equals registry_max_gas_cap.
</dd>
<dt>
<code>flat_registration_fee_in_quants: u64</code>
</dt>
<dd>
 Flat registration fee charged by default for each task.
</dd>
<dt>
<code>congestion_threshold_percentage: u8</code>
</dt>
<dd>
 Ratio (in the range [0;100]) representing the acceptable upper limit of committed gas amount
 relative to registry_max_gas_cap. Beyond this threshold, congestion fees apply.
</dd>
<dt>
<code>congestion_base_fee_in_quants_per_sec: u64</code>
</dt>
<dd>
 Base fee per second for the full capacity of the automation registry when the congestion threshold is exceeded.
</dd>
<dt>
<code>congestion_exponent: u8</code>
</dt>
<dd>
 The congestion fee increases exponentially based on this value, ensuring higher fees as the registry approaches full capacity.
</dd>
<dt>
<code>task_capacity: u16</code>
</dt>
<dd>
 Maximum number of tasks that registry can hold.
</dd>
<dt>
<code>cycle_duration_secs: u64</code>
</dt>
<dd>
 Automation cycle duration in secods
</dd>
<dt>
<code>sys_task_duration_cap_in_secs: u64</code>
</dt>
<dd>
 Maximum allowable duration (in seconds) from the registration time that an system automation task can run.
 If the expiration time exceeds this duration, the task registration will fail.
</dd>
<dt>
<code>sys_registry_max_gas_cap: u64</code>
</dt>
<dd>
 Maximum gas allocation for system automation tasks per cycle
 Exceeding this limit during task registration will cause failure and is used in fee calculation.
</dd>
<dt>
<code>sys_task_capacity: u16</code>
</dt>
<dd>
 Maximum number of system tasks that registry can hold.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationRegistry"></a>

## Resource `AutomationRegistry`

It tracks entries both pending and completed, organized by unique indices.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tasks: <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;u64, <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>&gt;</code>
</dt>
<dd>
 A collection of automation task entries that are active state.
</dd>
<dt>
<code>current_index: u64</code>
</dt>
<dd>
 Automation task index which increase
</dd>
<dt>
<code>gas_committed_for_next_epoch: u64</code>
</dt>
<dd>
 Gas committed for next epoch
</dd>
<dt>
<code>epoch_locked_fees: u64</code>
</dt>
<dd>
 Total fee charged to users during the epoch, which is not withdrawable
</dd>
<dt>
<code>gas_committed_for_this_epoch: u256</code>
</dt>
<dd>
 Total committed max gas amount at the beginning of the current epoch.
</dd>
<dt>
<code>registry_fee_address: <b>address</b></code>
</dt>
<dd>
 It's resource address which is use to deposit user automation fee
</dd>
<dt>
<code>registry_fee_address_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>
 Resource account signature capability
</dd>
<dt>
<code>epoch_active_task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>
 Cached active task indexes for the current epoch.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_RegistryStateForSystemTasks"></a>

## Struct `RegistryStateForSystemTasks`

It tracks entries both pending and completed, organized by unique indices.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_RegistryStateForSystemTasks">RegistryStateForSystemTasks</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gas_committed_for_next_cycle: u64</code>
</dt>
<dd>
 Gas committed for next cycle
</dd>
<dt>
<code>gas_committed_for_this_cycle: u64</code>
</dt>
<dd>
 Total committed max gas amount at the beginning of the current cycle.
</dd>
<dt>
<code>task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>
 Cached system task indexes
</dd>
<dt>
<code>authorized_accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>
 Authorized accounts to registry system tasks
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationRegistryV2"></a>

## Resource `AutomationRegistryV2`

It tracks entries both pending and active for user and system automation tasks, organized by unique indices.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>main: <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a></code>
</dt>
<dd>

</dd>
<dt>
<code>system_tasks_state: <a href="automation_registry.md#0x1_automation_registry_RegistryStateForSystemTasks">automation_registry::RegistryStateForSystemTasks</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TransitionState"></a>

## Struct `TransitionState`

It tracks entries both pending and completed, organized by unique indices.
Holds intermediate state data of the automation cycle transition from END->STARTED, or SUSPENDED->READY


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>refund_duration: u64</code>
</dt>
<dd>
 Refund duration of automation fees when automation feature/cycle is suspended.
</dd>
<dt>
<code>new_cycle_duration: u64</code>
</dt>
<dd>
 Duration of the new cycle to charge fees for.
</dd>
<dt>
<code>automation_fee_per_sec: u64</code>
</dt>
<dd>
 Calculated automation fee per second for a new cycle or for refund period.
</dd>
<dt>
<code>gas_committed_for_new_cycle: u64</code>
</dt>
<dd>
 Gas committed for the new cycle being transitioned.
</dd>
<dt>
<code>gas_committed_for_next_cycle: u64</code>
</dt>
<dd>
 Gas committed for the next cycle.
</dd>
<dt>
<code>sys_gas_committed_for_next_cycle: u64</code>
</dt>
<dd>
 Gas committed by system tasks for the next cycle.
</dd>
<dt>
<code>locked_fees: u64</code>
</dt>
<dd>
 Total fee charged from users for the new cycle, which is not withdrawable.
</dd>
<dt>
<code>expected_tasks_to_be_processed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>
 List of the tasks to be processed during transition.
 This list is sorted in ascending order.
 The requirement is that all tasks are processed in the order of their registration. Which should be true
 especially for cycle fee charges before new cycle start.
</dd>
<dt>
<code>next_task_index_position: u64</code>
</dt>
<dd>
 Position of the task index in the expected_tasks_to_be_processed to be processed next.
 It is incremented when an expected task is successfully processed.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationEpochInfo"></a>

## Resource `AutomationEpochInfo`

Epoch state. Deprecated since SUPRA_AUTOMATION_V2 version.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> <b>has</b> <b>copy</b>, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>expected_epoch_duration: u64</code>
</dt>
<dd>
 Epoch expected duration at the beginning of the new epoch, Based on this and actual
 epoch_duration which will be (current_time - last_reconfiguration_time) automation tasks
 refunds will be calculated.
 it will be updated upon each new epoch start with epoch_interval value.
 Although we should be careful with refunds if block production interval is quite high.
</dd>
<dt>
<code>epoch_interval: u64</code>
</dt>
<dd>
 Epoch interval that can be updated any moment of the time
</dd>
<dt>
<code>start_time: u64</code>
</dt>
<dd>
 Current epoch start time which is the same as last_reconfiguration_time
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationCycleInfo"></a>

## Struct `AutomationCycleInfo`

Provides information of the current cycle state.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">AutomationCycleInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: u64</code>
</dt>
<dd>
 Current cycle id. Incremented when a start of a new cycle is given.
</dd>
<dt>
<code>state: u8</code>
</dt>
<dd>
 State of the current cycle.
</dd>
<dt>
<code>start_time: u64</code>
</dt>
<dd>
 Current cycle start time which is updated with the current chain time when a cycle is incremented.
</dd>
<dt>
<code>duration_secs: u64</code>
</dt>
<dd>
 Automation cycle duration in seconds.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationCycleEvent"></a>

## Struct `AutomationCycleEvent`

Event emitted for cycle state transition.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleEvent">AutomationCycleEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>cycle_state_info: <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">automation_registry::AutomationCycleInfo</a></code>
</dt>
<dd>
 Updated cycle state information.
</dd>
<dt>
<code>old_state: u8</code>
</dt>
<dd>
 The state transitioned from
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationCycleDetails"></a>

## Resource `AutomationCycleDetails`

Cycle state.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: u64</code>
</dt>
<dd>
 Cycle index corresponding to the current state. Incremented when a transition to the new cycle is finalized.
</dd>
<dt>
<code>state: u8</code>
</dt>
<dd>
 State of the current cycle.
</dd>
<dt>
<code>start_time: u64</code>
</dt>
<dd>
 Current cycle start time which is updated with the current chain time when a cycle is incremented.
</dd>
<dt>
<code>duration_secs: u64</code>
</dt>
<dd>
 Automation cycle duration in seconds for the current cycle.
</dd>
<dt>
<code>transition_state: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>&gt;</code>
</dt>
<dd>
 Intermediate state of cycle transition to next one or suspended state.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationRefundBookkeeping"></a>

## Resource `AutomationRefundBookkeeping`

Automation Deposited fee bookkeeping configs


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> <b>has</b> <b>copy</b>, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>total_deposited_automation_fee: u64</code>
</dt>
<dd>
 Total deposited fee so far which is locked in resource account unless refund of it (fully or partially) is done.
 Regardless of the refunded amount the actual deposited amount is deduced to unlock it from the resource account.
</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationTaskMetaData"></a>

## Resource `AutomationTaskMetaData`

<code><a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a></code> represents a single automation task item, containing metadata.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>
 Automation task index in registry
</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>
 The address of the task owner.
</dd>
<dt>
<code>payload_tx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The function signature associated with the registry entry.
</dd>
<dt>
<code>expiry_time: u64</code>
</dt>
<dd>
 Expiry of the task, represented in a timestamp in second.
</dd>
<dt>
<code>tx_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The transaction hash of the request transaction.
</dd>
<dt>
<code>max_gas_amount: u64</code>
</dt>
<dd>
 Max gas amount of automation task
</dd>
<dt>
<code>gas_price_cap: u64</code>
</dt>
<dd>
 Maximum gas price cap for the task
</dd>
<dt>
<code>automation_fee_cap_for_epoch: u64</code>
</dt>
<dd>
 Maximum automation fee for epoch to be paid ever.
</dd>
<dt>
<code>aux_data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Auxiliary data specified for the task to aid registration.
 Not used currently. Reserved for future extensions.
</dd>
<dt>
<code>registration_time: u64</code>
</dt>
<dd>
 Registration timestamp in seconds
</dd>
<dt>
<code>state: u8</code>
</dt>
<dd>
 Flag indicating whether the task is active, cancelled or pending.
</dd>
<dt>
<code>locked_fee_for_next_epoch: u64</code>
</dt>
<dd>
 Deposit fee locked for the task equal to the automation-fee-cap for epoch specified for it.
 It will be refunded fully when active task is expired or cancelled by user
 and partially if a pending task is cancelled by user or an active task is cancelled by the system due to
 insufficient balance to  pay the automation fee for the epoch
</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskRegistrationFeeWithdraw"></a>

## Struct `TaskRegistrationFeeWithdraw`

Event on task registration fee withdrawal from owner account upon registration.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskRegistrationFeeWithdraw">TaskRegistrationFeeWithdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskRegistrationDepositFeeWithdraw"></a>

## Struct `TaskRegistrationDepositFeeWithdraw`

Event on task registration fee withdrawal from owner account upon registration.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskRegistrationDepositFeeWithdraw">TaskRegistrationDepositFeeWithdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>registration_fee: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_deposit_fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_RegistryFeeWithdraw"></a>

## Struct `RegistryFeeWithdraw`

Emitted on withdrawal of specified amount from automation registry fee address to the specified address.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_RegistryFeeWithdraw">RegistryFeeWithdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskEpochFeeWithdraw"></a>

## Struct `TaskEpochFeeWithdraw`

Event emitted when an automation fee is charged for an automation task for the epoch.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskEpochFeeWithdraw">TaskEpochFeeWithdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskFeeRefund"></a>

## Struct `TaskFeeRefund`

Event emitted when an automation fee is refunded for an automation task at the end of the epoch for excessive
duration paid at the beginning of the epoch due to epoch-duration reduction by governance.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskFeeRefund">TaskFeeRefund</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskDepositFeeRefund"></a>

## Struct `TaskDepositFeeRefund`

Event emitted when a deposit fee is refunded for an automation task.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskDepositFeeRefund">TaskDepositFeeRefund</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ErrorUnlockTaskDepositFee"></a>

## Struct `ErrorUnlockTaskDepositFee`

Event emitted when an automation fee is being refunded but inner state bookkeeping total locked deposits is less than
potential locked deposit for the task.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ErrorUnlockTaskDepositFee">ErrorUnlockTaskDepositFee</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_registered_deposit: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_deposit: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ErrorUnlockTaskEpochFee"></a>

## Struct `ErrorUnlockTaskEpochFee`

Event emitted when a task epoch fee is being refunded but locked epoch fees is less than
potential requested refund.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ErrorUnlockTaskEpochFee">ErrorUnlockTaskEpochFee</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_epoch_fees: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>refund: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskCancelled"></a>

## Struct `TaskCancelled`

Event emitted on automation task cancellation by owner.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskCancelled">TaskCancelled</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskCancelledV2"></a>

## Struct `TaskCancelledV2`

Event emitted on automation task cancellation by owner.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskCancelledV2">TaskCancelledV2</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>registration_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TasksStopped"></a>

## Struct `TasksStopped`

Event emitted on automation tasks stopped by owner.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TasksStopped">TasksStopped</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_TaskStopped">automation_registry::TaskStopped</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskStopped"></a>

## Struct `TaskStopped`



<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskStopped">TaskStopped</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_refund: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch_fee_refund: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TasksStoppedV2"></a>

## Struct `TasksStoppedV2`

Event emitted on automation tasks stopped by owner.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TasksStoppedV2">TasksStoppedV2</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_TaskStoppedV2">automation_registry::TaskStoppedV2</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskStoppedV2"></a>

## Struct `TaskStoppedV2`



<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskStoppedV2">TaskStoppedV2</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_refund: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch_fee_refund: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>registration_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskCancelledInsufficentBalance"></a>

## Struct `TaskCancelledInsufficentBalance`

Event emitted when an automation task is cancelled due to insufficient balance.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskCancelledInsufficentBalance">TaskCancelledInsufficentBalance</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskCancelledInsufficentBalanceV2"></a>

## Struct `TaskCancelledInsufficentBalanceV2`

Event emitted when an automation task is cancelled due to insufficient balance.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskCancelledInsufficentBalanceV2">TaskCancelledInsufficentBalanceV2</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>registration_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskCancelledCapacitySurpassed"></a>

## Struct `TaskCancelledCapacitySurpassed`

Event emitted when an automation task is cancelled due to automation fee capacity surpass.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskCancelledCapacitySurpassed">TaskCancelledCapacitySurpassed</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>automation_fee_cap: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_TaskCancelledCapacitySurpassedV2"></a>

## Struct `TaskCancelledCapacitySurpassedV2`

Event emitted when an automation task is cancelled due to automation fee capacity surpass.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_TaskCancelledCapacitySurpassedV2">TaskCancelledCapacitySurpassedV2</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>automation_fee_cap: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>registration_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_RemovedTasks"></a>

## Struct `RemovedTasks`

Event emitted on epoch transition containing removed task indexes.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_RemovedTasks">RemovedTasks</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ActiveTasks"></a>

## Struct `ActiveTasks`

Event emitted on epoch transition containing active task indexes for the new epoch.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ActiveTasks">ActiveTasks</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ErrorTaskDoesNotExist"></a>

## Struct `ErrorTaskDoesNotExist`

Event emitted when on new epoch a task is accessed with index of the task for the expected list
but value does not exist in the map


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ErrorTaskDoesNotExist">ErrorTaskDoesNotExist</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ErrorTaskDoesNotExistForWithdrawal"></a>

## Struct `ErrorTaskDoesNotExistForWithdrawal`

Event emitted when on new epoch a task is accessed with index of the task automation fee withdrawal
but it does not exist in the list.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ErrorTaskDoesNotExistForWithdrawal">ErrorTaskDoesNotExistForWithdrawal</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ErrorInsufficientBalanceToRefund"></a>

## Struct `ErrorInsufficientBalanceToRefund`

Event emitted during epoch transition when refunds to be paid is not possible due to insufficient resource account balance.
Type of the refund can be related either to the deposit paid during registration (0), or to epoch-fee caused by
the shortening of the epoch (1)


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ErrorInsufficientBalanceToRefund">ErrorInsufficientBalanceToRefund</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>refund_type: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_ErrorInconsistentSuspendedState"></a>

## Struct `ErrorInconsistentSuspendedState`

Event emitted when on new epoch inconsistent state of the registry has been identified.
When automation is in suspended state, there are no tasks expected.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_ErrorInconsistentSuspendedState">ErrorInconsistentSuspendedState</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_EnabledRegistrationEvent"></a>

## Struct `EnabledRegistrationEvent`

Emitted when the registration in the automation registry is enabled.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_EnabledRegistrationEvent">EnabledRegistrationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_DisabledRegistrationEvent"></a>

## Struct `DisabledRegistrationEvent`

Emitted when the registration in the automation registry is disabled.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_DisabledRegistrationEvent">DisabledRegistrationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_AuthorizationGranted"></a>

## Struct `AuthorizationGranted`

Emitted when the account is authorized to submit system automation tasks


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AuthorizationGranted">AuthorizationGranted</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_AuthorizationRevoked"></a>

## Struct `AuthorizationRevoked`

Emitted when the account authorization is revoked to submit system automation tasks


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AuthorizationRevoked">AuthorizationRevoked</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationTaskFeeMeta"></a>

## Struct `AutomationTaskFeeMeta`

Represents the fee charged for an automation task execution and some additional information.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">AutomationTaskFeeMeta</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>automation_fee_cap: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expiry_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>max_gas_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_deposit_fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_IntermediateState"></a>

## Struct `IntermediateState`

Represents intermediate state of the registry on epoch change.
Deprecated in production, substituted with <code><a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a></code>.
Kept for backward compatible framework upgrade.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateState">IntermediateState</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_committed_for_next_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch_locked_fees: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_IntermediateStateOfEpochChange"></a>

## Struct `IntermediateStateOfEpochChange`

Represents intermediate state of the registry on epoch change.
Deprecated in production, substituted with <code><a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a></code>.
Kept for backward compatible framework upgrade.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>removed_tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_committed_for_new_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_committed_for_next_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch_locked_fees: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_IntermediateStateOfCycleChange"></a>

## Struct `IntermediateStateOfCycleChange`

Represents intermediate state of the registry on cycle change.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>removed_tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gas_committed_for_next_cycle: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>sys_gas_committed_for_next_cycle: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch_locked_fees: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_automation_registry_AutomationTaskFee"></a>

## Struct `AutomationTaskFee`

Represents the fee charged for an automation task execution and some additional information.
Used only in tests, substituted with AutomationTaskFeeMeta in production code.
Kept for backward compatible framework upgrade.


<pre><code><b>struct</b> <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_automation_registry_MAX_U64"></a>

Max U64 value


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_automation_registry_CANCELLED"></a>



<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>: u8 = 2;
</code></pre>



<a id="0x1_automation_registry_EINSUFFICIENT_BALANCE"></a>

Insufficient balance in the resource wallet for withdrawal


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 16;
</code></pre>



<a id="0x1_automation_registry_ACTIVE"></a>



<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a>: u8 = 1;
</code></pre>



<a id="0x1_automation_registry_CYCLE_FINISHED"></a>

Triggered when cycle end is identified.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_CYCLE_FINISHED">CYCLE_FINISHED</a>: u8 = 2;
</code></pre>



<a id="0x1_automation_registry_CYCLE_READY"></a>

Constants describing CYCLE state.
State transition flow is:
CYCLE_READY -> CYCLE_STARTED
CYCLE_STARTED -> { CYCLE_FINISHED, CYCLE_SUSPENDED }
CYCLE_FINISHED ->  CYCLE_STARTED
CYCLE_SUSPENDED -> { CYCLE_READY, STARTED }


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a>: u8 = 0;
</code></pre>



<a id="0x1_automation_registry_CYCLE_STARTED"></a>

Triggered eigther when SUPRA_NATIVE_AUTOMATION feature is enabled or by registry when cycle transition is completed.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>: u8 = 1;
</code></pre>



<a id="0x1_automation_registry_CYCLE_SUSPENDED"></a>

State describing the entire lifecycle of automation being suspended.
Triggered when SUPRA_NATIVE_AUTOMATION feature is disabled.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_CYCLE_SUSPENDED">CYCLE_SUSPENDED</a>: u8 = 3;
</code></pre>



<a id="0x1_automation_registry_DECIMAL"></a>

Decimal place to make


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>: u256 = 100000000;
</code></pre>



<a id="0x1_automation_registry_DEPOSIT_EPOCH_FEE"></a>

Constants describing REFUND TYPE


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_DEPOSIT_EPOCH_FEE">DEPOSIT_EPOCH_FEE</a>: u8 = 0;
</code></pre>



<a id="0x1_automation_registry_EALREADY_CANCELLED"></a>

Task is already cancelled.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EALREADY_CANCELLED">EALREADY_CANCELLED</a>: u64 = 11;
</code></pre>



<a id="0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND"></a>

Task with provided task index not found


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>: u64 = 6;
</code></pre>



<a id="0x1_automation_registry_ECONGESTION_EXP_NON_ZERO"></a>

Congestion exponent must be non-zero.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ECONGESTION_EXP_NON_ZERO">ECONGESTION_EXP_NON_ZERO</a>: u64 = 20;
</code></pre>



<a id="0x1_automation_registry_ECYCLE_DURATION_NON_ZERO"></a>

Automation cycle duration cannot be zero.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ECYCLE_DURATION_NON_ZERO">ECYCLE_DURATION_NON_ZERO</a>: u64 = 30;
</code></pre>



<a id="0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS"></a>

Attempt to register an automation task while cycle transition is in progress.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>: u64 = 32;
</code></pre>



<a id="0x1_automation_registry_EDEPOSIT_REFUND"></a>

Failed to unlock/refund deposit for a task. Internal error, for more details see emitted error events.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EDEPOSIT_REFUND">EDEPOSIT_REFUND</a>: u64 = 27;
</code></pre>



<a id="0x1_automation_registry_EDEPRECATED_SINCE_V2"></a>

Deprecated function call since cycle based automation release.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EDEPRECATED_SINCE_V2">EDEPRECATED_SINCE_V2</a>: u64 = 29;
</code></pre>



<a id="0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE"></a>

Supra native automation feature is not initialized or enabled


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>: u64 = 15;
</code></pre>



<a id="0x1_automation_registry_EEMPTY_TASK_INDEXES"></a>

Task index list is empty.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EEMPTY_TASK_INDEXES">EEMPTY_TASK_INDEXES</a>: u64 = 25;
</code></pre>



<a id="0x1_automation_registry_EEPOCH_FEE_REFUND"></a>

Failed to unlock/refund epoch fee for a task. Internal error, for more details see emitted error events.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EEPOCH_FEE_REFUND">EEPOCH_FEE_REFUND</a>: u64 = 28;
</code></pre>



<a id="0x1_automation_registry_EEXPIRY_BEFORE_NEXT_CYCLE"></a>

Expiry time must be after the start of the next cycle


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_BEFORE_NEXT_CYCLE">EEXPIRY_BEFORE_NEXT_CYCLE</a>: u64 = 3;
</code></pre>



<a id="0x1_automation_registry_EEXPIRY_TIME_UPPER"></a>

Expiry time does not go beyond upper cap duration


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_TIME_UPPER">EEXPIRY_TIME_UPPER</a>: u64 = 2;
</code></pre>



<a id="0x1_automation_registry_EGAS_AMOUNT_UPPER"></a>

Gas amount must not go beyond upper cap limit


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EGAS_AMOUNT_UPPER">EGAS_AMOUNT_UPPER</a>: u64 = 7;
</code></pre>



<a id="0x1_automation_registry_EGAS_COMMITTEED_VALUE_OVERFLOW"></a>

The gas committed for next epoch value is overflow after adding new max gas


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_OVERFLOW">EGAS_COMMITTEED_VALUE_OVERFLOW</a>: u64 = 12;
</code></pre>



<a id="0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW"></a>

The gas committed for next epoch value is underflow after remove old max gas


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW">EGAS_COMMITTEED_VALUE_UNDERFLOW</a>: u64 = 13;
</code></pre>



<a id="0x1_automation_registry_EINCONSISTENT_TRANSITION_STATE"></a>

Attempt to process a task when expected list of the tasks has been alrady processed.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINCONSISTENT_TRANSITION_STATE">EINCONSISTENT_TRANSITION_STATE</a>: u64 = 35;
</code></pre>



<a id="0x1_automation_registry_EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH"></a>

Automation fee capacity for the epoch should not be less than estimated one.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH">EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH</a>: u64 = 21;
</code></pre>



<a id="0x1_automation_registry_EINSUFFICIENT_BALANCE_FOR_REFUND"></a>

Resource Account does not have sufficient balance to process the refund for the specified task.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE_FOR_REFUND">EINSUFFICIENT_BALANCE_FOR_REFUND</a>: u64 = 26;
</code></pre>



<a id="0x1_automation_registry_EINVALID_AUX_DATA_LENGTH"></a>

Invalid number of auxiliary data.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_AUX_DATA_LENGTH">EINVALID_AUX_DATA_LENGTH</a>: u64 = 14;
</code></pre>



<a id="0x1_automation_registry_EINVALID_EXPIRY_TIME"></a>

Invalid expiry time: it cannot be earlier than the current time


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_EXPIRY_TIME">EINVALID_EXPIRY_TIME</a>: u64 = 1;
</code></pre>



<a id="0x1_automation_registry_EINVALID_GAS_PRICE"></a>

Invalid gas price: it cannot be zero


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_GAS_PRICE">EINVALID_GAS_PRICE</a>: u64 = 4;
</code></pre>



<a id="0x1_automation_registry_EINVALID_INPUT_CYCLE_INDEX"></a>

The tasks are requested to be processed for invalid cycle.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_INPUT_CYCLE_INDEX">EINVALID_INPUT_CYCLE_INDEX</a>: u64 = 34;
</code></pre>



<a id="0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT"></a>

Invalid max gas amount for automated task: it cannot be zero


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT">EINVALID_MAX_GAS_AMOUNT</a>: u64 = 5;
</code></pre>



<a id="0x1_automation_registry_EINVALID_MIGRATION_ACTION"></a>

Attempt to do migration to cycle based automation which is already enabled.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_MIGRATION_ACTION">EINVALID_MIGRATION_ACTION</a>: u64 = 31;
</code></pre>



<a id="0x1_automation_registry_EINVALID_REGISTRY_STATE"></a>

Attempt to run operation in invalid registry state.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>: u64 = 33;
</code></pre>



<a id="0x1_automation_registry_EINVALID_TASK_TYPE"></a>

Invalid task type value. Supported 1 for user submitted tasks, 0 for system submitted tasks.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_TASK_TYPE">EINVALID_TASK_TYPE</a>: u64 = 40;
</code></pre>



<a id="0x1_automation_registry_EINVALID_TASK_TYPE_LENGTH"></a>

Task type specified as first elemeny of aux-data should have length 1


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_TASK_TYPE_LENGTH">EINVALID_TASK_TYPE_LENGTH</a>: u64 = 39;
</code></pre>



<a id="0x1_automation_registry_EINVALID_TXN_HASH"></a>

Transaction hash that registering current task is invalid. Length should be 32.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_TXN_HASH">EINVALID_TXN_HASH</a>: u64 = 9;
</code></pre>



<a id="0x1_automation_registry_EMAX_CONGESTION_THRESHOLD"></a>

Congestion threshold should not exceed 100.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EMAX_CONGESTION_THRESHOLD">EMAX_CONGESTION_THRESHOLD</a>: u64 = 19;
</code></pre>



<a id="0x1_automation_registry_EOUT_OF_ORDER_TASK_PROCESSING_REQUEST"></a>

The out of order task processing has been identified during transition.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EOUT_OF_ORDER_TASK_PROCESSING_REQUEST">EOUT_OF_ORDER_TASK_PROCESSING_REQUEST</a>: u64 = 36;
</code></pre>



<a id="0x1_automation_registry_EPOCH_FEE"></a>



<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EPOCH_FEE">EPOCH_FEE</a>: u8 = 1;
</code></pre>



<a id="0x1_automation_registry_EREGISTERED_TASK_INVALID_TYPE"></a>

Type of the registered task does not match the expected one.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTERED_TASK_INVALID_TYPE">EREGISTERED_TASK_INVALID_TYPE</a>: u64 = 43;
</code></pre>



<a id="0x1_automation_registry_EREGISTRY_IS_FULL"></a>

Registry task capacity has reached.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_IS_FULL">EREGISTRY_IS_FULL</a>: u64 = 23;
</code></pre>



<a id="0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO"></a>

Automation registry max gas capacity cannot be zero.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO">EREGISTRY_MAX_GAS_CAP_NON_ZERO</a>: u64 = 22;
</code></pre>



<a id="0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS"></a>

Automation registry max gas capacity for system tasks cannot be zero.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS">EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS</a>: u64 = 37;
</code></pre>



<a id="0x1_automation_registry_EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO"></a>

Automation registry max gas capacity for  system tasks cannot be zero.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO">EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO</a>: u64 = 46;
</code></pre>



<a id="0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE"></a>

Requested amount exceeds the locked balance


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE">EREQUEST_EXCEEDS_LOCKED_BALANCE</a>: u64 = 17;
</code></pre>



<a id="0x1_automation_registry_ESYSTEM_AUTOMATION_TASK_NOT_FOUND"></a>

Attempt to register a system task with unauthorized account.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ESYSTEM_AUTOMATION_TASK_NOT_FOUND">ESYSTEM_AUTOMATION_TASK_NOT_FOUND</a>: u64 = 42;
</code></pre>



<a id="0x1_automation_registry_ETASK_REGISTRATION_DISABLED"></a>

Task registration is currently disabled.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ETASK_REGISTRATION_DISABLED">ETASK_REGISTRATION_DISABLED</a>: u64 = 24;
</code></pre>



<a id="0x1_automation_registry_EUNACCEPTABLE_AUTOMATION_GAS_LIMIT"></a>

Current committed gas amount is greater than the automation gas limit.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_AUTOMATION_GAS_LIMIT">EUNACCEPTABLE_AUTOMATION_GAS_LIMIT</a>: u64 = 10;
</code></pre>



<a id="0x1_automation_registry_EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT"></a>

Current committed gas amount by system tasks is greater than the new system automation gas limit.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT">EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT</a>: u64 = 45;
</code></pre>



<a id="0x1_automation_registry_EUNACCEPTABLE_SYS_TASK_DURATION_CAP"></a>

Current automation cycle interval is greater than specified system task duration cap.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_SYS_TASK_DURATION_CAP">EUNACCEPTABLE_SYS_TASK_DURATION_CAP</a>: u64 = 38;
</code></pre>



<a id="0x1_automation_registry_EUNACCEPTABLE_TASK_DURATION_CAP"></a>

Current automation cycle interval is greater than specified task duration cap.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_TASK_DURATION_CAP">EUNACCEPTABLE_TASK_DURATION_CAP</a>: u64 = 18;
</code></pre>



<a id="0x1_automation_registry_EUNAUTHORIZED_SYSTEM_ACCOUNT"></a>

Attempt to register a system task with unauthorized account.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_SYSTEM_ACCOUNT">EUNAUTHORIZED_SYSTEM_ACCOUNT</a>: u64 = 41;
</code></pre>



<a id="0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER"></a>

Unauthorized access: the caller is not the owner of the task


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>: u64 = 8;
</code></pre>



<a id="0x1_automation_registry_EUNKNOWN_MULTISIG_ADDRESS"></a>

The input address is not identified as multisig account.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNKNOWN_MULTISIG_ADDRESS">EUNKNOWN_MULTISIG_ADDRESS</a>: u64 = 47;
</code></pre>



<a id="0x1_automation_registry_EUNSUPPORTED_TASK_OPERATION"></a>

Attempt to run an unsupported action for a task.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNSUPPORTED_TASK_OPERATION">EUNSUPPORTED_TASK_OPERATION</a>: u64 = 44;
</code></pre>



<a id="0x1_automation_registry_GST"></a>



<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>: u8 = 2;
</code></pre>



<a id="0x1_automation_registry_MAX_PERCENTAGE"></a>

100 Percentage


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_MAX_PERCENTAGE">MAX_PERCENTAGE</a>: u8 = 100;
</code></pre>



<a id="0x1_automation_registry_MICROSECS_CONVERSION_FACTOR"></a>

Conversion factor between microseconds and second


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_MICROSECS_CONVERSION_FACTOR">MICROSECS_CONVERSION_FACTOR</a>: u64 = 1000000;
</code></pre>



<a id="0x1_automation_registry_PENDING"></a>

Constants describing task state.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>: u8 = 0;
</code></pre>



<a id="0x1_automation_registry_PRIORITY_AUX_DATA_INDEX"></a>

Index of the aux data holding task priority value


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_PRIORITY_AUX_DATA_INDEX">PRIORITY_AUX_DATA_INDEX</a>: u64 = 1;
</code></pre>



<a id="0x1_automation_registry_REFUND_FACTOR"></a>

Defines divisor for refunds of deposit fees with penalty
Factor of <code>2</code> suggests that <code>1/2</code> of the deposit will be refunded.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_REFUND_FACTOR">REFUND_FACTOR</a>: u64 = 2;
</code></pre>



<a id="0x1_automation_registry_REFUND_FRACTION"></a>



<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_REFUND_FRACTION">REFUND_FRACTION</a>: u64 = 2;
</code></pre>



<a id="0x1_automation_registry_REGISTRY_RESOURCE_SEED"></a>

Registry resource creation seed


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_REGISTRY_RESOURCE_SEED">REGISTRY_RESOURCE_SEED</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [115, 117, 112, 114, 97, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 97, 117, 116, 111, 109, 97, 116, 105, 111, 110, 95, 114, 101, 103, 105, 115, 116, 114, 121];
</code></pre>



<a id="0x1_automation_registry_SUPPORTED_AUX_DATA_COUNT_MAX"></a>

Supported aux data count


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_SUPPORTED_AUX_DATA_COUNT_MAX">SUPPORTED_AUX_DATA_COUNT_MAX</a>: u64 = 2;
</code></pre>



<a id="0x1_automation_registry_TASK_EXECUTION_GAS"></a>

Constants defining single task processing maximum limits
Single task processing execution gas.
max_execution_gas is defined 920_000_000, where scaling factor is 1_000_000.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TASK_EXECUTION_GAS">TASK_EXECUTION_GAS</a>: u64 = 4000000;
</code></pre>



<a id="0x1_automation_registry_TASK_IO_GAS"></a>

Single task processing IO gas.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TASK_IO_GAS">TASK_IO_GAS</a>: u64 = 10000000;
</code></pre>



<a id="0x1_automation_registry_TASK_STORAGE_FEE"></a>

Max storage fee per task.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TASK_STORAGE_FEE">TASK_STORAGE_FEE</a>: u64 = 1000;
</code></pre>



<a id="0x1_automation_registry_TASK_SUPPORT_FACTOR"></a>

Task support factor in percentage. It should not exceed 100.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TASK_SUPPORT_FACTOR">TASK_SUPPORT_FACTOR</a>: u64 = 80;
</code></pre>



<a id="0x1_automation_registry_TASK_WRITE_OPS"></a>

Max write operation per task.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TASK_WRITE_OPS">TASK_WRITE_OPS</a>: u64 = 10;
</code></pre>



<a id="0x1_automation_registry_TXN_HASH_LENGTH"></a>

The length of the transaction hash.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TXN_HASH_LENGTH">TXN_HASH_LENGTH</a>: u64 = 32;
</code></pre>



<a id="0x1_automation_registry_TYPE_AUX_DATA_INDEX"></a>

Index of the aux data holding type value


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TYPE_AUX_DATA_INDEX">TYPE_AUX_DATA_INDEX</a>: u64 = 0;
</code></pre>



<a id="0x1_automation_registry_UST"></a>

Constants decribing the task type, USER SUBMITTED TASK (UST - 1), GOVERNANCE SUBMITTED TASK(GST - 2)


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>: u8 = 1;
</code></pre>



<a id="0x1_automation_registry_is_transition_finalized"></a>

## Function `is_transition_finalized`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_transition_finalized">is_transition_finalized</a>(state: &<a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_transition_finalized">is_transition_finalized</a>(state: &<a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.expected_tasks_to_be_processed) == state.next_task_index_position
}
</code></pre>



</details>

<a id="0x1_automation_registry_is_transition_in_progress"></a>

## Function `is_transition_in_progress`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_transition_in_progress">is_transition_in_progress</a>(state: &<a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_transition_in_progress">is_transition_in_progress</a>(state: &<a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>): bool {
    state.next_task_index_position != 0
}
</code></pre>



</details>

<a id="0x1_automation_registry_mark_task_processed"></a>

## Function `mark_task_processed`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_mark_task_processed">mark_task_processed</a>(state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>, task_index: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_mark_task_processed">mark_task_processed</a>(state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>, task_index: u64) {
    <b>assert</b>!(state.next_task_index_position &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.expected_tasks_to_be_processed), <a href="automation_registry.md#0x1_automation_registry_EINCONSISTENT_TRANSITION_STATE">EINCONSISTENT_TRANSITION_STATE</a>);
    <b>let</b> expected_task = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&state.expected_tasks_to_be_processed, state.next_task_index_position);
    <b>assert</b>!(expected_task == &task_index, <a href="automation_registry.md#0x1_automation_registry_EOUT_OF_ORDER_TASK_PROCESSING_REQUEST">EOUT_OF_ORDER_TASK_PROCESSING_REQUEST</a>);
    state.next_task_index_position = state.next_task_index_position + 1;
}
</code></pre>



</details>

<a id="0x1_automation_registry_is_of_type"></a>

## Function `is_of_type`



<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>, type: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>, type: u8): bool {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&task.aux_data) == <a href="automation_registry.md#0x1_automation_registry_SUPPORTED_AUX_DATA_COUNT_MAX">SUPPORTED_AUX_DATA_COUNT_MAX</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_AUX_DATA_LENGTH">EINVALID_AUX_DATA_LENGTH</a>);
    <b>let</b> type_data = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&task.aux_data, <a href="automation_registry.md#0x1_automation_registry_TYPE_AUX_DATA_INDEX">TYPE_AUX_DATA_INDEX</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(type_data) == 1, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TASK_TYPE_LENGTH">EINVALID_TASK_TYPE_LENGTH</a>);
    <b>let</b> type_value = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(type_data, 0);
    *type_value == type
}
</code></pre>



</details>

<a id="0x1_automation_registry_is_initialized"></a>

## Function `is_initialized`

Checks whether all required resources are created.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>(): bool {
    <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework)
}
</code></pre>



</details>

<a id="0x1_automation_registry_is_feature_enabled_and_initialized"></a>

## Function `is_feature_enabled_and_initialized`

Means to query by user whether the automation registry has been properly initialized and ready to be utilized.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_feature_enabled_and_initialized">is_feature_enabled_and_initialized</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_feature_enabled_and_initialized">is_feature_enabled_and_initialized</a>(): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>() && <a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>()
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_next_task_index"></a>

## Function `get_next_task_index`

Returns next task index in registry


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_next_task_index">get_next_task_index</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_next_task_index">get_next_task_index</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.current_index
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_task_count"></a>

## Function `get_task_count`

Returns number of available tasks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_count">get_task_count</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_count">get_task_count</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_length">enumerable_map::length</a>(&state.main.tasks)
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_system_task_count"></a>

## Function `get_system_task_count`

Returns number of available system tasks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_task_count">get_system_task_count</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_task_count">get_system_task_count</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.system_tasks_state.task_ids)
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_task_ids"></a>

## Function `get_task_ids`

List all automation task ids available in register.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_ids">get_task_ids</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_ids">get_task_ids</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&state.main.tasks)
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_epoch_locked_balance"></a>

## Function `get_epoch_locked_balance`

Get locked balance of the resource account in terms of epoch-fees


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_epoch_locked_balance">get_epoch_locked_balance</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_epoch_locked_balance">get_epoch_locked_balance</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_locked_fees
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_locked_deposit_balance"></a>

## Function `get_locked_deposit_balance`

Get locked balance of the resource account in terms of deposited automation fees.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_locked_deposit_balance">get_locked_deposit_balance</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_locked_deposit_balance">get_locked_deposit_balance</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    <b>let</b> refund_bookkeeping = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    refund_bookkeeping.total_deposited_automation_fee
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_registry_total_locked_balance"></a>

## Function `get_registry_total_locked_balance`

Get total locked balance of the resource account.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_registry_total_locked_balance">get_registry_total_locked_balance</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_registry_total_locked_balance">get_registry_total_locked_balance</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <a href="automation_registry.md#0x1_automation_registry_get_epoch_locked_balance">get_epoch_locked_balance</a>() + <a href="automation_registry.md#0x1_automation_registry_get_locked_deposit_balance">get_locked_deposit_balance</a>()
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_active_task_ids"></a>

## Function `get_active_task_ids`

List all active automation task ids for the current epoch.
Note that the tasks with CANCELLED state are still considered active for the current epoch,
as cancellation takes effect in the next epoch only.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_active_task_ids">get_active_task_ids</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_active_task_ids">get_active_task_ids</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    state.main.epoch_active_task_ids
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_task_details"></a>

## Function `get_task_details`

Retrieves the details of a automation task entry by its task index.
Error will be returned if entry with specified task index does not exist.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_details">get_task_details</a>(task_index: u64): <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_details">get_task_details</a>(task_index: u64): <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry_state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&registry_state.main.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&registry_state.main.tasks, task_index)
}
</code></pre>



</details>

<a id="0x1_automation_registry_deconstruct_task_metadata"></a>

## Function `deconstruct_task_metadata`

Retrieves specific metadata details of an automation task entry by its task index.

1. <code><b>address</b></code>                 - The owner of the task.
2. <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>              - The payload transaction (encoded).
3. <code>u64</code>                     - The expiry time of the task (timestamp).
4. <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>              - The hash of the transaction.
5. <code>u64</code>                     - The maximum gas amount allowed for the task.
6. <code>u64</code>                     - The gas price cap for executing the task.
7. <code>u64</code>                     - The automation fee cap for the current epoch.
8. <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>      - Auxiliary data related to the task (can be multiple items).
9. <code>u64</code>                     - The time at which the task was registered (timestamp).
10. <code>u8</code>                     - The state of the task (e.g., active, cancelled, completed).
11. <code>u64</code>                    - The locked fee reserved for the next epoch execution.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_deconstruct_task_metadata">deconstruct_task_metadata</a>(task_metadata: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>): (<b>address</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64, u64, u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, u64, u8, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_deconstruct_task_metadata">deconstruct_task_metadata</a>(
    task_metadata: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>
): (<b>address</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64, u64, u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, u64, u8, u64) {
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
</code></pre>



</details>

<a id="0x1_automation_registry_get_task_owner"></a>

## Function `get_task_owner`

Retrieves the owner address of a task by its task index.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_owner">get_task_owner</a>(task_index: u64): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_owner">get_task_owner</a>(task_index: u64): <b>address</b> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> automation_task_metadata = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&automation_task_metadata.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);
    <b>let</b> task_metadata = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&automation_task_metadata.tasks, task_index);
    task_metadata.owner
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_task_details_bulk"></a>

## Function `get_task_details_bulk`

Retrieves the details of a automation tasks entry by their task index.
If a task does not exist, it is not included in the result, and no error is reported


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_details_bulk">get_task_details_bulk</a>(task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_details_bulk">get_task_details_bulk</a>(task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry_state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <b>let</b> task_details = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_indexes, |task_index| {
        <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&registry_state.main.tasks, task_index)) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> task_details, <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&registry_state.main.tasks, task_index))
        }
    });
    task_details
}
</code></pre>



</details>

<a id="0x1_automation_registry_has_sender_active_task_with_id"></a>

## Function `has_sender_active_task_with_id`

Checks whether there is an active task in registry with specified input task index.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id">has_sender_active_task_with_id</a>(sender: <b>address</b>, task_index: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id">has_sender_active_task_with_id</a>(sender: <b>address</b>, task_index: u64): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id_and_type">has_sender_active_task_with_id_and_type</a>(sender, task_index, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_has_sender_active_system_task_with_id"></a>

## Function `has_sender_active_system_task_with_id`

Checks whether there is an active system task in registry with specified input task index.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_system_task_with_id">has_sender_active_system_task_with_id</a>(sender: <b>address</b>, task_index: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_system_task_with_id">has_sender_active_system_task_with_id</a>(sender: <b>address</b>, task_index: u64): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id_and_type">has_sender_active_task_with_id_and_type</a>(sender, task_index, <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_has_sender_active_task_with_id_and_type"></a>

## Function `has_sender_active_task_with_id_and_type`

Checks whether there is an active task in registry with specified input task index of the input type.
The type can be either 1 for user submitted tasks, and 2 for governance authorized tasks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id_and_type">has_sender_active_task_with_id_and_type</a>(sender: <b>address</b>, task_index: u64, type: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id_and_type">has_sender_active_task_with_id_and_type</a>(sender: <b>address</b>, task_index: u64, type: u8): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry_state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&registry_state.main.tasks, task_index)) {
        <b>let</b> value = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_ref">enumerable_map::get_value_ref</a>(&registry_state.main.tasks, task_index);
        value.state != <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a> && value.owner == sender && <a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(value, type)
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_registry_fee_address"></a>

## Function `get_registry_fee_address`

Get registry fee resource account address


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_registry_fee_address">get_registry_fee_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_registry_fee_address">get_registry_fee_address</a>(): <b>address</b> {
    <a href="account.md#0x1_account_create_resource_address">account::create_resource_address</a>(&@supra_framework, <a href="automation_registry.md#0x1_automation_registry_REGISTRY_RESOURCE_SEED">REGISTRY_RESOURCE_SEED</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_gas_committed_for_next_epoch"></a>

## Function `get_gas_committed_for_next_epoch`

Get gas committed for next epoch


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_gas_committed_for_next_epoch">get_gas_committed_for_next_epoch</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_gas_committed_for_next_epoch">get_gas_committed_for_next_epoch</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_next_epoch
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_gas_committed_for_current_epoch"></a>

## Function `get_gas_committed_for_current_epoch`

Get gas committed for the current epoch at the beginning of the epoch.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_gas_committed_for_current_epoch">get_gas_committed_for_current_epoch</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_gas_committed_for_current_epoch">get_gas_committed_for_current_epoch</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_this_epoch <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_automation_registry_config"></a>

## Function `get_automation_registry_config`

Get automation registry configuration


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_registry_config">get_automation_registry_config</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_registry_config">get_automation_registry_config</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework).main_config
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_automation_registry_config_for_system_tasks"></a>

## Function `get_automation_registry_config_for_system_tasks`

Get automation registry configuration for system tasks


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_registry_config_for_system_tasks">get_automation_registry_config_for_system_tasks</a>(): <a href="automation_registry.md#0x1_automation_registry_RegistryConfigForSystemTasks">automation_registry::RegistryConfigForSystemTasks</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_registry_config_for_system_tasks">get_automation_registry_config_for_system_tasks</a>(): <a href="automation_registry.md#0x1_automation_registry_RegistryConfigForSystemTasks">RegistryConfigForSystemTasks</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework).system_task_config
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_next_epoch_registry_max_gas_cap"></a>

## Function `get_next_epoch_registry_max_gas_cap`

Get automation registry maximum gas capacity for the next epoch


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_next_epoch_registry_max_gas_cap">get_next_epoch_registry_max_gas_cap</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_next_epoch_registry_max_gas_cap">get_next_epoch_registry_max_gas_cap</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework).next_cycle_registry_max_gas_cap
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_automation_epoch_info"></a>

## Function `get_automation_epoch_info`

Get automation epoch info


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_epoch_info">get_automation_epoch_info</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_epoch_info">get_automation_epoch_info</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
    <b>assert</b>!(<b>false</b>, <a href="automation_registry.md#0x1_automation_registry_EDEPRECATED_SINCE_V2">EDEPRECATED_SINCE_V2</a>);
    <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
        expected_epoch_duration: 0,
        epoch_interval: 0,
        start_time: 0,

    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_estimate_automation_fee"></a>

## Function `estimate_automation_fee`

Estimates automation fee for the next epoch for specified task occupancy for the configured epoch-interval
referencing the current automation registry fee parameters, current total occupancy and registry maximum allowed
occupancy for the next epoch.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee">estimate_automation_fee</a>(task_occupancy: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee">estimate_automation_fee</a>(
    task_occupancy: u64
): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy">estimate_automation_fee_with_committed_occupancy</a>(task_occupancy, registry.main.gas_committed_for_next_epoch)
}
</code></pre>



</details>

<a id="0x1_automation_registry_estimate_automation_fee_with_committed_occupancy"></a>

## Function `estimate_automation_fee_with_committed_occupancy`

Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
referencing the current automation registry fee parameters, specified total/committed occupancy and registry
maximum allowed occupancy for the next epoch.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy">estimate_automation_fee_with_committed_occupancy</a>(task_occupancy: u64, committed_occupancy: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy">estimate_automation_fee_with_committed_occupancy</a>(
    task_occupancy: u64,
    committed_occupancy: u64
): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>let</b> cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>let</b> config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(
        task_occupancy,
        committed_occupancy,
        cycle_info.duration_secs,
        config
    )
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_automation_fee_multiplier_for_committed_occupancy"></a>

## Function `calculate_automation_fee_multiplier_for_committed_occupancy`

Calculates automation fee per second for the specified task occupancy
referencing the current automation registry fee parameters, specified total/committed occupancy and current registry
maximum allowed occupancy.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_committed_occupancy">calculate_automation_fee_multiplier_for_committed_occupancy</a>(total_committed_max_gas: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_committed_occupancy">calculate_automation_fee_multiplier_for_committed_occupancy</a>(
    total_committed_max_gas: u64
): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    // Compute the automation fee multiplier for cycle
    <b>let</b> active_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <b>let</b> automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(
        &active_config.main_config,
        (total_committed_max_gas <b>as</b> u256),
        active_config.main_config.registry_max_gas_cap);
    (automation_fee_per_sec <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle"></a>

## Function `calculate_automation_fee_multiplier_for_current_cycle`

Calculates automation fee per second for the current cycle
referencing the current automation registry fee parameters, and committed gas for this cycle stored in
the automation registry and current maximum allowed occupancy.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle">calculate_automation_fee_multiplier_for_current_cycle</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle">calculate_automation_fee_multiplier_for_current_cycle</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    // Compute the automation fee multiplier for this cycle
    <b>let</b> active_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle_internal">calculate_automation_fee_multiplier_for_current_cycle_internal</a>(active_config, &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main)
}
</code></pre>



</details>

<a id="0x1_automation_registry_is_registration_enabled"></a>

## Function `is_registration_enabled`

Returns the current status of the registration in the automation registry.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_registration_enabled">is_registration_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_registration_enabled">is_registration_enabled</a>(): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework).registration_enabled
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_cycle_duration"></a>

## Function `get_cycle_duration`

Returns the current duration of the automation cycle.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_cycle_duration">get_cycle_duration</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_cycle_duration">get_cycle_duration</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework).duration_secs
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_cycle_info"></a>

## Function `get_cycle_info`

Returns the current cycle info.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_cycle_info">get_cycle_info</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">automation_registry::AutomationCycleInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_cycle_info">get_cycle_info</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">AutomationCycleInfo</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> {
    <b>let</b> details = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_into_automation_cycle_info">into_automation_cycle_info</a>(details)
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_record_max_task_count"></a>

## Function `get_record_max_task_count`

Returns the maximum number of the tasks that can be processed in scope of single bookkeeping transaction.


<pre><code>#[view]
<b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_record_max_task_count">get_record_max_task_count</a>(max_execution_gas: u64, max_io_gas: u64, max_storage_fee: u64, max_write_op: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_record_max_task_count">get_record_max_task_count</a>(max_execution_gas: u64, max_io_gas: u64, max_storage_fee: u64, max_write_op: u64): u64 {
    <b>let</b> task_count_by_exec_gas = max_execution_gas / <a href="automation_registry.md#0x1_automation_registry_TASK_EXECUTION_GAS">TASK_EXECUTION_GAS</a>;
    <b>let</b> task_count_by_io_gas = max_io_gas / <a href="automation_registry.md#0x1_automation_registry_TASK_IO_GAS">TASK_IO_GAS</a>;
    <b>let</b> task_count_by_storage_fee = max_storage_fee / <a href="automation_registry.md#0x1_automation_registry_TASK_STORAGE_FEE">TASK_STORAGE_FEE</a>;
    <b>let</b> task_count_by_write_op = max_write_op / <a href="automation_registry.md#0x1_automation_registry_TASK_WRITE_OPS">TASK_WRITE_OPS</a>;

    <b>let</b> task_count = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_min">math64::min</a>(task_count_by_exec_gas, task_count_by_io_gas);
    task_count = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_min">math64::min</a>(task_count, task_count_by_storage_fee);
    task_count = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_min">math64::min</a>(task_count, task_count_by_write_op);
    task_count * <a href="automation_registry.md#0x1_automation_registry_TASK_SUPPORT_FACTOR">TASK_SUPPORT_FACTOR</a> / 100
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_system_task_indexes"></a>

## Function `get_system_task_indexes`

List of system registered tasks


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_task_indexes">get_system_task_indexes</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_task_indexes">get_system_task_indexes</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    registry.system_tasks_state.task_ids
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_system_gas_committed_for_next_cycle"></a>

## Function `get_system_gas_committed_for_next_cycle`

Get committed gas for the next cycle by system tasks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_gas_committed_for_next_cycle">get_system_gas_committed_for_next_cycle</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_gas_committed_for_next_cycle">get_system_gas_committed_for_next_cycle</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    registry.system_tasks_state.gas_committed_for_next_cycle
}
</code></pre>



</details>

<a id="0x1_automation_registry_get_system_gas_committed_for_current_cycle"></a>

## Function `get_system_gas_committed_for_current_cycle`

Get committed gas for the current cycle by system tasks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_gas_committed_for_current_cycle">get_system_gas_committed_for_current_cycle</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_system_gas_committed_for_current_cycle">get_system_gas_committed_for_current_cycle</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    registry.system_tasks_state.gas_committed_for_this_cycle
}
</code></pre>



</details>

<a id="0x1_automation_registry_is_authorized_account"></a>

## Function `is_authorized_account`

Checks whether the input account address is authorized.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_authorized_account">is_authorized_account</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_authorized_account">is_authorized_account</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&registry.system_tasks_state.authorized_accounts, &<a href="account.md#0x1_account">account</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_withdraw_automation_task_fees"></a>

## Function `withdraw_automation_task_fees`

Withdraw accumulated automation task fees from the resource account - access by admin


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_withdraw_automation_task_fees">withdraw_automation_task_fees</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_withdraw_automation_task_fees">withdraw_automation_task_fees</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <b>to</b>: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> , <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_transfer_fee_to_account_internal">transfer_fee_to_account_internal</a>(<b>to</b>, amount);
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_RegistryFeeWithdraw">RegistryFeeWithdraw</a> { <b>to</b>, amount });
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_config"></a>

## Function `update_config`

Update Automation Registry Config


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config">update_config</a>(_supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _task_duration_cap_in_secs: u64, _registry_max_gas_cap: u64, _automation_base_fee_in_quants_per_sec: u64, _flat_registration_fee_in_quants: u64, _congestion_threshold_percentage: u8, _congestion_base_fee_in_quants_per_sec: u64, _congestion_exponent: u8, _task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config">update_config</a>(
    _supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _task_duration_cap_in_secs: u64,
    _registry_max_gas_cap: u64,
    _automation_base_fee_in_quants_per_sec: u64,
    _flat_registration_fee_in_quants: u64,
    _congestion_threshold_percentage: u8,
    _congestion_base_fee_in_quants_per_sec: u64,
    _congestion_exponent: u8,
    _task_capacity: u16,
) {
    <b>assert</b>!(<b>false</b>, <a href="automation_registry.md#0x1_automation_registry_EDEPRECATED_SINCE_V2">EDEPRECATED_SINCE_V2</a>);
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_config_v2"></a>

## Function `update_config_v2`

Update Automation Registry Config along with cycle duration.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_v2">update_config_v2</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, automation_base_fee_in_quants_per_sec: u64, flat_registration_fee_in_quants: u64, congestion_threshold_percentage: u8, congestion_base_fee_in_quants_per_sec: u64, congestion_exponent: u8, task_capacity: u16, cycle_duration_secs: u64, sys_task_duration_cap_in_secs: u64, sys_registry_max_gas_cap: u64, sys_task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_v2">update_config_v2</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(
        cycle_duration_secs,
        task_duration_cap_in_secs,
        sys_task_duration_cap_in_secs,
        registry_max_gas_cap,
        sys_registry_max_gas_cap,
        congestion_threshold_percentage,
        congestion_exponent);

    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);

    <b>assert</b>!(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_next_epoch &lt;= registry_max_gas_cap,
        <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_AUTOMATION_GAS_LIMIT">EUNACCEPTABLE_AUTOMATION_GAS_LIMIT</a>
    );

    <b>assert</b>!(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle &lt;= sys_registry_max_gas_cap,
        <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT">EUNACCEPTABLE_SYSTEM_AUTOMATION_GAS_LIMIT</a>
    );

    <b>let</b> new_automation_registry_config = <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfigV2">AutomationRegistryConfigV2</a> {
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
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<b>copy</b> new_automation_registry_config);

    // next cyle registry max gas cap will be <b>update</b> instantly
    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    automation_registry_config.next_cycle_registry_max_gas_cap = registry_max_gas_cap;
    automation_registry_config.next_cycle_sys_registry_max_gas_cap = sys_registry_max_gas_cap;

    <a href="event.md#0x1_event_emit">event::emit</a>(new_automation_registry_config);
}
</code></pre>



</details>

<a id="0x1_automation_registry_enable_registration"></a>

## Function `enable_registration`

Enables the registration process in the automation registry.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_enable_registration">enable_registration</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_enable_registration">enable_registration</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    automation_registry_config.registration_enabled = <b>true</b>;
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_EnabledRegistrationEvent">EnabledRegistrationEvent</a> {});
}
</code></pre>



</details>

<a id="0x1_automation_registry_disable_registration"></a>

## Function `disable_registration`

Disables the registration process in the automation registry.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_disable_registration">disable_registration</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_disable_registration">disable_registration</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    automation_registry_config.registration_enabled = <b>false</b>;
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_DisabledRegistrationEvent">DisabledRegistrationEvent</a> {});
}
</code></pre>



</details>

<a id="0x1_automation_registry_grant_authorization"></a>

## Function `grant_authorization`

Grants authorization to the input account to submit system automation tasks.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_grant_authorization">grant_authorization</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_grant_authorization">grant_authorization</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> system_tasks_state = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).system_tasks_state;
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&system_tasks_state.authorized_accounts, &<a href="account.md#0x1_account">account</a>)) {
        <b>return</b>
    };
    <b>assert</b>!(<a href="multisig_account.md#0x1_multisig_account_account_exists">multisig_account::account_exists</a>(<a href="account.md#0x1_account">account</a>), <a href="automation_registry.md#0x1_automation_registry_EUNKNOWN_MULTISIG_ADDRESS">EUNKNOWN_MULTISIG_ADDRESS</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> system_tasks_state.authorized_accounts, <a href="account.md#0x1_account">account</a>);
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_AuthorizationGranted">AuthorizationGranted</a> {
        <a href="account.md#0x1_account">account</a>
    })
}
</code></pre>



</details>

<a id="0x1_automation_registry_revoke_authorization"></a>

## Function `revoke_authorization`

Revoke authorization from the input account to submit system automation tasks.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_revoke_authorization">revoke_authorization</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_revoke_authorization">revoke_authorization</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> system_tasks_state = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).system_tasks_state;
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&system_tasks_state.authorized_accounts, &<a href="account.md#0x1_account">account</a>)) {
        <b>return</b>
    };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove_value">vector::remove_value</a>(&<b>mut</b> system_tasks_state.authorized_accounts, &<a href="account.md#0x1_account">account</a>);
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_AuthorizationRevoked">AuthorizationRevoked</a> {
        <a href="account.md#0x1_account">account</a>
    })
}
</code></pre>



</details>

<a id="0x1_automation_registry_cancel_task"></a>

## Function `cancel_task`

Cancel Automation task with specified task_index.
Only existing task, which is PENDING or ACTIVE, can be cancelled and only by task owner.
If the task is
- active, its state is updated to be CANCELLED.
- pending, it is removed form the list.
- cancelled, an error is reported
Committed gas-limit is updated by reducing it with the max-gas-amount of the cancelled task.


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cancel_task">cancel_task</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_index: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cancel_task">cancel_task</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_index: u64
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>{
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>let</b> cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).main;
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);

    <b>let</b> automation_task_metadata = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&automation_task_metadata, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>), <a href="automation_registry.md#0x1_automation_registry_EUNSUPPORTED_TASK_OPERATION">EUNSUPPORTED_TASK_OPERATION</a>);
    <b>assert</b>!(automation_task_metadata.owner == owner, <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>);
    <b>assert</b>!(automation_task_metadata.state != <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>, <a href="automation_registry.md#0x1_automation_registry_EALREADY_CANCELLED">EALREADY_CANCELLED</a>);
    <b>if</b> (automation_task_metadata.state == <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
        <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
            &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
        );
        // When Pending tasks are cancelled, refund of the deposit fee is done <b>with</b> penalty
        <b>let</b> result = <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
            refund_bookkeeping,
            &resource_signer,
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
            automation_task_metadata.task_index,
            owner,
            automation_task_metadata.locked_fee_for_next_epoch / <a href="automation_registry.md#0x1_automation_registry_REFUND_FACTOR">REFUND_FACTOR</a>,
            automation_task_metadata.locked_fee_for_next_epoch);
        <b>assert</b>!(result, <a href="automation_registry.md#0x1_automation_registry_EDEPOSIT_REFUND">EDEPOSIT_REFUND</a>);
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
    } <b>else</b> { // it is safe not <b>to</b> check the state <b>as</b> above, the cancelled tasks are already rejected.
        // Active tasks will be refunded the deposited amount fully at the end of the epoch
        <b>let</b> automation_task_metadata_mut = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(
            &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks,
            task_index
        );
        automation_task_metadata_mut.state = <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>;
    };

    // This check means the task was expected <b>to</b> be executed in the next cycle, but it <b>has</b> been cancelled.
    // We need <b>to</b> remove its gas commitment from `gas_committed_for_next_epoch` for this particular task.
    <b>if</b> (automation_task_metadata.expiry_time &gt; (cycle_info.start_time + cycle_info.duration_secs)) {
        <b>assert</b>!(
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch &gt;= automation_task_metadata.max_gas_amount,
            <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW">EGAS_COMMITTEED_VALUE_UNDERFLOW</a>
        );
        // Adjust the gas committed for the next epoch by subtracting the gas amount of the cancelled task
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch - automation_task_metadata.max_gas_amount;
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledV2">TaskCancelledV2</a> { task_index: automation_task_metadata.task_index, owner, registration_hash: automation_task_metadata.tx_hash });
}
</code></pre>



</details>

<a id="0x1_automation_registry_stop_tasks"></a>

## Function `stop_tasks`

Immediately stops automation tasks for the specified <code>task_indexes</code>.
Only tasks that exist and are owned by the sender can be stopped.
If any of the specified tasks are not owned by the sender, the transaction will abort.
When a task is stopped, the committed gas for the next epoch is reduced
by the max gas amount of the stopped task. Half of the remaining task fee is refunded.


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_stop_tasks">stop_tasks</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_stop_tasks">stop_tasks</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>let</b> cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>);
    // Ensure that task indexes are provided
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&task_indexes), <a href="automation_registry.md#0x1_automation_registry_EEMPTY_TASK_INDEXES">EEMPTY_TASK_INDEXES</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).main;
    <b>let</b> arc = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework).main_config;
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);

    <b>let</b> tcmg = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch;

    // Compute the automation fee multiplier for epoch
    <b>let</b> automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(&arc, tcmg, arc.registry_max_gas_cap);

    <b>let</b> stopped_task_details = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> total_refund_fee = 0;
    <b>let</b> epoch_locked_fees = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees;

    // Calculate refundable fee for this remaining time task in current epoch
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> cycle_end_time = cycle_info.duration_secs + cycle_info.start_time;
    <b>let</b> residual_interval = <b>if</b> (cycle_end_time &lt;= current_time) {
        0
    } <b>else</b> {
        cycle_end_time - current_time
    };

    // Loop through each task index <b>to</b> validate and stop the task
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_indexes, |task_index| {
        <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index)) {
            // Remove task from registry
            <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
            <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&task, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>), <a href="automation_registry.md#0x1_automation_registry_EUNSUPPORTED_TASK_OPERATION">EUNSUPPORTED_TASK_OPERATION</a>);

            // Ensure only the task owner can stop it
            <b>assert</b>!(task.owner == owner, <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>);

            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove_value">vector::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids, &task_index);

            // This check means the task was expected <b>to</b> be executed in the next cycle, but it <b>has</b> been stopped.
            // We need <b>to</b> remove its gas commitment from `gas_committed_for_next_cycle` for this particular task.
            // Also it checks that task should not be cancelled.
            <b>if</b> (task.state != <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a> && task.expiry_time &gt; cycle_end_time) {
                // Prevent underflow in gas committed
                <b>assert</b>!(
                    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch &gt;= task.max_gas_amount,
                    <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW">EGAS_COMMITTEED_VALUE_UNDERFLOW</a>
                );

                // Reduce committed gas by the stopped task's max gas
                <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch - task.max_gas_amount;
            };

            <b>let</b> (epoch_fee_refund, deposit_refund) = <b>if</b> (task.state != <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
                <b>let</b> task_fee = <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(
                    &arc,
                    &task,
                    residual_interval,
                    current_time,
                    automation_fee_per_sec
                );
                // Refund full deposit and the half of the remaining run-time fee when task is active or cancelled stage
                (task_fee / <a href="automation_registry.md#0x1_automation_registry_REFUND_FRACTION">REFUND_FRACTION</a>, task.locked_fee_for_next_epoch)
            } <b>else</b> {
                (0, (task.locked_fee_for_next_epoch / <a href="automation_registry.md#0x1_automation_registry_REFUND_FRACTION">REFUND_FRACTION</a>))
            };
            <b>let</b> result = <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_deposit">safe_unlock_locked_deposit</a>(
                refund_bookkeeping,
                task.locked_fee_for_next_epoch,
                task.task_index);
            <b>assert</b>!(result, <a href="automation_registry.md#0x1_automation_registry_EDEPOSIT_REFUND">EDEPOSIT_REFUND</a>);
            <b>let</b> (result, remaining_epoch_locked_fees) = <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_epoch_fee">safe_unlock_locked_epoch_fee</a>(
                epoch_locked_fees,
                epoch_fee_refund,
                task.task_index);
            <b>assert</b>!(result, <a href="automation_registry.md#0x1_automation_registry_EEPOCH_FEE_REFUND">EEPOCH_FEE_REFUND</a>);
            epoch_locked_fees = remaining_epoch_locked_fees;

            total_refund_fee = total_refund_fee + (epoch_fee_refund + deposit_refund);

            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(
                &<b>mut</b> stopped_task_details,
                <a href="automation_registry.md#0x1_automation_registry_TaskStoppedV2">TaskStoppedV2</a> { task_index, deposit_refund, epoch_fee_refund, registration_hash: task.tx_hash }
            );
        }
    });

    // Refund and emit <a href="event.md#0x1_event">event</a> <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> tasks were stopped
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&stopped_task_details)) {
        <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
            &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
        );

        <b>let</b> resource_account_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address);
        <b>assert</b>!(resource_account_balance &gt;= total_refund_fee, <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE_FOR_REFUND">EINSUFFICIENT_BALANCE_FOR_REFUND</a>);
        <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(&resource_signer, owner, total_refund_fee);

        // Emit task stopped <a href="event.md#0x1_event">event</a>
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TasksStoppedV2">TasksStoppedV2</a> {
            tasks: stopped_task_details,
            owner
        });
    };
}
</code></pre>



</details>

<a id="0x1_automation_registry_stop_system_tasks"></a>

## Function `stop_system_tasks`

Immediately stops system automation tasks for the specified <code>task_indexes</code>.
Only tasks that exist and are owned by the sender can be stopped.
If any of the specified tasks are not owned by the sender, the transaction will abort.
When a task is stopped, the committed gas for the next epoch is reduced
by the max gas amount of the stopped task.


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_stop_system_tasks">stop_system_tasks</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_stop_system_tasks">stop_system_tasks</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>let</b> cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>);
    // Ensure that task indexes are provided
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&task_indexes), <a href="automation_registry.md#0x1_automation_registry_EEMPTY_TASK_INDEXES">EEMPTY_TASK_INDEXES</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);

    <b>let</b> stopped_task_details = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // Calculate refundable fee for this remaining time task in current epoch
    <b>let</b> cycle_end_time = cycle_info.duration_secs + cycle_info.start_time;

    // Loop through each task index <b>to</b> validate and stop the task
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_indexes, |task_index| {
        <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index)) {
            // Remove task from registry
            <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);

            // Ensure only the task owner can stop it
            <b>assert</b>!(task.owner == owner, <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>);
            <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&task, <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>), <a href="automation_registry.md#0x1_automation_registry_EUNSUPPORTED_TASK_OPERATION">EUNSUPPORTED_TASK_OPERATION</a>);

            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove_value">vector::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_active_task_ids, &task_index);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove_value">vector::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.task_ids, &task_index);

            // This check means the task was expected <b>to</b> be executed in the next cycle, but it <b>has</b> been stopped.
            // We need <b>to</b> remove its gas commitment from `gas_committed_for_next_cycle` for this particular task.
            // Also it checks that task should not be cancelled.
            <b>if</b> (task.state != <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a> && task.expiry_time &gt; cycle_end_time) {
                // Prevent underflow in gas committed
                <b>assert</b>!(
                    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle &gt;= task.max_gas_amount,
                    <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW">EGAS_COMMITTEED_VALUE_UNDERFLOW</a>
                );

                // Reduce committed gas by the stopped task's max gas
                <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle =
                    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle - task.max_gas_amount;
            };


            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(
                &<b>mut</b> stopped_task_details,
                <a href="automation_registry.md#0x1_automation_registry_TaskStoppedV2">TaskStoppedV2</a> { task_index, deposit_refund: 0, epoch_fee_refund: 0, registration_hash: task.tx_hash }
            );
        }
    });

    // Refund and emit <a href="event.md#0x1_event">event</a> <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> tasks were stopped
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&stopped_task_details)) {
        // Emit task stopped <a href="event.md#0x1_event">event</a>
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TasksStoppedV2">TasksStoppedV2</a> {
            tasks: stopped_task_details,
            owner
        });
    };
}
</code></pre>



</details>

<a id="0x1_automation_registry_cancel_system_task"></a>

## Function `cancel_system_task`

Cancel System automation task with specified task_index.
Only existing task, which is PENDING or ACTIVE, can be cancelled and only by task owner.
If the task is
- active, its state is updated to be CANCELLED.
- pending, it is removed form the list.
- cancelled, an error is reported
Committed gas-limit is updated by reducing it with the max-gas-amount of the cancelled task.


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cancel_system_task">cancel_system_task</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_index: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cancel_system_task">cancel_system_task</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_index: u64
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>let</b> cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.task_ids, &task_index), <a href="automation_registry.md#0x1_automation_registry_ESYSTEM_AUTOMATION_TASK_NOT_FOUND">ESYSTEM_AUTOMATION_TASK_NOT_FOUND</a>);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);

    <b>let</b> automation_task_metadata = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);
    <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&automation_task_metadata, <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>), <a href="automation_registry.md#0x1_automation_registry_EUNSUPPORTED_TASK_OPERATION">EUNSUPPORTED_TASK_OPERATION</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>assert</b>!(automation_task_metadata.owner == owner, <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>);
    <b>assert</b>!(automation_task_metadata.state != <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>, <a href="automation_registry.md#0x1_automation_registry_EALREADY_CANCELLED">EALREADY_CANCELLED</a>);
    <b>if</b> (automation_task_metadata.state == <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);
    } <b>else</b> { // it is safe not <b>to</b> check the state <b>as</b> above, the cancelled tasks are already rejected.
        <b>let</b> automation_task_metadata_mut = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(
            &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks,
            task_index
        );
        automation_task_metadata_mut.state = <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>;
    };

    // This check means the task was expected <b>to</b> be executed in the next cycle, but it <b>has</b> been cancelled.
    // We need <b>to</b> remove its gas commitment from `gas_committed_for_next_epoch` for this particular task.
    <b>if</b> (automation_task_metadata.expiry_time &gt; (cycle_info.start_time + cycle_info.duration_secs)) {
        <b>assert</b>!(
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle &gt;= automation_task_metadata.max_gas_amount,
            <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW">EGAS_COMMITTEED_VALUE_UNDERFLOW</a>
        );
        // Adjust the gas committed for the next epoch by subtracting the gas amount of the cancelled task
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle =
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle - automation_task_metadata.max_gas_amount;
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledV2">TaskCancelledV2</a> { task_index: automation_task_metadata.task_index, owner, registration_hash: automation_task_metadata.tx_hash });
}
</code></pre>



</details>

<a id="0x1_automation_registry_initialize_refund_bookkeeping_resource"></a>

## Function `initialize_refund_bookkeeping_resource`

Public entry function to initialize bookeeping resource when feature enabling automation deposit fee charges is released.


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_initialize_refund_bookkeeping_resource">initialize_refund_bookkeeping_resource</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_initialize_refund_bookkeeping_resource">initialize_refund_bookkeeping_resource</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
        total_deposited_automation_fee: 0
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_migrate_v2"></a>

## Function `migrate_v2`

API to gracfully migrate from automation feature v1 inplementation to v2 where bookkeeping of the tasks is
detached from epoch-change and cycle based lifecycle of the automation registry is enabled and
tasks are updated to have UST task-type.
IMPORTANT: Should always be followed by <code>SUPRA_AUTOMATION_V2</code> feature flag being enabled and
supra_governance::reconfiguration otherwise registry/chain will end-up in inconsistent state.

monitor_cycle_end (block_prologue->automation_registry::monitor_cycle_end) which will lead to panic and node will stop
thus not causing any inconcistensy in the chain


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_migrate_v2">migrate_v2</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cycle_duration_secs: u64, sys_task_duration_cap_in_secs: u64, sys_registry_max_gas_cap: u64, sys_task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_migrate_v2">migrate_v2</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cycle_duration_secs: u64,
    sys_task_duration_cap_in_secs: u64,
    sys_registry_max_gas_cap: u64,
    sys_task_capacity: u16
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>{
    assert_supra_framework(supra_framework);
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_automation_v2_enabled">features::supra_automation_v2_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EINVALID_MIGRATION_ACTION">EINVALID_MIGRATION_ACTION</a>);
    <b>assert</b>!(<b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework), <a href="automation_registry.md#0x1_automation_registry_EINVALID_MIGRATION_ACTION">EINVALID_MIGRATION_ACTION</a>);
    <a href="automation_registry.md#0x1_automation_registry_validate_system_configuration_parameters_common">validate_system_configuration_parameters_common</a>(cycle_duration_secs, sys_task_duration_cap_in_secs, sys_registry_max_gas_cap);

    // Prepare the state for migration
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>move_from</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> automation_epoch_info = <b>move_from</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);

    <b>let</b> automation_registry_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(
        @supra_framework
    ).main_config;

    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    // Refund the epoch fees <b>as</b> epoch will be cut short and on_new_epoch will be dummy due <b>to</b> migration,
    // so this is the only place <b>to</b> do the refunds
    <a href="automation_registry.md#0x1_automation_registry_update_state_for_migration">update_state_for_migration</a>(
        &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        &automation_registry_config,
        automation_epoch_info,
        current_time
    );

    // Initializing the cycle releated resouces
    <b>let</b> id = 0;
    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> {
        start_time: current_time,
        index: id,
        duration_secs: cycle_duration_secs,
        state: <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a>,
        transition_state: std::option::none()
    });

    <a href="automation_registry.md#0x1_automation_registry_migrate_registry_config">migrate_registry_config</a>(supra_framework, sys_task_duration_cap_in_secs,  sys_registry_max_gas_cap, sys_task_capacity);

    // Initialize registry state for system tasks and new AutomtionRegistryV2 holding both system task and general registry state
    <a href="automation_registry.md#0x1_automation_registry_migrate_registry_state">migrate_registry_state</a>(supra_framework, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>);

    // Remain in <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a> state <b>if</b> feature is not enabled or registry is not fully initialized
    <b>if</b> (!<a href="automation_registry.md#0x1_automation_registry_is_feature_enabled_and_initialized">is_feature_enabled_and_initialized</a>()) {
        <b>return</b>
    };
    // Emit cycle end which will lead the <b>native</b> layer <b>to</b> start preparation <b>to</b> the new cycle.
    <b>let</b> cycle_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    // Update the config <b>to</b> start the cycle <b>with</b> new config.
    <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer_for_migration">update_config_from_buffer_for_migration</a>(cycle_info);
    <a href="automation_registry.md#0x1_automation_registry_on_cycle_end_internal">on_cycle_end_internal</a>(cycle_info);
}
</code></pre>



</details>

<a id="0x1_automation_registry_initialize"></a>

## Function `initialize`

Initialization of Automation Registry with configuration parameters for SUPRA_AUTOMATION_V2 version.
Expected to have this function call either at genesis startup or as part of the SUPRA_FRAMEWORK upgrade where
automation feature is being introduced very first time utilizing <code><a href="genesis.md#0x1_genesis_initialize_supra_native_automation_v2">genesis::initialize_supra_native_automation_v2</a></code>.
In case if framework upgrade is happening on the chain where automation feature with epoch based lifecycle is
already released and is in ongoing state, then <code>migrate_v2</code> function should be utilized instead.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cycle_duration_secs: u64, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, automation_base_fee_in_quants_per_sec: u64, flat_registration_fee_in_quants: u64, congestion_threshold_percentage: u8, congestion_base_fee_in_quants_per_sec: u64, congestion_exponent: u8, task_capacity: u16, sys_task_duration_cap_in_secs: u64, sys_registry_max_gas_cap: u64, sys_task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_initialize">initialize</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(
        cycle_duration_secs,
        task_duration_cap_in_secs,
        sys_task_duration_cap_in_secs,
        registry_max_gas_cap,
        sys_registry_max_gas_cap,
        congestion_threshold_percentage,
        congestion_exponent);

    <b>let</b> (registry_fee_resource_signer, registry_fee_address_signer_cap) = <a href="automation_registry.md#0x1_automation_registry_create_registry_resource_account">create_registry_resource_account</a>(
        supra_framework
    );

    <b>let</b> system_tasks_state =  <a href="automation_registry.md#0x1_automation_registry_RegistryStateForSystemTasks">RegistryStateForSystemTasks</a> {
        gas_committed_for_this_cycle: 0,
        gas_committed_for_next_cycle: 0,
        authorized_accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    };

    <b>let</b> general_registry_state = <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
        tasks: <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_new_map">enumerable_map::new_map</a>(),
        current_index: 0,
        gas_committed_for_next_epoch: 0,
        epoch_locked_fees: 0,
        gas_committed_for_this_epoch: 0,
        registry_fee_address: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&registry_fee_resource_signer),
        registry_fee_address_signer_cap,
        epoch_active_task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    };

    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
        main: general_registry_state,
        system_tasks_state
    });

    <b>let</b> system_task_config =  <a href="automation_registry.md#0x1_automation_registry_RegistryConfigForSystemTasks">RegistryConfigForSystemTasks</a> {
        task_duration_cap_in_secs: sys_task_duration_cap_in_secs,
        registry_max_gas_cap: sys_registry_max_gas_cap,
        task_capacity: sys_task_capacity,
        aux_properties: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>(),
    };
    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
        main_config: <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a> {
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
        registration_enabled: <b>true</b>,
        system_task_config,
        aux_configs: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>()
    });

    <b>let</b> (cycle_state, cycle_id) =
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_automation_v2_enabled">features::supra_automation_v2_enabled</a>() && <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>()) {
            (<a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, 1)
        } <b>else</b> {
            (<a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a>, 0)
        };

    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a> {
        start_time: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        index: cycle_id,
        duration_secs: cycle_duration_secs,
        state: cycle_state,
        transition_state: std::option::none&lt;<a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>&gt;(),
    });

    <a href="automation_registry.md#0x1_automation_registry_initialize_refund_bookkeeping_resource">initialize_refund_bookkeeping_resource</a>(supra_framework);

}
</code></pre>



</details>

<a id="0x1_automation_registry_monitor_cycle_end"></a>

## Function `monitor_cycle_end`

Checks the cycle end and emit an event on it.
Does nothing if SUPRA_NATIVE_AUTOMATION or SUPRA_AUTOMATION_V2 is disabled.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_monitor_cycle_end">monitor_cycle_end</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_monitor_cycle_end">monitor_cycle_end</a>() <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>if</b> (!<a href="automation_registry.md#0x1_automation_registry_is_feature_enabled_and_initialized">is_feature_enabled_and_initialized</a>() || !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_automation_v2_enabled">features::supra_automation_v2_enabled</a>()) {
        <b>return</b>
    };
    <a href="automation_registry.md#0x1_automation_registry_assert_automation_cycle_management_support">assert_automation_cycle_management_support</a>();
    <b>let</b> cycle_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>if</b> (cycle_info.state != <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>
        || cycle_info.start_time + cycle_info.duration_secs &gt; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()) {
        <b>return</b>
    };
    <a href="automation_registry.md#0x1_automation_registry_on_cycle_end_internal">on_cycle_end_internal</a>(cycle_info)
}
</code></pre>



</details>

<a id="0x1_automation_registry_on_new_epoch"></a>

## Function `on_new_epoch`

On new epoch will be triggered for automation registry caused by <code>supra_governance::reconfiguration</code> or DKG finalization
to update the automation registry state depending on SUPRA_NATIVE_AUTOMATION feature flag state.

If registry is not fully initialized nothing is done.

If native automation feature is disabled and automation cycle in CYCLE_STARTED state,
then automation lifecycle is suspended immediately. And detached managment will
initiate reprocessing of the available tasks which will end up in refund and cealnup actions.

Otherwise suspention is postponed untill the end of the transition state.

Nothing will be done if automation cycle was already suspneded, i.e. in CYCLE_READY state.

If native automation feature is enabled and automation lifecycle has been in CYCLE_READY state,
then lifecycle is restarted.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_new_epoch">on_new_epoch</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_new_epoch">on_new_epoch</a>() <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>if</b> (!<a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>() || !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_automation_v2_enabled">features::supra_automation_v2_enabled</a>()) {
        <b>return</b>
    };
    <b>let</b> cycle_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>let</b> general_registry_data = &<b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).main;
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>()) {
        // If the lifecycle <b>has</b> been suspended and we are recovering from it, then we <b>update</b> config from buffer and
        // then start a new cycle directly.
        // Unless we are in <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a> state, the feature flag being enabled will not have <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> effect.
        // All the other states mean that we are in the middle of previous transition, which should end
        // before reenabling the feature.
        <b>if</b> (cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a>) {
            <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_length">enumerable_map::length</a>(&general_registry_data.tasks) != 0) {
                <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_ErrorInconsistentSuspendedState">ErrorInconsistentSuspendedState</a> {});
                <b>return</b>
            };
            <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info);
            <a href="automation_registry.md#0x1_automation_registry_move_to_started_state">move_to_started_state</a>(cycle_info);
        };
        <b>return</b>
    };

    // We do not <b>update</b> config here, <b>as</b> due <b>to</b> feature being disabled, cycle ends early so it is expected
    // that the current fee-parameters will be used <b>to</b> calculate automation-fee for refund for a cycle
    // that <b>has</b> been kept short.
    // So the confing should remain intact.
    <b>if</b> (cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>) {
        <a href="automation_registry.md#0x1_automation_registry_try_move_to_suspended_state">try_move_to_suspended_state</a>(general_registry_data, cycle_info);
    } <b>else</b> <b>if</b> (cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_FINISHED">CYCLE_FINISHED</a> && std::option::is_some(&cycle_info.transition_state)) {
        <b>let</b> trasition_state = std::option::borrow(&cycle_info.transition_state);
        <b>if</b> (!<a href="automation_registry.md#0x1_automation_registry_is_transition_in_progress">is_transition_in_progress</a>(trasition_state)) {
            // Just entered cycle-end phase, and meanwhile also feature <b>has</b> been disabled so it is safe <b>to</b> <b>move</b> <b>to</b> suspended state.
            <a href="automation_registry.md#0x1_automation_registry_try_move_to_suspended_state">try_move_to_suspended_state</a>(general_registry_data, cycle_info);
        }
        // Otherwise wait of the cycle transition <b>to</b> end and then feature flag value will be taken into <a href="account.md#0x1_account">account</a>.
    }
    // If in already SUSPENED state or in READY state then do nothing.
}
</code></pre>



</details>

<a id="0x1_automation_registry_register"></a>

## Function `register`

Registers a new automation task entry.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_register">register</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, payload_tx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, expiry_time: u64, max_gas_amount: u64, gas_price_cap: u64, automation_fee_cap_for_epoch: u64, tx_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aux_data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_register">register</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    payload_tx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    expiry_time: u64,
    max_gas_amount: u64,
    gas_price_cap: u64,
    automation_fee_cap_for_epoch: u64,
    tx_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    aux_data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    // Guarding registration <b>if</b> feature is not enabled.
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>let</b> has_no_priority = <a href="automation_registry.md#0x1_automation_registry_check_and_validate_aux_data">check_and_validate_aux_data</a>(&aux_data, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>);

    <b>let</b> automation_registry_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <b>let</b> automation_cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(automation_registry_config.registration_enabled, <a href="automation_registry.md#0x1_automation_registry_ETASK_REGISTRATION_DISABLED">ETASK_REGISTRATION_DISABLED</a>);
    <b>assert</b>!(automation_cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>);

    // If registry is full, reject task registration
    <b>assert</b>!((<a href="automation_registry.md#0x1_automation_registry_get_task_count">get_task_count</a>() <b>as</b> u16) &lt; automation_registry_config.main_config.task_capacity, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_IS_FULL">EREGISTRY_IS_FULL</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).main;

    //Well-formedness check of payload_tx is done in <b>native</b> layer beforehand.

    <b>let</b> registration_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <a href="automation_registry.md#0x1_automation_registry_validate_task_duration">validate_task_duration</a>(
        expiry_time,
        registration_time,
        automation_registry_config,
        automation_cycle_info,
        <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>
    );

    <b>assert</b>!(gas_price_cap &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EINVALID_GAS_PRICE">EINVALID_GAS_PRICE</a>);
    <b>assert</b>!(max_gas_amount &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT">EINVALID_MAX_GAS_AMOUNT</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&tx_hash) == <a href="automation_registry.md#0x1_automation_registry_TXN_HASH_LENGTH">TXN_HASH_LENGTH</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TXN_HASH">EINVALID_TXN_HASH</a>);

    <b>let</b> committed_gas = (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch <b>as</b> u128) + (max_gas_amount <b>as</b> u128);
    <b>assert</b>!(committed_gas &lt;= <a href="automation_registry.md#0x1_automation_registry_MAX_U64">MAX_U64</a>, <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_OVERFLOW">EGAS_COMMITTEED_VALUE_OVERFLOW</a>);

    <b>let</b> committed_gas = (committed_gas <b>as</b> u64);
    <b>assert</b>!(committed_gas &lt;= automation_registry_config.next_cycle_registry_max_gas_cap, <a href="automation_registry.md#0x1_automation_registry_EGAS_AMOUNT_UPPER">EGAS_AMOUNT_UPPER</a>);

    // Check the automation fee capacity
    <b>let</b> estimated_automation_fee_for_epoch = <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(
        max_gas_amount,
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch,
        automation_cycle_info.duration_secs,
        automation_registry_config);
    <b>assert</b>!(automation_fee_cap_for_epoch &gt;= estimated_automation_fee_for_epoch,
        <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH">EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH</a>
    );

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = committed_gas;
    <b>let</b> task_index = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index;

    <b>if</b> (has_no_priority) {
        <b>let</b> priority = std::bcs::to_bytes(&task_index);
        <a href="../../supra-stdlib/doc/vector_utils.md#0x1_vector_utils_replace">vector_utils::replace</a>(&<b>mut</b> aux_data, <a href="automation_registry.md#0x1_automation_registry_PRIORITY_AUX_DATA_INDEX">PRIORITY_AUX_DATA_INDEX</a>, priority);
    };

    <b>let</b> automation_task_metadata = <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> {
        task_index,
        owner,
        payload_tx,
        expiry_time,
        max_gas_amount,
        gas_price_cap,
        automation_fee_cap_for_epoch,
        aux_data,
        state: <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>,
        registration_time,
        tx_hash,
        locked_fee_for_next_epoch: automation_fee_cap_for_epoch
    };

    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_add_value">enumerable_map::add_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index, automation_task_metadata);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index + 1;

    // Charge flat registration fee from the user at the time of registration and deposit for automation_fee for epoch.
    <b>let</b> fee = automation_registry_config.main_config.flat_registration_fee_in_quants + automation_fee_cap_for_epoch;

    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    refund_bookkeeping.total_deposited_automation_fee = refund_bookkeeping.total_deposited_automation_fee + automation_fee_cap_for_epoch;

    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(owner_signer, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address, fee);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskRegistrationDepositFeeWithdraw">TaskRegistrationDepositFeeWithdraw</a> {
        task_index,
        owner,
        registration_fee: automation_registry_config.main_config.flat_registration_fee_in_quants ,
        locked_deposit_fee: automation_fee_cap_for_epoch
    });
    <a href="event.md#0x1_event_emit">event::emit</a>(automation_task_metadata);
}
</code></pre>



</details>

<a id="0x1_automation_registry_register_system_task"></a>

## Function `register_system_task`

Registers a new system automation task entry.
Note, system tasks are not charged registration and deposit fee.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_register_system_task">register_system_task</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, payload_tx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, expiry_time: u64, max_gas_amount: u64, tx_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, aux_data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_register_system_task">register_system_task</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    payload_tx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    expiry_time: u64,
    max_gas_amount: u64,
    tx_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    aux_data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    // Guarding registration <b>if</b> feature is not enabled.
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>let</b> has_no_priority = <a href="automation_registry.md#0x1_automation_registry_check_and_validate_aux_data">check_and_validate_aux_data</a>(&aux_data, <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>);

    <b>let</b> automation_registry_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <b>let</b> automation_cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(automation_registry_config.registration_enabled, <a href="automation_registry.md#0x1_automation_registry_ETASK_REGISTRATION_DISABLED">ETASK_REGISTRATION_DISABLED</a>);
    <b>assert</b>!(automation_cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_TRANSITION_IN_PROGRESS">ECYCLE_TRANSITION_IN_PROGRESS</a>);

    // If registry is full, reject task registration
    <b>assert</b>!((<a href="automation_registry.md#0x1_automation_registry_get_system_task_count">get_system_task_count</a>() <b>as</b> u16) &lt; automation_registry_config.system_task_config.task_capacity, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_IS_FULL">EREGISTRY_IS_FULL</a>);

    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.authorized_accounts, &owner),
        <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_SYSTEM_ACCOUNT">EUNAUTHORIZED_SYSTEM_ACCOUNT</a>
    );

    //Well-formedness check of payload_tx is done in <b>native</b> layer beforehand.

    <b>let</b> registration_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <a href="automation_registry.md#0x1_automation_registry_validate_task_duration">validate_task_duration</a>(
        expiry_time,
        registration_time,
        automation_registry_config,
        automation_cycle_info,
        <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>
    );

    <b>assert</b>!(max_gas_amount &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT">EINVALID_MAX_GAS_AMOUNT</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&tx_hash) == <a href="automation_registry.md#0x1_automation_registry_TXN_HASH_LENGTH">TXN_HASH_LENGTH</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TXN_HASH">EINVALID_TXN_HASH</a>);

    <b>let</b> committed_gas = (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle <b>as</b> u128) + (max_gas_amount <b>as</b> u128);
    <b>assert</b>!(committed_gas &lt;= <a href="automation_registry.md#0x1_automation_registry_MAX_U64">MAX_U64</a>, <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_OVERFLOW">EGAS_COMMITTEED_VALUE_OVERFLOW</a>);

    <b>let</b> committed_gas = (committed_gas <b>as</b> u64);
    <b>assert</b>!(committed_gas &lt;= automation_registry_config.next_cycle_sys_registry_max_gas_cap, <a href="automation_registry.md#0x1_automation_registry_EGAS_AMOUNT_UPPER">EGAS_AMOUNT_UPPER</a>);

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle = committed_gas;
    <b>let</b> task_index = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.current_index;
    <b>if</b> (has_no_priority) {
        <b>let</b> priority = std::bcs::to_bytes(&task_index);
        <a href="../../supra-stdlib/doc/vector_utils.md#0x1_vector_utils_replace">vector_utils::replace</a>(&<b>mut</b> aux_data, <a href="automation_registry.md#0x1_automation_registry_PRIORITY_AUX_DATA_INDEX">PRIORITY_AUX_DATA_INDEX</a>, priority);
    };

    <b>let</b> automation_task_metadata = <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> {
        task_index,
        owner,
        payload_tx,
        expiry_time,
        max_gas_amount,
        // No max gas price, <b>as</b> system tasks are not charged
        gas_price_cap: 0,
        // No Automation fee cap, <b>as</b> system tasks are not charged
        automation_fee_cap_for_epoch: 0,
        aux_data,
        state: <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>,
        registration_time,
        tx_hash,
        // No deposit fee <b>as</b> system tasks are not charged
        locked_fee_for_next_epoch: 0
    };

    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_add_value">enumerable_map::add_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index, automation_task_metadata);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.current_index = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.current_index + 1;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.task_ids, task_index);

    <a href="event.md#0x1_event_emit">event::emit</a>(automation_task_metadata);
}
</code></pre>



</details>

<a id="0x1_automation_registry_process_tasks"></a>

## Function `process_tasks`

Called by MoveVm on <code>AutomationBookkeepingAction::Process</code> action emitted by native layer ahead of cycle transition


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_process_tasks">process_tasks</a>(vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, cycle_index: u64, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_process_tasks">process_tasks</a>(
    vm: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    cycle_index: u64,
    task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    // Operational constraint: can only be invoked by the VM
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(&vm);
    <b>let</b> cycle_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>if</b> (cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_FINISHED">CYCLE_FINISHED</a>) {
        <a href="automation_registry.md#0x1_automation_registry_on_cycle_transition">on_cycle_transition</a>(cycle_index, task_indexes);
        <b>return</b>
    };
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_SUSPENDED">CYCLE_SUSPENDED</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
    <a href="automation_registry.md#0x1_automation_registry_on_cycle_suspend">on_cycle_suspend</a>(cycle_index, task_indexes);
}
</code></pre>



</details>

<a id="0x1_automation_registry_on_cycle_transition"></a>

## Function `on_cycle_transition`

Traverses the list of the tasks and based on the task state and expiry information either charges or drops
the task after refunding eligable fees.

Input cycle index corresponds to the new cycle to which the transition is being done.

Tasks are cheked not to be processed more than once.
This function should be called only if registry is in CYCLE_FINISHED state, meaning a normal cycle transition is
happening.

After processing all input tasks, intermediate transition state is updated and transition end is check
(whether all expected tasks has been processed already).

In case if transition end is detected a start of the new cycle is given
(if during trasition period suspention is not requested) and corresponding event is emitted.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_cycle_transition">on_cycle_transition</a>(cycle_index: u64, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_cycle_transition">on_cycle_transition</a>(cycle_index: u64, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
<b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&task_indexes)) {
        <b>return</b>
    };

    <b>let</b> cycle_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_FINISHED">CYCLE_FINISHED</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
    <b>assert</b>!(std::option::is_some(&cycle_info.transition_state), <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
    <b>assert</b>!(cycle_info.index + 1 == cycle_index, <a href="automation_registry.md#0x1_automation_registry_EINVALID_INPUT_CYCLE_INDEX">EINVALID_INPUT_CYCLE_INDEX</a>);

    <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);

    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    <b>let</b> automation_registry_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <b>let</b> intermedate_result = <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a> {
        removed_tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        gas_committed_for_next_cycle: 0,
        sys_gas_committed_for_next_cycle: 0,
        epoch_locked_fees: <a href="coin.md#0x1_coin_zero">coin::zero</a>()
    };

    <a href="automation_registry.md#0x1_automation_registry_drop_or_charge_tasks">drop_or_charge_tasks</a>(
        task_indexes,
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        refund_bookkeeping,
        transition_state,
        &automation_registry_config.main_config,
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        &<b>mut</b> intermedate_result
    );
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a> {
        removed_tasks,
        gas_committed_for_next_cycle,
        sys_gas_committed_for_next_cycle,
        epoch_locked_fees
    } = intermedate_result;

    transition_state.locked_fees = transition_state.locked_fees + <a href="coin.md#0x1_coin_value">coin::value</a>(&epoch_locked_fees);
    transition_state.gas_committed_for_next_cycle = transition_state.gas_committed_for_next_cycle + gas_committed_for_next_cycle;
    transition_state.sys_gas_committed_for_next_cycle = transition_state.sys_gas_committed_for_next_cycle + sys_gas_committed_for_next_cycle;
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.registry_fee_address, epoch_locked_fees);

    <a href="automation_registry.md#0x1_automation_registry_update_cycle_transition_state_from_finished">update_cycle_transition_state_from_finished</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, cycle_info);

    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&removed_tasks)) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_RemovedTasks">RemovedTasks</a>{
            task_indexes: removed_tasks
        })
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_on_cycle_suspend"></a>

## Function `on_cycle_suspend`

Traverses the list of the tasks and refunds automation(if not PENDING) and depoist fees for all tasks
and removes from registry.

Input cycle index corresponds to the cycle being suspended.

This function is called only if automation feature is disabled, i.e. CYCLE_SUSPENDED state.

After processing input set of tasks the end of suspention process is checked(i.e. all expected tasks has been processed).
In case if end is identified the registry state is update to CYCLE_READY and corresponding event is emitted.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_cycle_suspend">on_cycle_suspend</a>(cycle_index: u64, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_cycle_suspend">on_cycle_suspend</a>(cycle_index: u64, task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; )
<b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&task_indexes)) {
        <b>return</b>
    };

    <b>let</b> cycle_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>&gt;(@supra_framework);
    <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_SUSPENDED">CYCLE_SUSPENDED</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
    <b>assert</b>!(std::option::is_some(&cycle_info.transition_state), <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
    <b>assert</b>!(cycle_info.index == cycle_index, <a href="automation_registry.md#0x1_automation_registry_EINVALID_INPUT_CYCLE_INDEX">EINVALID_INPUT_CYCLE_INDEX</a>);
    <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);


    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    <b>let</b> arc = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.registry_fee_address_signer_cap
    );
    <b>let</b> removed_tasks = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> epoch_locked_fees = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_locked_fees;
    // Sort task indexes <b>as</b> order is important
    task_indexes = sort_vector_u64(task_indexes);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_indexes, |task_index| {
        <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index)) {
            <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);
            <a href="automation_registry.md#0x1_automation_registry_mark_task_processed">mark_task_processed</a>(transition_state, task_index);
            // Nothing <b>to</b> refund for <a href="automation_registry.md#0x1_automation_registry_GST">GST</a> tasks
            <b>if</b> (<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&task, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>)) {
                epoch_locked_fees = <a href="automation_registry.md#0x1_automation_registry_refund_task_fees">refund_task_fees</a>(
                    task,
                    &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main,
                    refund_bookkeeping,
                    arc,
                    transition_state,
                    &resource_signer,
                    epoch_locked_fees,
                    current_time,
                    &<b>mut</b> removed_tasks
                )
            };
        }
    });

    <a href="automation_registry.md#0x1_automation_registry_update_cycle_transition_state_from_suspended">update_cycle_transition_state_from_suspended</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, cycle_info);
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_RemovedTasks">RemovedTasks</a> {
        task_indexes: removed_tasks
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_drop_or_charge_tasks"></a>

## Function `drop_or_charge_tasks`

Traverses all input task indexes and either drops or tries to charge automation fee if possible.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_drop_or_charge_tasks">drop_or_charge_tasks</a>(task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, transition_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, current_time: u64, intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">automation_registry::IntermediateStateOfCycleChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_drop_or_charge_tasks">drop_or_charge_tasks</a>(
    task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    transition_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    current_time: u64,
    intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a>,
) {

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.registry_fee_address_signer_cap
    );
    <b>let</b> current_cycle_end_time = current_time + transition_state.new_cycle_duration;

    // Sort task indexes <b>to</b> charge automation fees in the tasks chronological order
    task_ids = sort_vector_u64(task_ids);

    // Process each active task and calculate fee for the epoch for the tasks
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_ids, |task_index| {
        <a href="automation_registry.md#0x1_automation_registry_drop_or_charge_task">drop_or_charge_task</a>(
            task_index,
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
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
</code></pre>



</details>

<a id="0x1_automation_registry_drop_or_charge_task"></a>

## Function `drop_or_charge_task`

Drops or charges the input task.
If the task is already processed or missing from the registry then nothing is done.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_drop_or_charge_task">drop_or_charge_task</a>(task_index: u64, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, transition_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, current_time: u64, current_cycle_end_time: u64, intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">automation_registry::IntermediateStateOfCycleChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_drop_or_charge_task">drop_or_charge_task</a>(
    task_index: u64,
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    transition_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    current_time: u64,
    current_cycle_end_time: u64,
    intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a>,
)
{
    <b>if</b> (!<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index)) {
        <b>return</b>
    };
    <a href="automation_registry.md#0x1_automation_registry_mark_task_processed">mark_task_processed</a>(transition_state, task_index);
    <b>let</b> task_meta = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);
    <b>if</b> (task_meta.state == <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a> || task_meta.expiry_time &lt;= current_time) {
        <b>if</b> (<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(task_meta, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>)) {
            <a href="automation_registry.md#0x1_automation_registry_refund_deposit_and_drop">refund_deposit_and_drop</a>(task_index, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, refund_bookkeeping, resource_signer, &<b>mut</b> intermediate_state.removed_tasks);
        } <b>else</b> {
            <a href="automation_registry.md#0x1_automation_registry_drop_system_task">drop_system_task</a>(task_index, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, &<b>mut</b> intermediate_state.removed_tasks)
        };
        <b>return</b>
    } <b>else</b> <b>if</b> (<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(task_meta, <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>)) {
        // Governance submitted tasks are not charged
        intermediate_state.sys_gas_committed_for_next_cycle = intermediate_state.sys_gas_committed_for_next_cycle + task_meta.max_gas_amount;
        task_meta.state = <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a>;
        <b>return</b>
    };

    <b>let</b> fee= <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(
        arc,
        task_meta,
        transition_state.new_cycle_duration,
        current_time,
        (transition_state.automation_fee_per_sec <b>as</b> u256));
    // If the task reached this phase that means it is valid active task for the new epoch.
    // During cleanup all expired tasks <b>has</b> been removed from the registry but the state of the tasks is not updated.
    // As here we need <b>to</b> distinguish new tasks from already existing active tasks,
    // <b>as</b> the fee calculation for them will be different based on their active duration in the epoch.
    // For more details see calculate_task_fee function.
    task_meta.state = <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a>;
    <b>let</b> task = <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">AutomationTaskFeeMeta</a> {
        task_index,
        owner: task_meta.owner,
        fee,
        expiry_time: task_meta.expiry_time,
        automation_fee_cap: task_meta.automation_fee_cap_for_epoch,
        max_gas_amount: task_meta.max_gas_amount,
        locked_deposit_fee: task_meta.locked_fee_for_next_epoch,
    };
    <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        refund_bookkeeping,
        resource_signer,
        task,
        current_cycle_end_time,
        intermediate_state
    );
}
</code></pre>



</details>

<a id="0x1_automation_registry_refund_deposit_and_drop"></a>

## Function `refund_deposit_and_drop`

Refunds the deposit fee of the task and removes from registry.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_deposit_and_drop">refund_deposit_and_drop</a>(task_index: u64, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, removed_tasks: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_deposit_and_drop">refund_deposit_and_drop</a>(
    task_index: u64,
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    removed_tasks: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; ) : <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> {
    <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);
    <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&task, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>), <a href="automation_registry.md#0x1_automation_registry_EREGISTERED_TASK_INVALID_TYPE">EREGISTERED_TASK_INVALID_TYPE</a>);
    <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
        refund_bookkeeping,
        resource_signer,
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.registry_fee_address,
        task_index,
        task.owner,
        task.locked_fee_for_next_epoch,
        task.locked_fee_for_next_epoch);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(removed_tasks, task_index);
    task
}
</code></pre>



</details>

<a id="0x1_automation_registry_drop_system_task"></a>

## Function `drop_system_task`

Removes system task from registry state.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_drop_system_task">drop_system_task</a>(task_index: u64, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, removed_tasks: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_drop_system_task">drop_system_task</a>(
    task_index: u64,
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    removed_tasks: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
)  {
    <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task_index);
    <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&task, <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>), <a href="automation_registry.md#0x1_automation_registry_EREGISTERED_TASK_INVALID_TYPE">EREGISTERED_TASK_INVALID_TYPE</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove_value">vector::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.task_ids, &task.task_index);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(removed_tasks, task_index);
}
</code></pre>



</details>

<a id="0x1_automation_registry_refund_task_fees"></a>

## Function `refund_task_fees`

Refunds the deposit fee and any autoamtion fees of the task.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_task_fees">refund_task_fees</a>(task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">automation_registry::ActiveAutomationRegistryConfigV2</a>, transition_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">automation_registry::TransitionState</a>, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_locked_fees: u64, current_time: u64, removed_tasks: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_task_fees">refund_task_fees</a>(
    task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>,
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>,
    transition_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    epoch_locked_fees: u64,
    current_time: u64,
    removed_tasks: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;

) : u64 {
    <b>assert</b>!(<a href="automation_registry.md#0x1_automation_registry_is_of_type">is_of_type</a>(&task, <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>), <a href="automation_registry.md#0x1_automation_registry_EREGISTERED_TASK_INVALID_TYPE">EREGISTERED_TASK_INVALID_TYPE</a>);
    // Do not attempt fee refund <b>if</b> remaining duration is 0
    <b>if</b> (task.state != <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a> && transition_state.refund_duration != 0) {
        <b>let</b> refund = <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(
            &arc.main_config,
            &task,
            transition_state.refund_duration,
            current_time,
            (transition_state.automation_fee_per_sec <b>as</b> u256));
        <b>let</b> (_, remaining_epoch_locked_fees) = <a href="automation_registry.md#0x1_automation_registry_safe_fee_refund">safe_fee_refund</a>(
            epoch_locked_fees,
            resource_signer,
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
            task.task_index,
            task.owner,
            refund);
        epoch_locked_fees = remaining_epoch_locked_fees;
    };

    <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
        refund_bookkeeping,
        resource_signer,
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
        task.task_index,
        task.owner,
        task.locked_fee_for_next_epoch,
        task.locked_fee_for_next_epoch);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(removed_tasks, task.task_index);
    epoch_locked_fees
}
</code></pre>



</details>

<a id="0x1_automation_registry_into_automation_cycle_info"></a>

## Function `into_automation_cycle_info`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_into_automation_cycle_info">into_automation_cycle_info</a>(details: &<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>): <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">automation_registry::AutomationCycleInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_into_automation_cycle_info">into_automation_cycle_info</a>(details: &<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>): <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">AutomationCycleInfo</a> {
    <a href="automation_registry.md#0x1_automation_registry_AutomationCycleInfo">AutomationCycleInfo</a> {
        index: details.index,
        state: details.state,
        start_time: details.start_time,
        duration_secs: details.duration_secs
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_cycle_transition_state_from_suspended"></a>

## Function `update_cycle_transition_state_from_suspended`

Updates the cycle state if the transition is identified to be finalized.

As transition happens from suspended state and while transition was in progress
- if the feature was enabled back, then the transition will happen direclty to starated state,
- otherwise the transition will be done to the ready state.

In both cases config will be updated. In this case we will make sure to keep the consistency of state
when transition to ready state happens through paths
- Started -> Suspended -> Ready
- or Started-> {Finished, Suspended} -> Ready
- or Started -> Finished -> {Started, Suspended}


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_cycle_transition_state_from_suspended">update_cycle_transition_state_from_suspended</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_cycle_transition_state_from_suspended">update_cycle_transition_state_from_suspended</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>,
) <b>acquires</b>  <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>  {
    <b>assert</b>!(std::option::is_some(&cycle_info.transition_state), <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
    <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);

    <b>if</b> (!<a href="automation_registry.md#0x1_automation_registry_is_transition_finalized">is_transition_finalized</a>(transition_state)) {
        <b>return</b>
    };

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_this_cycle = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.task_ids = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_next_epoch = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_this_epoch = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_active_task_ids = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_locked_fees = 0;

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>()) {
        // Update the config in case <b>if</b> transition flow is STARTED -&gt; SUSPENDED-&gt; STARTED.
        // <b>to</b> reflect new configs for the new cycle <b>if</b> it <b>has</b> been updated during SUSPENDED state processing
        <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info);
        <a href="automation_registry.md#0x1_automation_registry_move_to_started_state">move_to_started_state</a>(cycle_info)
    } <b>else</b> {
        <a href="automation_registry.md#0x1_automation_registry_move_to_ready_state">move_to_ready_state</a>(cycle_info)
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_cycle_transition_state_from_finished"></a>

## Function `update_cycle_transition_state_from_finished`

Updates the cycle state if the transition is identified to be finalized.

From CYCLE_FINALIZED state we always move to the next cycle and in CYCLE_STARTED state.

But if it happened so that there was a suspension during cycle transition which was ignored,
then immediately cycle state is updated to suspended.

Expectation will be that native layer catches this double transition and issues refunds for the new cycle fees
which will not proceeded farther in any case.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_cycle_transition_state_from_finished">update_cycle_transition_state_from_finished</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_cycle_transition_state_from_finished">update_cycle_transition_state_from_finished</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>,
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>assert</b>!(std::option::is_some(&cycle_info.transition_state), <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);

    <b>let</b> transition_state = std::option::borrow(&cycle_info.transition_state);
    <b>let</b> transition_finalized = <a href="automation_registry.md#0x1_automation_registry_is_transition_finalized">is_transition_finalized</a>(transition_state);

    <b>if</b> (!transition_finalized) {
        <b>return</b>
    };

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_next_cycle = transition_state.sys_gas_committed_for_next_cycle;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.system_tasks_state.gas_committed_for_this_cycle = transition_state.sys_gas_committed_for_next_cycle;

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_next_epoch = transition_state.gas_committed_for_next_cycle;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.gas_committed_for_this_epoch = (transition_state.gas_committed_for_new_cycle <b>as</b> u256);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_active_task_ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_locked_fees = transition_state.locked_fees;

    // Set current <a href="timestamp.md#0x1_timestamp">timestamp</a> <b>as</b> cycle start_time
    // Increase cycle and <b>update</b> the state <b>to</b> Started
    <a href="automation_registry.md#0x1_automation_registry_move_to_started_state">move_to_started_state</a>(cycle_info);
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_active_task_ids)) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_ActiveTasks">ActiveTasks</a> {
            task_indexes: <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_active_task_ids
        });
    };
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>()) {
        <a href="automation_registry.md#0x1_automation_registry_try_move_to_suspended_state">try_move_to_suspended_state</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main, cycle_info)
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal"></a>

## Function `estimate_automation_fee_with_committed_occupancy_internal`

Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
referencing the current automation registry fee parameters, specified total/committed occupancy and registry
maximum allowed occupancy for the next epoch.
Note it is expected that committed_occupancy does not include currnet task's occupancy.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(task_occupancy: u64, committed_occupancy: u64, duration: u64, active_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">automation_registry::ActiveAutomationRegistryConfigV2</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(
    task_occupancy: u64,
    committed_occupancy: u64,
    duration: u64,
    active_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>
): u64 {
    <b>let</b> total_committed_max_gas = committed_occupancy + task_occupancy;

    // Compute the automation fee multiplier for epoch
    <b>let</b> automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(
        &active_config.main_config,
        (total_committed_max_gas <b>as</b> u256),
        active_config.next_cycle_registry_max_gas_cap);

    <b>if</b> (automation_fee_per_sec == 0) {
        <b>return</b> 0
    };

    <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_for_interval">calculate_automation_fee_for_interval</a>(
        duration,
        task_occupancy,
        automation_fee_per_sec,
        active_config.next_cycle_registry_max_gas_cap)
}
</code></pre>



</details>

<a id="0x1_automation_registry_validate_configuration_parameters_common"></a>

## Function `validate_configuration_parameters_common`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(cycle_duration_secs: u64, task_duration_cap_in_secs: u64, sys_task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, sys_registry_max_gas_cap: u64, congestion_threshold_percentage: u8, congestion_exponent: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(
    cycle_duration_secs: u64,
    task_duration_cap_in_secs: u64,
    sys_task_duration_cap_in_secs: u64,
    registry_max_gas_cap: u64,
    sys_registry_max_gas_cap: u64,
    congestion_threshold_percentage: u8,
    congestion_exponent: u8,
) {
    <b>assert</b>!(cycle_duration_secs &gt; 0, <a href="automation_registry.md#0x1_automation_registry_ECYCLE_DURATION_NON_ZERO">ECYCLE_DURATION_NON_ZERO</a>);
    <b>assert</b>!(congestion_threshold_percentage &lt;= <a href="automation_registry.md#0x1_automation_registry_MAX_PERCENTAGE">MAX_PERCENTAGE</a>, <a href="automation_registry.md#0x1_automation_registry_EMAX_CONGESTION_THRESHOLD">EMAX_CONGESTION_THRESHOLD</a>);
    <b>assert</b>!(congestion_exponent &gt; 0, <a href="automation_registry.md#0x1_automation_registry_ECONGESTION_EXP_NON_ZERO">ECONGESTION_EXP_NON_ZERO</a>);
    <b>assert</b>!(task_duration_cap_in_secs &gt; cycle_duration_secs, <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_TASK_DURATION_CAP">EUNACCEPTABLE_TASK_DURATION_CAP</a>);
    <b>assert</b>!(sys_task_duration_cap_in_secs &gt; cycle_duration_secs, <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_SYS_TASK_DURATION_CAP">EUNACCEPTABLE_SYS_TASK_DURATION_CAP</a>);
    <b>assert</b>!(registry_max_gas_cap &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO">EREGISTRY_MAX_GAS_CAP_NON_ZERO</a>);
    <b>assert</b>!(sys_registry_max_gas_cap &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO">EREGISTRY_SYSTEM_MAX_GAS_CAP_NON_ZERO</a>);
}
</code></pre>



</details>

<a id="0x1_automation_registry_validate_system_configuration_parameters_common"></a>

## Function `validate_system_configuration_parameters_common`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_system_configuration_parameters_common">validate_system_configuration_parameters_common</a>(cycle_duration_secs: u64, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_system_configuration_parameters_common">validate_system_configuration_parameters_common</a>(
    cycle_duration_secs: u64,
    task_duration_cap_in_secs: u64,
    registry_max_gas_cap: u64,
) {
    <b>assert</b>!(task_duration_cap_in_secs &gt; cycle_duration_secs, <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_SYS_TASK_DURATION_CAP">EUNACCEPTABLE_SYS_TASK_DURATION_CAP</a>);
    <b>assert</b>!(registry_max_gas_cap &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS">EREGISTRY_MAX_GAS_CAP_NON_ZERO_SYS</a>);
}
</code></pre>



</details>

<a id="0x1_automation_registry_create_registry_resource_account"></a>

## Function `create_registry_resource_account`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_create_registry_resource_account">create_registry_resource_account</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_create_registry_resource_account">create_registry_resource_account</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability) {
    <b>let</b> (registry_fee_resource_signer, registry_fee_address_signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(
        supra_framework,
        <a href="automation_registry.md#0x1_automation_registry_REGISTRY_RESOURCE_SEED">REGISTRY_RESOURCE_SEED</a>
    );
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;SupraCoin&gt;(&registry_fee_resource_signer);
    (registry_fee_resource_signer, registry_fee_address_signer_cap)
}
</code></pre>



</details>

<a id="0x1_automation_registry_on_cycle_end_internal"></a>

## Function `on_cycle_end_internal`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_cycle_end_internal">on_cycle_end_internal</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_cycle_end_internal">on_cycle_end_internal</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = &<b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework).main;
    <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_length">enumerable_map::length</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks) == 0) {
        // Registry is empty <b>update</b> config-buffer and <b>move</b> <b>to</b> started state directly
        <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info);
        <a href="automation_registry.md#0x1_automation_registry_move_to_started_state">move_to_started_state</a>(cycle_info);
        <b>return</b>
    };
    <b>let</b> expected_tasks_to_be_processed = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);
    expected_tasks_to_be_processed = sort_vector_u64(expected_tasks_to_be_processed);
    <b>let</b> transition_state = <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a> {
        refund_duration: 0,
        new_cycle_duration: cycle_info.duration_secs,
        automation_fee_per_sec: 0,
        gas_committed_for_new_cycle: <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch,
        gas_committed_for_next_cycle: 0,
        sys_gas_committed_for_next_cycle: 0,
        locked_fees: 0,
        expected_tasks_to_be_processed,
        next_task_index_position: 0
    };
    cycle_info.transition_state = std::option::some(transition_state);
    // During cycle transition we <b>update</b> config only after transition state is created in order <b>to</b> have new cycle
    // duration <b>as</b> transition state parameter.
    <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info);
    // Calculate automation fee per second for the new epoch only after configuration is updated.
    <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);
    // As we already know the committed gas for the new cycle it is being calculated using updated fee-parameters
    // and will be used <b>to</b> charge tasks during transition process.
    transition_state.automation_fee_per_sec =
        <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_committed_occupancy">calculate_automation_fee_multiplier_for_committed_occupancy</a>(transition_state.gas_committed_for_new_cycle);
    <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info, <a href="automation_registry.md#0x1_automation_registry_CYCLE_FINISHED">CYCLE_FINISHED</a>);
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_cycle_state_to"></a>

## Function `update_cycle_state_to`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>, state: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>, state: u8) {
    <b>let</b> old_state = cycle_info.state;
    cycle_info.state = state;
    <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="automation_registry.md#0x1_automation_registry_AutomationCycleEvent">AutomationCycleEvent</a> {
        cycle_state_info: <a href="automation_registry.md#0x1_automation_registry_into_automation_cycle_info">into_automation_cycle_info</a>(cycle_info),
        old_state,
    };
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="event.md#0x1_event">event</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_move_to_ready_state"></a>

## Function `move_to_ready_state`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_move_to_ready_state">move_to_ready_state</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_move_to_ready_state">move_to_ready_state</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>) {
    // If the cycle duration updated <b>has</b> been identified during transtion, then the transition state is kept
    // <b>with</b> reset values <b>except</b> new cycle duration <b>to</b> have it properly set for the next new cycle.
    // This may happen in case of cycle was ended and feature-flag <b>has</b> been disbaled before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> task <b>has</b>
    // been processed for the cycle transition.
    // Note that we want <b>to</b> have consistent data in ready state which says that the cycle pointed in the ready state
    // <b>has</b> been finished/summerized, and we are ready <b>to</b> start the next new cycle. and all the cycle inforamation should
    // match the finalized/summerized cycle since its start, including cycle duration
    <b>if</b> (std::option::is_some(&cycle_info.transition_state)) {
        <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);
        <b>if</b> (transition_state.new_cycle_duration == cycle_info.duration_secs) {
            cycle_info.transition_state = std::option::none&lt;<a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a>&gt;();
        } <b>else</b> {
            // Reset all <b>except</b> new cycle duration
            transition_state.refund_duration = 0;
            transition_state.automation_fee_per_sec = 0;
            transition_state.gas_committed_for_new_cycle = 0;
            transition_state.gas_committed_for_next_cycle = 0;
            transition_state.sys_gas_committed_for_next_cycle = 0;
            transition_state.locked_fees = 0;
            transition_state.expected_tasks_to_be_processed = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
            transition_state.next_task_index_position = 0;
        }
    };
    <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info, <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_move_to_started_state"></a>

## Function `move_to_started_state`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_move_to_started_state">move_to_started_state</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_move_to_started_state">move_to_started_state</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>) {
    cycle_info.index = cycle_info.index + 1;
    cycle_info.start_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>if</b> (std::option::is_some(&cycle_info.transition_state)) {
        <b>let</b> transition_state = std::option::extract(&<b>mut</b> cycle_info.transition_state);
        cycle_info.duration_secs = transition_state.new_cycle_duration;
    };
    <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info, <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_try_move_to_suspended_state"></a>

## Function `try_move_to_suspended_state`

Transition to suspended state is expected to be called
a) when cycle is active and in progress
- here we simply move to suspended state so native layer can start requesting tasks processing
which will end up in  refunds and cleanup. Note that refund will be done based on total gas-committed
for the current cycle defined at the begining for the cycle, and using current automation fee parameters
b) when cycle has just finished and there was another transaction causing feature suspension
- as this both events happen in scope of the same block, then we will simply update the state to suspended
and the native layer should identify the transition and request processing of the all available tasks.
Note that in this case automation fee refund will not be expected and suspention and cycle end matched and
no fee was yet charged to be refunded.
So the duration for refund and automation-fee-per-second for refund will be 0
c) when cycle transition was in progress and there was a feature suspension, but it could not be applied,
and postponed till the cycle transition concludes
In all cases if there are no tasks in registry the state will be updated directly to CYCLE_READY state.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_move_to_suspended_state">try_move_to_suspended_state</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_move_to_suspended_state">try_move_to_suspended_state</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>
) <b>acquires</b>  <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_length">enumerable_map::length</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks) == 0) {
        // Registry is empty <b>move</b> <b>to</b> ready state directly
        // <a href="automation_registry.md#0x1_automation_registry_move_to_ready_state">move_to_ready_state</a>(cycle_info);
        <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info, <a href="automation_registry.md#0x1_automation_registry_CYCLE_READY">CYCLE_READY</a>);
        <b>return</b>
    };
    <b>if</b> (std::option::is_none(&cycle_info.transition_state)) {
        // Indicates that cycle was in STARTED state when suspention <b>has</b> been identified.
        // It is safe <b>to</b> <b>assert</b> that cycle_end_time will always be greater than current chain time <b>as</b>
        // the cycle end is check in the <a href="block.md#0x1_block">block</a> metadata txn execution which proceeds <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> other transaction in the <a href="block.md#0x1_block">block</a>.
        // Including the transaction which caused transition <b>to</b> suspended state.
        // So in case <b>if</b> cycle_end_time &lt; current_time then cycle end would have been identified
        // and we would have enterend <b>else</b> branch instead.
        // This holds <b>true</b> even <b>if</b> we identified suspention when moving from FINALIZED-&gt;STARTED state.
        // As in this case we will first transition <b>to</b> the STARTED state and only then <b>to</b> SUSPENDED.
        // And when transition <b>to</b> STARTED state we <b>update</b> the cycle start-time <b>to</b> be the current-chain-time.
        <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
        <b>let</b> cycle_end_time = cycle_info.start_time + cycle_info.duration_secs;
        <b>assert</b>!(current_time &gt;= cycle_info.start_time, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
        <b>assert</b>!(current_time &lt; cycle_end_time, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
        <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_STARTED">CYCLE_STARTED</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
        <b>let</b> active_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(@supra_framework);
        <b>let</b> expected_tasks_to_be_processed = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);
        expected_tasks_to_be_processed = sort_vector_u64(expected_tasks_to_be_processed);
        <b>let</b> transition_state = <a href="automation_registry.md#0x1_automation_registry_TransitionState">TransitionState</a> {
            refund_duration: cycle_end_time - current_time,
            new_cycle_duration: cycle_info.duration_secs,
            automation_fee_per_sec: <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle_internal">calculate_automation_fee_multiplier_for_current_cycle_internal</a>(active_config, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>),
            gas_committed_for_new_cycle: 0,
            gas_committed_for_next_cycle: 0,
            sys_gas_committed_for_next_cycle: 0,
            locked_fees: 0,
            expected_tasks_to_be_processed,
            next_task_index_position: 0
        };
        cycle_info.transition_state = std::option::some(transition_state);
    } <b>else</b> {
        <b>assert</b>!(cycle_info.state == <a href="automation_registry.md#0x1_automation_registry_CYCLE_FINISHED">CYCLE_FINISHED</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
        <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);
        <b>assert</b>!(!<a href="automation_registry.md#0x1_automation_registry_is_transition_in_progress">is_transition_in_progress</a>(transition_state), <a href="automation_registry.md#0x1_automation_registry_EINVALID_REGISTRY_STATE">EINVALID_REGISTRY_STATE</a>);
        // Did not manage <b>to</b> charge cycle fee, so automation_fee_per_sec be 0 along <b>with</b> remaining duration
        // So the tasks sent for refund, will get only deposit refunded.
        transition_state.refund_duration = 0;
        transition_state.automation_fee_per_sec = 0;
        transition_state.gas_committed_for_new_cycle = 0;
    };
    <a href="automation_registry.md#0x1_automation_registry_update_cycle_state_to">update_cycle_state_to</a>(cycle_info, <a href="automation_registry.md#0x1_automation_registry_CYCLE_SUSPENDED">CYCLE_SUSPENDED</a>)
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_state_for_migration"></a>

## Function `update_state_for_migration`

Refunds automation fee for epoch for all eligible tasks and clears automation registry state in terms of
fee primitives.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_state_for_migration">update_state_for_migration</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, aei: <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>, current_time: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_state_for_migration">update_state_for_migration</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    aei: <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>,
    current_time: u64
) {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
        start_time,
        epoch_interval: _,
        expected_epoch_duration,

    } = aei;
    <b>let</b> previous_epoch_duration = current_time - start_time;
    <b>let</b> refund_interval = 0;
    <b>let</b> refund_automation_fee_per_sec = 0;

    // If epoch actual duration is greater or equal <b>to</b> expected epoch-duration then there is nothing <b>to</b> refund.
    <b>if</b> (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees != 0 && previous_epoch_duration &lt; expected_epoch_duration) {
        <b>let</b> previous_tcmg = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch;
        refund_interval = expected_epoch_duration - previous_epoch_duration;
        // Compute the automation fee multiplier for ended epoch
        refund_automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(arc, previous_tcmg, arc.registry_max_gas_cap);
    };
    <a href="automation_registry.md#0x1_automation_registry_refund_fees_and_update_tasks">refund_fees_and_update_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, arc, refund_automation_fee_per_sec, refund_interval, current_time);

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch = 0;
}
</code></pre>



</details>

<a id="0x1_automation_registry_refund_fees_and_update_tasks"></a>

## Function `refund_fees_and_update_tasks`

Refunds automation fee for epoch for all eligible tasks during migration.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_fees_and_update_tasks">refund_fees_and_update_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, refund_automation_fee_per_sec: u256, refund_interval: u64, current_time: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_fees_and_update_tasks">refund_fees_and_update_tasks</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    refund_automation_fee_per_sec: u256,
    refund_interval: u64,
    current_time: u64)
{
    <b>let</b> ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
    );
    <b>let</b> epoch_locked_fees = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees;

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(ids, |task_index| {
        <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
        // Defult type before migration is <a href="automation_registry.md#0x1_automation_registry_UST">UST</a> and the priority is the task index
        task.aux_data = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="automation_registry.md#0x1_automation_registry_UST">UST</a>], <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&task_index)];
        <b>if</b> (refund_automation_fee_per_sec != 0 && task.state != <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
            <b>let</b> refund = <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(
                arc,
                task,
                refund_interval,
                current_time,
                refund_automation_fee_per_sec);
            <b>let</b> (_, remaining_epoch_locked_fees) = <a href="automation_registry.md#0x1_automation_registry_safe_fee_refund">safe_fee_refund</a>(
                epoch_locked_fees,
                &resource_signer,
                <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
                task.task_index,
                task.owner,
                refund);
            epoch_locked_fees = remaining_epoch_locked_fees;
        };
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_safe_deposit_refund"></a>

## Function `safe_deposit_refund`

Refunds specified amount of deposit to the task owner and unlocks full deposit from registry resource account.
Error events are emitted
- if the registry resource account does not have enough balance for refund.
- if the full deposit can not be unlocked.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(rb: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_address: <b>address</b>, task_index: u64, task_owner: <b>address</b>, refundable_deposit: u64, locked_deposit: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
    rb: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    resource_address: <b>address</b>,
    task_index: u64,
    task_owner: <b>address</b>,
    refundable_deposit: u64,
    locked_deposit: u64
):  bool {
    // This check will make sure that no more than totally locked deposited will be refunded.
    // If there is an attempt then it means implementation bug.
    <b>let</b> result = <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_deposit">safe_unlock_locked_deposit</a>(rb, locked_deposit, task_index);
    <b>if</b> (!result) {
        <b>return</b> result
    };

    <b>let</b> result = <a href="automation_registry.md#0x1_automation_registry_safe_refund">safe_refund</a>(
        resource_signer,
        resource_address,
        task_index,
        task_owner,
        refundable_deposit,
        <a href="automation_registry.md#0x1_automation_registry_DEPOSIT_EPOCH_FEE">DEPOSIT_EPOCH_FEE</a>);

    <b>if</b> (result) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="automation_registry.md#0x1_automation_registry_TaskDepositFeeRefund">TaskDepositFeeRefund</a> { task_index, owner: task_owner, amount: refundable_deposit }
        );
    };
    result
}
</code></pre>



</details>

<a id="0x1_automation_registry_safe_unlock_locked_deposit"></a>

## Function `safe_unlock_locked_deposit`

Unlocks the deposit paid by the task from internal deposit refund bookkeeping state.
Error event is emitted if the deposit refund bookkeeping state is inconsistent with the requested unlock amount.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_deposit">safe_unlock_locked_deposit</a>(rb: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, locked_deposit: u64, task_index: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_deposit">safe_unlock_locked_deposit</a>(
    rb: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    locked_deposit: u64,
    task_index: u64
): bool {
    <b>let</b> has_locked_deposit = rb.total_deposited_automation_fee &gt;= locked_deposit;
    <b>if</b> (has_locked_deposit) {
        rb.total_deposited_automation_fee = rb.total_deposited_automation_fee - locked_deposit;
    } <b>else</b> {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="automation_registry.md#0x1_automation_registry_ErrorUnlockTaskDepositFee">ErrorUnlockTaskDepositFee</a> { total_registered_deposit: rb.total_deposited_automation_fee, locked_deposit, task_index }
        );
    };
    has_locked_deposit
}
</code></pre>



</details>

<a id="0x1_automation_registry_safe_unlock_locked_epoch_fee"></a>

## Function `safe_unlock_locked_epoch_fee`

Unlocks the locked fee paid by the task for epoch.
Error event is emitted if the epoch locked fee amount is inconsistent with the requested unlock amount.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_epoch_fee">safe_unlock_locked_epoch_fee</a>(epoch_locked_fees: u64, refundable_fee: u64, task_index: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_epoch_fee">safe_unlock_locked_epoch_fee</a>(
    epoch_locked_fees: u64,
    refundable_fee: u64,
    task_index: u64
): (bool, u64) {
    // This check makes sure that more than locked amount of the fees will be not be refunded.
    // Any attempt means <b>internal</b> bug.
    <b>let</b> has_locked_fee = epoch_locked_fees &gt;= refundable_fee;
    <b>if</b> (has_locked_fee) {
        // unlock the refunded amount
        epoch_locked_fees = epoch_locked_fees - refundable_fee;
    } <b>else</b> {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="automation_registry.md#0x1_automation_registry_ErrorUnlockTaskEpochFee">ErrorUnlockTaskEpochFee</a> { locked_epoch_fees: epoch_locked_fees, task_index, refund: refundable_fee}
        );
    };
    (has_locked_fee, epoch_locked_fees)
}
</code></pre>



</details>

<a id="0x1_automation_registry_safe_fee_refund"></a>

## Function `safe_fee_refund`

Refunds fee paid by the task for the epoch to the task owner.
Note that here we do not unlock the fee, as on epoch change locked epoch-fees for the ended epoch are
automatically unlocked.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_fee_refund">safe_fee_refund</a>(epoch_locked_fees: u64, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_address: <b>address</b>, task_index: u64, task_owner: <b>address</b>, refundable_fee: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_fee_refund">safe_fee_refund</a>(
    epoch_locked_fees: u64,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    resource_address: <b>address</b>,
    task_index: u64,
    task_owner: <b>address</b>,
    refundable_fee: u64
):  (bool, u64) {
    <b>let</b> (result, remaining_locked_fees) = <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_epoch_fee">safe_unlock_locked_epoch_fee</a>(epoch_locked_fees, refundable_fee, task_index);
    <b>if</b> (!result) {
        <b>return</b> (result, remaining_locked_fees)
    };
    <b>let</b> result = <a href="automation_registry.md#0x1_automation_registry_safe_refund">safe_refund</a>(
        resource_signer,
        resource_address,
        task_index,
        task_owner,
        refundable_fee,
        <a href="automation_registry.md#0x1_automation_registry_EPOCH_FEE">EPOCH_FEE</a>);
    <b>if</b> (result) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="automation_registry.md#0x1_automation_registry_TaskFeeRefund">TaskFeeRefund</a> { task_index, owner: task_owner, amount: refundable_fee }
        );
    };
    (result, remaining_locked_fees)
}
</code></pre>



</details>

<a id="0x1_automation_registry_safe_refund"></a>

## Function `safe_refund`

Refunds specified amount to the task owner.
Error event is emitted if the resource account does not have enough balance.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_refund">safe_refund</a>(resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_address: <b>address</b>, task_index: u64, task_owner: <b>address</b>, refundable_amount: u64, refund_type: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_refund">safe_refund</a>(
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    resource_address: <b>address</b>,
    task_index: u64,
    task_owner: <b>address</b>,
    refundable_amount: u64,
    refund_type: u8
):  bool {
    <b>let</b> balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(resource_address);
    <b>if</b> (balance &lt; refundable_amount) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="automation_registry.md#0x1_automation_registry_ErrorInsufficientBalanceToRefund">ErrorInsufficientBalanceToRefund</a> { refund_type, task_index, owner: task_owner, amount: refundable_amount }
        );
        <b>return</b> <b>false</b>
    };

    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(resource_signer, task_owner, refundable_amount);
    <b>return</b> <b>true</b>
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_task_fee"></a>

## Function `calculate_task_fee`

Calculates automation task fees for a single task at the time of new epoch.
This is supposed to be called only after removing expired task and must not be called for expired task.
It returns calculated task fee for the interval the task will be active.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>, potential_fee_timeframe: u64, current_time: u64, automation_fee_per_sec: u256): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>,
    potential_fee_timeframe: u64,
    current_time: u64,
    automation_fee_per_sec: u256
): u64 {
    <b>if</b> (automation_fee_per_sec == 0) { <b>return</b> 0 };
    <b>if</b> (task.expiry_time &lt;= current_time) { <b>return</b> 0 };
    // Subtraction is safe here, <b>as</b> we already excluded expired tasks
    <b>let</b> task_active_timeframe = task.expiry_time - current_time;
    // If the task is a new task i.e. in Pending state, then it is charged always for
    // the input potential_fee_timeframe(which is epoch-interval),
    // For the new tasks which active-timeframe is less than epoch-interval
    // it would mean it is their first and only epoch and we charge the fee for entire epoch.
    // Note that although the new short tasks are charged for entire epoch, the refunding logic remains the same for
    // them <b>as</b> for the long tasks.
    // This way bad-actors will be discourged <b>to</b> submit small and short tasks <b>with</b> big occupancy by blocking other
    // good-actors register tasks.
    <b>let</b> actual_fee_timeframe = <b>if</b> (task.state == <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
        potential_fee_timeframe
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_min">math64::min</a>(task_active_timeframe, potential_fee_timeframe)
    };
    <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_for_interval">calculate_automation_fee_for_interval</a>(
        actual_fee_timeframe,
        task.max_gas_amount,
        automation_fee_per_sec,
        arc.registry_max_gas_cap)
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_automation_fee_for_interval"></a>

## Function `calculate_automation_fee_for_interval`

Calculates automation task fees for a single task at the time of new epoch.
This is supposed to be called only after removing expired task and must not be called for expired task.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_for_interval">calculate_automation_fee_for_interval</a>(interval: u64, task_occupancy: u64, automation_fee_per_sec: u256, registry_max_gas_cap: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_for_interval">calculate_automation_fee_for_interval</a>(
    interval: u64,
    task_occupancy: u64,
    automation_fee_per_sec: u256,
    registry_max_gas_cap: u64,
): u64 {
    <b>let</b> max_gas_cap = (registry_max_gas_cap <b>as</b> u256);
    <b>let</b> duration = (interval <b>as</b> u256);
    <b>let</b> task_occupancy_ratio_by_duration = (duration * <a href="automation_registry.md#0x1_automation_registry_upscale_from_u64">upscale_from_u64</a>(task_occupancy)) / max_gas_cap;

    <b>let</b> automation_fee_for_interval = automation_fee_per_sec * task_occupancy_ratio_by_duration;

    <a href="automation_registry.md#0x1_automation_registry_downscale_to_u64">downscale_to_u64</a>(automation_fee_for_interval)
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle_internal"></a>

## Function `calculate_automation_fee_multiplier_for_current_cycle_internal`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle_internal">calculate_automation_fee_multiplier_for_current_cycle_internal</a>(active_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">automation_registry::ActiveAutomationRegistryConfigV2</a>, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_current_cycle_internal">calculate_automation_fee_multiplier_for_current_cycle_internal</a>(
    active_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>,
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>
): u64 {
    // Compute the automation fee multiplier for this cycle
    <b>let</b> multiplier = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(
        &active_config.main_config,
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch,
        active_config.main_config.registry_max_gas_cap);
    (multiplier <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch"></a>

## Function `calculate_automation_fee_multiplier_for_epoch`

Calculate automation fee multiplier for epoch. It is measured in quants/sec.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, tcmg: u256, registry_max_gas_cap: u64): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    tcmg: u256,
    registry_max_gas_cap: u64
): u256 {
    <b>let</b> acf = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_congestion_fee">calculate_automation_congestion_fee</a>(arc, tcmg, registry_max_gas_cap);
    acf + (arc.automation_base_fee_in_quants_per_sec <b>as</b> u256)
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_automation_congestion_fee"></a>

## Function `calculate_automation_congestion_fee`

Calculate automation congestion fee for the epoch


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_congestion_fee">calculate_automation_congestion_fee</a>(arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, tcmg: u256, registry_max_gas_cap: u64): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_automation_congestion_fee">calculate_automation_congestion_fee</a>(
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    tcmg: u256,
    registry_max_gas_cap: u64
): u256 {
    <b>if</b> (arc.congestion_threshold_percentage == <a href="automation_registry.md#0x1_automation_registry_MAX_PERCENTAGE">MAX_PERCENTAGE</a> || arc.congestion_base_fee_in_quants_per_sec == 0) {
        <b>return</b> 0
    };

    <b>let</b> max_gas_cap = (registry_max_gas_cap <b>as</b> u256);
    <b>let</b> threshold_percentage = <a href="automation_registry.md#0x1_automation_registry_upscale_from_u8">upscale_from_u8</a>(arc.congestion_threshold_percentage);

    // Calculate congestion threshold surplus for the current epoch
    <b>let</b> threshold_usage = <a href="automation_registry.md#0x1_automation_registry_upscale_from_u256">upscale_from_u256</a>(tcmg) * 100 / max_gas_cap;
    <b>if</b> (threshold_usage &lt;= threshold_percentage) 0
    <b>else</b> {
        <b>let</b> threshold_surplus_normalized = (threshold_usage - threshold_percentage) / 100;

        // Ensure threshold + threshold_surplus does not exceeds 1 (1 in scaled terms)
        <b>let</b> threshold_percentage_scaled = threshold_percentage / 100;
        <b>let</b> threshold_surplus_clip = <b>if</b> ((threshold_surplus_normalized + threshold_percentage_scaled) &gt; <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>) {
            <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> - threshold_percentage_scaled
        } <b>else</b> {
            threshold_surplus_normalized
        };
        // Compute the automation congestion fee (acf) for the epoch
        <b>let</b> threshold_surplus_exponential = <a href="automation_registry.md#0x1_automation_registry_calculate_exponentiation">calculate_exponentiation</a>(
            threshold_surplus_clip,
            arc.congestion_exponent
        );

        // Calculate acf by multiplying base fee <b>with</b> exponential result
        <b>let</b> acf = (arc.congestion_base_fee_in_quants_per_sec <b>as</b> u256) * threshold_surplus_exponential;
        <a href="automation_registry.md#0x1_automation_registry_downscale_to_u256">downscale_to_u256</a>(acf)
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_exponentiation"></a>

## Function `calculate_exponentiation`

Calculates (1 + base)^exponent, where <code>base</code> is represented with <code><a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a></code> decimal places.
For example, if <code>base</code> is 0.5, it should be passed as 0.5 * DECIMAL (i.e., 50000000).
The result is returned as an integer with <code><a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a></code> decimal places.
It will return the result of (((1 + base)^exponent) - 1), scaled by <code><a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a></code> (e.g., 103906250 for 1.0390625).
The reason for using <code>(1 + base)^exponent</code> is that <code>base</code> would be the fraction by which the congestion threshold is crossed,
thus highly likely to be less than one. To ensure that as <code>exponent</code> increases, the function increases, <code>1</code> is added.
In the final result, after <code>(1 + base)^exponent</code> is calculated, <code>1</code> is subtracted so as not to subsume the automation
base fee in this component. This would allow the freedom to set a multiplier for the automation base fee separately
from the congestion fee.
<code>exponent</code> here acts as the degree of the polynomial, therefore an <code>exponent</code> of <code>2</code> or higher
would allow the congestion fee to increase in a non-linear fashion.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_exponentiation">calculate_exponentiation</a>(base: u256, exponent: u8): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_exponentiation">calculate_exponentiation</a>(base: u256, exponent: u8): u256 {
    // Add 1 (represented <b>as</b> <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>) <b>to</b> the base
    <b>let</b> one_scaled = <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>; // 1.0 in <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> representation
    <b>let</b> adjusted_base = base + one_scaled; // (1 + base) in <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> representation

    // Initialize result <b>as</b> 1 (represented in <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>)
    <b>let</b> result = one_scaled;

    // Perform exponential calculation using integer arithmetic
    <b>let</b> i = 0;
    <b>while</b> (i &lt; exponent) {
        result = result * adjusted_base / <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>; // Adjust for decimal places
        i = i + 1;
    };

    // Subtract the initial added 1 (<a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>) <b>to</b> get the final result
    result - one_scaled
}
</code></pre>



</details>

<a id="0x1_automation_registry_try_withdraw_task_automation_fee"></a>

## Function `try_withdraw_task_automation_fee`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">automation_registry::AutomationRegistryV2</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">automation_registry::AutomationTaskFeeMeta</a>, current_cycle_end_time: u64, intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">automation_registry::IntermediateStateOfCycleChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">AutomationTaskFeeMeta</a>,
    current_cycle_end_time: u64,
    intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfCycleChange">IntermediateStateOfCycleChange</a>) {
    // Remove the automation task <b>if</b> the epoch fee cap is exceeded
    // It might happen that task <b>has</b> been expired by the time charging is being done.
    // This may be caused by the fact that bookkeeping transactions <b>has</b> been withheld due <b>to</b> epoch transition.
    <b>if</b> (task.fee &gt; task.automation_fee_cap) {
        <b>let</b> task_meta = <a href="automation_registry.md#0x1_automation_registry_refund_deposit_and_drop">refund_deposit_and_drop</a>(
            task.task_index,
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
            refund_bookkeeping,
            resource_signer,
            &<b>mut</b> intermediate_state.removed_tasks
        );
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledCapacitySurpassedV2">TaskCancelledCapacitySurpassedV2</a> {
            task_index: task.task_index,
            owner: task.owner,
            fee: task.fee,
            automation_fee_cap: task.automation_fee_cap,
            registration_hash: task_meta.tx_hash,
        });
        <b>return</b>
    };
    <b>let</b> user_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(task.owner);
    <b>if</b> (user_balance &lt; task.fee) {
        // If the user does not have enough balance, remove the task, DON'T refund the locked deposit, but simply unlock it
        // and emit an <a href="event.md#0x1_event">event</a>
        <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_deposit">safe_unlock_locked_deposit</a>(refund_bookkeeping, task.locked_deposit_fee, task.task_index);
        <b>let</b> task_meta = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.tasks, task.task_index);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> intermediate_state.removed_tasks, task.task_index);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledInsufficentBalanceV2">TaskCancelledInsufficentBalanceV2</a> {
            task_index: task.task_index,
            owner: task.owner,
            fee: task.fee,
            balance: user_balance,
            registration_hash: task_meta.tx_hash
        });
        <b>return</b>
    };
    <b>if</b> (task.fee != 0) {
        // Charge the fee and emit a success <a href="event.md#0x1_event">event</a>
        <b>let</b> withdrawn_coins = <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;SupraCoin&gt;(
            &<a href="create_signer.md#0x1_create_signer">create_signer</a>(task.owner),
            task.fee
        );
        // Merge <b>to</b> total task fees deducted from the users <a href="account.md#0x1_account">account</a>
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> intermediate_state.epoch_locked_fees, withdrawn_coins);
    };
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskEpochFeeWithdraw">TaskEpochFeeWithdraw</a> {
        task_index: task.task_index,
        owner: task.owner,
        fee: task.fee,
    });

    // Calculate gas commitment for the next epoch only for valid active tasks
    <b>if</b> (task.expiry_time &gt; current_cycle_end_time) {
        intermediate_state.gas_committed_for_next_cycle = intermediate_state.gas_committed_for_next_cycle+ task.max_gas_amount;
    };
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_config_from_buffer_for_migration"></a>

## Function `update_config_from_buffer_for_migration`

The function updates the ActiveAutomationRegistryConfig structure with values extracted from the buffer, if the buffer exists.
This function will be called only during migration and can be removed in subsequent releases
Note this function should be called in scope of migrate_v2 after automation configuration has been migrated V2 as well


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer_for_migration">update_config_from_buffer_for_migration</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer_for_migration">update_config_from_buffer_for_migration</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>&gt;()) {
        <b>let</b> buffer = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>&gt;();
        <b>let</b> automation_registry_config = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(
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
    // In case <b>if</b> between supra-framework <b>update</b> and migration step the config <b>has</b> been updated using the new v2 API.
    <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info)
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_config_from_buffer"></a>

## Function `update_config_from_buffer`

The function updates the ActiveAutomationRegistryConfig structure with values extracted from the buffer, if the buffer exists.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>(cycle_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
    <b>if</b> (!<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfigV2">AutomationRegistryConfigV2</a>&gt;()) {
        <b>return</b>
    };
    <b>let</b> buffer = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfigV2">AutomationRegistryConfigV2</a>&gt;();
    <b>let</b> active_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(
        @supra_framework
    );
    {
        <b>let</b> automation_registry_config = &<b>mut</b> active_config.main_config;
        automation_registry_config.task_duration_cap_in_secs = buffer.task_duration_cap_in_secs;
        automation_registry_config.registry_max_gas_cap = buffer.registry_max_gas_cap;
        automation_registry_config.automation_base_fee_in_quants_per_sec = buffer.automation_base_fee_in_quants_per_sec;
        automation_registry_config.flat_registration_fee_in_quants = buffer.flat_registration_fee_in_quants;
        automation_registry_config.congestion_threshold_percentage = buffer.congestion_threshold_percentage;
        automation_registry_config.congestion_base_fee_in_quants_per_sec = buffer.congestion_base_fee_in_quants_per_sec;
        automation_registry_config.congestion_exponent = buffer.congestion_exponent;
        automation_registry_config.task_capacity = buffer.task_capacity;
    };

    <b>if</b> (std::option::is_some(&cycle_info.transition_state)) {
        <b>let</b> transition_state = std::option::borrow_mut(&<b>mut</b> cycle_info.transition_state);
        transition_state.new_cycle_duration = buffer.cycle_duration_secs;
    } <b>else</b> {
        cycle_info.duration_secs = buffer.cycle_duration_secs;
    };

    {
        <b>let</b> system_task_config = &<b>mut</b> active_config.system_task_config;
        system_task_config.task_capacity = buffer.sys_task_capacity;
        system_task_config.registry_max_gas_cap = buffer.sys_registry_max_gas_cap;
        system_task_config.task_duration_cap_in_secs = buffer.sys_task_duration_cap_in_secs;

    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_transfer_fee_to_account_internal"></a>

## Function `transfer_fee_to_account_internal`

Transfers the specified fee amount from the resource account to the target account.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_transfer_fee_to_account_internal">transfer_fee_to_account_internal</a>(<b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_transfer_fee_to_account_internal">transfer_fee_to_account_internal</a>(<b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    <b>let</b> resource_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.registry_fee_address);

    <b>assert</b>!(resource_balance &gt;= amount, <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>);

    <b>assert</b>!((resource_balance - amount)
        &gt;= <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.epoch_locked_fees + refund_bookkeeping.total_deposited_automation_fee,
        <a href="automation_registry.md#0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE">EREQUEST_EXCEEDS_LOCKED_BALANCE</a>);

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.main.registry_fee_address_signer_cap
    );
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(&resource_signer, <b>to</b>, amount);
}
</code></pre>



</details>

<a id="0x1_automation_registry_validate_task_duration"></a>

## Function `validate_task_duration`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_task_duration">validate_task_duration</a>(expiry_time: u64, registration_time: u64, automation_registry_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">automation_registry::ActiveAutomationRegistryConfigV2</a>, automation_cycle_info: &<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">automation_registry::AutomationCycleDetails</a>, task_type: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_task_duration">validate_task_duration</a>(
    expiry_time: u64,
    registration_time: u64,
    automation_registry_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>,
    automation_cycle_info: &<a href="automation_registry.md#0x1_automation_registry_AutomationCycleDetails">AutomationCycleDetails</a>,
    task_type: u8
) {
    <b>assert</b>!(expiry_time &gt; registration_time, <a href="automation_registry.md#0x1_automation_registry_EINVALID_EXPIRY_TIME">EINVALID_EXPIRY_TIME</a>);
    <b>let</b> task_duration = expiry_time - registration_time;
    <b>if</b> (task_type == <a href="automation_registry.md#0x1_automation_registry_UST">UST</a>) {
        <b>assert</b>!(task_duration &lt;= automation_registry_config.main_config.task_duration_cap_in_secs, <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_TIME_UPPER">EEXPIRY_TIME_UPPER</a>);
    } <b>else</b> {
        <b>assert</b>!(task_type == <a href="automation_registry.md#0x1_automation_registry_GST">GST</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TASK_TYPE">EINVALID_TASK_TYPE</a>);
        <b>assert</b>!(task_duration &lt;= automation_registry_config.system_task_config.task_duration_cap_in_secs, <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_TIME_UPPER">EEXPIRY_TIME_UPPER</a>);
    };
    // Check that task is valid at least in the next cycle
    <b>assert</b>!(
        expiry_time &gt; (automation_cycle_info.start_time + automation_cycle_info.duration_secs),
        <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_BEFORE_NEXT_CYCLE">EEXPIRY_BEFORE_NEXT_CYCLE</a>
    );
}
</code></pre>



</details>

<a id="0x1_automation_registry_check_and_validate_aux_data"></a>

## Function `check_and_validate_aux_data`

Validates auxiliary data , by checking task type and priority if any specified.
Returns true if priority is not specify, false if specified.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_check_and_validate_aux_data">check_and_validate_aux_data</a>(aux_data: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, task_type: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_check_and_validate_aux_data">check_and_validate_aux_data</a>(aux_data: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, task_type: u8) : bool {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(aux_data) == <a href="automation_registry.md#0x1_automation_registry_SUPPORTED_AUX_DATA_COUNT_MAX">SUPPORTED_AUX_DATA_COUNT_MAX</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_AUX_DATA_LENGTH">EINVALID_AUX_DATA_LENGTH</a>);

    // Check task type
    <b>let</b> maybe_task_type = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(aux_data, <a href="automation_registry.md#0x1_automation_registry_TYPE_AUX_DATA_INDEX">TYPE_AUX_DATA_INDEX</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(maybe_task_type) == 1, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TASK_TYPE_LENGTH">EINVALID_TASK_TYPE_LENGTH</a>);
    <b>let</b> type_value = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(maybe_task_type, 0);
    <b>assert</b>!(*type_value == task_type, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TASK_TYPE">EINVALID_TASK_TYPE</a>);

    // Check priority existence
    <b>let</b> maybe_task_priority = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(aux_data, <a href="automation_registry.md#0x1_automation_registry_PRIORITY_AUX_DATA_INDEX">PRIORITY_AUX_DATA_INDEX</a>);
    <b>let</b> has_no_priority = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(maybe_task_priority);
    <b>if</b> (!has_no_priority) {
        // If there is a value specified validate that it can be converted <b>to</b> u64 successfully.
        // This will allow <b>to</b> avoid invalid task registration
        <b>let</b> _ = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(*maybe_task_priority);
    };
    has_no_priority
}
</code></pre>



</details>

<a id="0x1_automation_registry_migrate_registry_config"></a>

## Function `migrate_registry_config`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_migrate_registry_config">migrate_registry_config</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sys_task_duration_cap_in_secs: u64, sys_registry_max_gas_cap: u64, sys_task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_migrate_registry_config">migrate_registry_config</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    sys_task_duration_cap_in_secs: u64,
    sys_registry_max_gas_cap: u64,
    sys_task_capacity: u16

) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>let</b> current_active_config = <b>move_from</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
        main_config,
        next_epoch_registry_max_gas_cap,
        registration_enabled
    } = current_active_config;
    <b>let</b> system_task_config =  <a href="automation_registry.md#0x1_automation_registry_RegistryConfigForSystemTasks">RegistryConfigForSystemTasks</a> {
        task_duration_cap_in_secs: sys_task_duration_cap_in_secs,
        registry_max_gas_cap: sys_registry_max_gas_cap,
        task_capacity: sys_task_capacity,
        aux_properties: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>()
    };
    <b>let</b> new_active_config = <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a> {
        main_config,
        next_cycle_registry_max_gas_cap: next_epoch_registry_max_gas_cap,
        next_cycle_sys_registry_max_gas_cap: sys_registry_max_gas_cap,
        registration_enabled,
        system_task_config,
        aux_configs: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>(),
    };
    <b>move_to</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfigV2">ActiveAutomationRegistryConfigV2</a>&gt;(supra_framework, new_active_config);
}
</code></pre>



</details>

<a id="0x1_automation_registry_migrate_registry_state"></a>

## Function `migrate_registry_state`

Initializes registry state for system tasks


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_migrate_registry_state">migrate_registry_state</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_migrate_registry_state">migrate_registry_state</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> system_tasks_state =  <a href="automation_registry.md#0x1_automation_registry_RegistryStateForSystemTasks">RegistryStateForSystemTasks</a> {
        gas_committed_for_this_cycle: 0,
        gas_committed_for_next_cycle: 0,
        authorized_accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    };

    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryV2">AutomationRegistryV2</a> {
        main: <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        system_tasks_state,
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_upscale_from_u8"></a>

## Function `upscale_from_u8`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_upscale_from_u8">upscale_from_u8</a>(value: u8): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_upscale_from_u8">upscale_from_u8</a>(value: u8): u256 { (value <b>as</b> u256) * <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> }
</code></pre>



</details>

<a id="0x1_automation_registry_upscale_from_u64"></a>

## Function `upscale_from_u64`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_upscale_from_u64">upscale_from_u64</a>(value: u64): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_upscale_from_u64">upscale_from_u64</a>(value: u64): u256 { (value <b>as</b> u256) * <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> }
</code></pre>



</details>

<a id="0x1_automation_registry_upscale_from_u256"></a>

## Function `upscale_from_u256`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_upscale_from_u256">upscale_from_u256</a>(value: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_upscale_from_u256">upscale_from_u256</a>(value: u256): u256 { value * <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> }
</code></pre>



</details>

<a id="0x1_automation_registry_downscale_to_u64"></a>

## Function `downscale_to_u64`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_downscale_to_u64">downscale_to_u64</a>(value: u256): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_downscale_to_u64">downscale_to_u64</a>(value: u256): u64 { ((value / <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a>) <b>as</b> u64) }
</code></pre>



</details>

<a id="0x1_automation_registry_downscale_to_u256"></a>

## Function `downscale_to_u256`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_downscale_to_u256">downscale_to_u256</a>(value: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_downscale_to_u256">downscale_to_u256</a>(value: u256): u256 { value / <a href="automation_registry.md#0x1_automation_registry_DECIMAL">DECIMAL</a> }
</code></pre>



</details>

<a id="0x1_automation_registry_assert_automation_cycle_management_support"></a>

## Function `assert_automation_cycle_management_support`

If SUPRA_AUTOMATION_V2 is enabled then call native function to assert full support of cycle based
automation registry management.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_assert_automation_cycle_management_support">assert_automation_cycle_management_support</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_assert_automation_cycle_management_support">assert_automation_cycle_management_support</a>() {
    <a href="automation_registry.md#0x1_automation_registry_native_automation_cycle_management_support">native_automation_cycle_management_support</a>();
}
</code></pre>



</details>

<a id="0x1_automation_registry_native_automation_cycle_management_support"></a>

## Function `native_automation_cycle_management_support`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_native_automation_cycle_management_support">native_automation_cycle_management_support</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_native_automation_cycle_management_support">native_automation_cycle_management_support</a>(): bool;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
