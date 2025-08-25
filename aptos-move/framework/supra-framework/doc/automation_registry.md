
<a id="0x1_automation_registry"></a>

# Module `0x1::automation_registry`

Supra Automation Registry

This contract is part of the Supra Framework and is designed to manage automated task entries


-  [Resource `ActiveAutomationRegistryConfig`](#0x1_automation_registry_ActiveAutomationRegistryConfig)
-  [Resource `AutomationRegistryConfig`](#0x1_automation_registry_AutomationRegistryConfig)
-  [Resource `AutomationRegistry`](#0x1_automation_registry_AutomationRegistry)
-  [Resource `AutomationEpochInfo`](#0x1_automation_registry_AutomationEpochInfo)
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
-  [Struct `TasksStopped`](#0x1_automation_registry_TasksStopped)
-  [Struct `TaskStopped`](#0x1_automation_registry_TaskStopped)
-  [Struct `TaskCancelledInsufficentBalance`](#0x1_automation_registry_TaskCancelledInsufficentBalance)
-  [Struct `TaskCancelledCapacitySurpassed`](#0x1_automation_registry_TaskCancelledCapacitySurpassed)
-  [Struct `RemovedTasks`](#0x1_automation_registry_RemovedTasks)
-  [Struct `ActiveTasks`](#0x1_automation_registry_ActiveTasks)
-  [Struct `ErrorTaskDoesNotExist`](#0x1_automation_registry_ErrorTaskDoesNotExist)
-  [Struct `ErrorTaskDoesNotExistForWithdrawal`](#0x1_automation_registry_ErrorTaskDoesNotExistForWithdrawal)
-  [Struct `ErrorInsufficientBalanceToRefund`](#0x1_automation_registry_ErrorInsufficientBalanceToRefund)
-  [Struct `EnabledRegistrationEvent`](#0x1_automation_registry_EnabledRegistrationEvent)
-  [Struct `DisabledRegistrationEvent`](#0x1_automation_registry_DisabledRegistrationEvent)
-  [Struct `AutomationTaskFeeMeta`](#0x1_automation_registry_AutomationTaskFeeMeta)
-  [Struct `IntermediateState`](#0x1_automation_registry_IntermediateState)
-  [Struct `IntermediateStateOfEpochChange`](#0x1_automation_registry_IntermediateStateOfEpochChange)
-  [Struct `AutomationTaskFee`](#0x1_automation_registry_AutomationTaskFee)
-  [Constants](#@Constants_0)
-  [Function `is_initialized`](#0x1_automation_registry_is_initialized)
-  [Function `is_feature_enabled_and_initialized`](#0x1_automation_registry_is_feature_enabled_and_initialized)
-  [Function `get_next_task_index`](#0x1_automation_registry_get_next_task_index)
-  [Function `get_task_count`](#0x1_automation_registry_get_task_count)
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
-  [Function `get_registry_fee_address`](#0x1_automation_registry_get_registry_fee_address)
-  [Function `get_gas_committed_for_next_epoch`](#0x1_automation_registry_get_gas_committed_for_next_epoch)
-  [Function `get_gas_committed_for_current_epoch`](#0x1_automation_registry_get_gas_committed_for_current_epoch)
-  [Function `get_automation_registry_config`](#0x1_automation_registry_get_automation_registry_config)
-  [Function `get_next_epoch_registry_max_gas_cap`](#0x1_automation_registry_get_next_epoch_registry_max_gas_cap)
-  [Function `get_automation_epoch_info`](#0x1_automation_registry_get_automation_epoch_info)
-  [Function `estimate_automation_fee`](#0x1_automation_registry_estimate_automation_fee)
-  [Function `estimate_automation_fee_with_committed_occupancy`](#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy)
-  [Function `is_registration_enabled`](#0x1_automation_registry_is_registration_enabled)
-  [Function `estimate_automation_fee_with_committed_occupancy_internal`](#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal)
-  [Function `validate_configuration_parameters_common`](#0x1_automation_registry_validate_configuration_parameters_common)
-  [Function `create_registry_resource_account`](#0x1_automation_registry_create_registry_resource_account)
-  [Function `initialize`](#0x1_automation_registry_initialize)
-  [Function `initialize_refund_bookkeeping_resource`](#0x1_automation_registry_initialize_refund_bookkeeping_resource)
-  [Function `on_new_epoch`](#0x1_automation_registry_on_new_epoch)
-  [Function `finalize_epoch_change`](#0x1_automation_registry_finalize_epoch_change)
-  [Function `finalize_epoch_change_for_feature_disabled_state`](#0x1_automation_registry_finalize_epoch_change_for_feature_disabled_state)
-  [Function `update_state_for_new_epoch`](#0x1_automation_registry_update_state_for_new_epoch)
-  [Function `refund_cleanup_tasks`](#0x1_automation_registry_refund_cleanup_tasks)
-  [Function `cleanup_tasks`](#0x1_automation_registry_cleanup_tasks)
-  [Function `safe_deposit_refund_all`](#0x1_automation_registry_safe_deposit_refund_all)
-  [Function `safe_deposit_refund`](#0x1_automation_registry_safe_deposit_refund)
-  [Function `safe_unlock_locked_deposit`](#0x1_automation_registry_safe_unlock_locked_deposit)
-  [Function `safe_unlock_locked_epoch_fee`](#0x1_automation_registry_safe_unlock_locked_epoch_fee)
-  [Function `safe_fee_refund`](#0x1_automation_registry_safe_fee_refund)
-  [Function `safe_refund`](#0x1_automation_registry_safe_refund)
-  [Function `calculate_task_fee`](#0x1_automation_registry_calculate_task_fee)
-  [Function `calculate_automation_fee_for_interval`](#0x1_automation_registry_calculate_automation_fee_for_interval)
-  [Function `calculate_automation_fee_multiplier_for_epoch`](#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch)
-  [Function `calculate_automation_congestion_fee`](#0x1_automation_registry_calculate_automation_congestion_fee)
-  [Function `calculate_exponentiation`](#0x1_automation_registry_calculate_exponentiation)
-  [Function `try_withdraw_task_automation_fees`](#0x1_automation_registry_try_withdraw_task_automation_fees)
-  [Function `try_withdraw_task_automation_fee`](#0x1_automation_registry_try_withdraw_task_automation_fee)
-  [Function `update_config_from_buffer`](#0x1_automation_registry_update_config_from_buffer)
-  [Function `withdraw_automation_task_fees`](#0x1_automation_registry_withdraw_automation_task_fees)
-  [Function `transfer_fee_to_account_internal`](#0x1_automation_registry_transfer_fee_to_account_internal)
-  [Function `update_config`](#0x1_automation_registry_update_config)
-  [Function `enable_registration`](#0x1_automation_registry_enable_registration)
-  [Function `disable_registration`](#0x1_automation_registry_disable_registration)
-  [Function `register`](#0x1_automation_registry_register)
-  [Function `check_registration_task_duration`](#0x1_automation_registry_check_registration_task_duration)
-  [Function `cancel_task`](#0x1_automation_registry_cancel_task)
-  [Function `stop_tasks`](#0x1_automation_registry_stop_tasks)
-  [Function `update_epoch_interval_in_registry`](#0x1_automation_registry_update_epoch_interval_in_registry)
-  [Function `sort_vector`](#0x1_automation_registry_sort_vector)
-  [Function `upscale_from_u8`](#0x1_automation_registry_upscale_from_u8)
-  [Function `upscale_from_u64`](#0x1_automation_registry_upscale_from_u64)
-  [Function `upscale_from_u256`](#0x1_automation_registry_upscale_from_u256)
-  [Function `downscale_to_u64`](#0x1_automation_registry_downscale_to_u64)
-  [Function `downscale_to_u256`](#0x1_automation_registry_downscale_to_u256)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map">0x1::enumerable_map</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
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

<a id="0x1_automation_registry_AutomationEpochInfo"></a>

## Resource `AutomationEpochInfo`

Epoch state


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



<a id="0x1_automation_registry_EDEPOSIT_REFUND"></a>

Failed to unlock/refund deposit for a task. Internal error, for more details see emitted error events.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EDEPOSIT_REFUND">EDEPOSIT_REFUND</a>: u64 = 27;
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



<a id="0x1_automation_registry_EEXPIRY_BEFORE_NEXT_EPOCH"></a>

Expiry time must be after the start of the next epoch


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_BEFORE_NEXT_EPOCH">EEXPIRY_BEFORE_NEXT_EPOCH</a>: u64 = 3;
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



<a id="0x1_automation_registry_EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH"></a>

Automation fee capacity for the epoch should not be less than estimated one.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH">EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH</a>: u64 = 21;
</code></pre>



<a id="0x1_automation_registry_EINSUFFICIENT_BALANCE_FOR_REFUND"></a>

Resource Account does not have sufficient balance to process the refund for the specified task.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE_FOR_REFUND">EINSUFFICIENT_BALANCE_FOR_REFUND</a>: u64 = 26;
</code></pre>



<a id="0x1_automation_registry_EINVALID_EXPIRY_TIME"></a>

Invalid expiry time: it cannot be earlier than the current time


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_EXPIRY_TIME">EINVALID_EXPIRY_TIME</a>: u64 = 1;
</code></pre>



<a id="0x1_automation_registry_EINVALID_GAS_PRICE"></a>

Invalid gas price: it cannot be zero


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_GAS_PRICE">EINVALID_GAS_PRICE</a>: u64 = 4;
</code></pre>



<a id="0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT"></a>

Invalid max gas amount for automated task: it cannot be zero


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT">EINVALID_MAX_GAS_AMOUNT</a>: u64 = 5;
</code></pre>



<a id="0x1_automation_registry_EINVALID_TXN_HASH"></a>

Transaction hash that registering current task is invalid. Length should be 32.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EINVALID_TXN_HASH">EINVALID_TXN_HASH</a>: u64 = 9;
</code></pre>



<a id="0x1_automation_registry_EMAX_CONGESTION_THRESHOLD"></a>

Congestion threshold should not exceed 100.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EMAX_CONGESTION_THRESHOLD">EMAX_CONGESTION_THRESHOLD</a>: u64 = 19;
</code></pre>



<a id="0x1_automation_registry_ENO_AUX_DATA_SUPPORTED"></a>

Auxiliary data during registration is not supported


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ENO_AUX_DATA_SUPPORTED">ENO_AUX_DATA_SUPPORTED</a>: u64 = 14;
</code></pre>



<a id="0x1_automation_registry_EPOCH_FEE"></a>



<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EPOCH_FEE">EPOCH_FEE</a>: u8 = 1;
</code></pre>



<a id="0x1_automation_registry_EREGISTRY_IS_FULL"></a>

Registry task capacity has reached.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_IS_FULL">EREGISTRY_IS_FULL</a>: u64 = 23;
</code></pre>



<a id="0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO"></a>

Automation registry max gas capacity cannot be zero.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO">EREGISTRY_MAX_GAS_CAP_NON_ZERO</a>: u64 = 22;
</code></pre>



<a id="0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE"></a>

Requested amount exceeds the locked balance


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE">EREQUEST_EXCEEDS_LOCKED_BALANCE</a>: u64 = 17;
</code></pre>



<a id="0x1_automation_registry_ETASK_REGISTRATION_DISABLED"></a>

Task registration is currently disabled.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_ETASK_REGISTRATION_DISABLED">ETASK_REGISTRATION_DISABLED</a>: u64 = 24;
</code></pre>



<a id="0x1_automation_registry_EUNACCEPTABLE_AUTOMATION_GAS_LIMIT"></a>

Current committed gas amount is greater than the automation gas limit.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_AUTOMATION_GAS_LIMIT">EUNACCEPTABLE_AUTOMATION_GAS_LIMIT</a>: u64 = 10;
</code></pre>



<a id="0x1_automation_registry_EUNACCEPTABLE_TASK_DURATION_CAP"></a>

Current epoch interval is greater than specified task duration cap.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_TASK_DURATION_CAP">EUNACCEPTABLE_TASK_DURATION_CAP</a>: u64 = 18;
</code></pre>



<a id="0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER"></a>

Unauthorized access: the caller is not the owner of the task


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>: u64 = 8;
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



<a id="0x1_automation_registry_TXN_HASH_LENGTH"></a>

The length of the transaction hash.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_TXN_HASH_LENGTH">TXN_HASH_LENGTH</a>: u64 = 32;
</code></pre>



<a id="0x1_automation_registry_is_initialized"></a>

## Function `is_initialized`

Checks whether all required resources are created.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>(): bool {
    <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework)
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_next_task_index">get_next_task_index</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_count">get_task_count</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_length">enumerable_map::length</a>(&state.tasks)
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_ids">get_task_ids</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&state.tasks)
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_epoch_locked_balance">get_epoch_locked_balance</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_registry_total_locked_balance">get_registry_total_locked_balance</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_active_task_ids">get_active_task_ids</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    state.epoch_active_task_ids
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_details">get_task_details</a>(task_index: u64): <a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> automation_task_metadata = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&automation_task_metadata.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&automation_task_metadata.tasks, task_index)
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_task_details_bulk">get_task_details_bulk</a>(task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>&gt; <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> automation_task_metadata = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> task_details = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_indexes, |task_index| {
        <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&automation_task_metadata.tasks, task_index)) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> task_details, <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&automation_task_metadata.tasks, task_index))
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_has_sender_active_task_with_id">has_sender_active_task_with_id</a>(sender: <b>address</b>, task_index: u64): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> automation_task_metadata = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&automation_task_metadata.tasks, task_index)) {
        <b>let</b> value = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_ref">enumerable_map::get_value_ref</a>(&automation_task_metadata.tasks, task_index);
        value.state != <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a> && value.owner == sender
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_gas_committed_for_next_epoch">get_gas_committed_for_next_epoch</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_gas_committed_for_current_epoch">get_gas_committed_for_current_epoch</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch <b>as</b> u64)
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_registry_config">get_automation_registry_config</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework).main_config
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_next_epoch_registry_max_gas_cap">get_next_epoch_registry_max_gas_cap</a>(): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework).next_epoch_registry_max_gas_cap
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_get_automation_epoch_info">get_automation_epoch_info</a>(): <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
    *<b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework)
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
): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy">estimate_automation_fee_with_committed_occupancy</a>(task_occupancy, registry.gas_committed_for_next_epoch)
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
): u64 <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>let</b> epoch_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);
    <b>let</b> config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(
        task_occupancy,
        committed_occupancy,
        epoch_info,
        config
    )
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_is_registration_enabled">is_registration_enabled</a>(): bool <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework).registration_enabled
}
</code></pre>



</details>

<a id="0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal"></a>

## Function `estimate_automation_fee_with_committed_occupancy_internal`

Estimates automation fee the next epoch for specified task occupancy for the configured epoch-interval
referencing the current automation registry fee parameters, specified total/committed occupancy and registry
maximum allowed occupancy for the next epoch.
Note it is expected that committed_occupancy does not include currnet task's occupancy.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(task_occupancy: u64, committed_occupancy: u64, epoch_info: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>, active_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">automation_registry::ActiveAutomationRegistryConfig</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(
    task_occupancy: u64,
    committed_occupancy: u64,
    epoch_info: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>,
    active_config: &<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>
): u64 {
    <b>let</b> total_committed_max_gas = committed_occupancy + task_occupancy;

    // Compute the automation fee multiplier for epoch
    <b>let</b> automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(
        &active_config.main_config,
        (total_committed_max_gas <b>as</b> u256),
        active_config.next_epoch_registry_max_gas_cap);

    <b>if</b> (automation_fee_per_sec == 0) {
        <b>return</b> 0
    };

    <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_for_interval">calculate_automation_fee_for_interval</a>(
        epoch_info.epoch_interval,
        task_occupancy,
        automation_fee_per_sec,
        active_config.next_epoch_registry_max_gas_cap)
}
</code></pre>



</details>

<a id="0x1_automation_registry_validate_configuration_parameters_common"></a>

## Function `validate_configuration_parameters_common`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(epoch_interval_secs: u64, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, congestion_threshold_percentage: u8, congestion_exponent: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(
    epoch_interval_secs: u64,
    task_duration_cap_in_secs: u64,
    registry_max_gas_cap: u64,
    congestion_threshold_percentage: u8,
    congestion_exponent: u8,
) {
    <b>assert</b>!(congestion_threshold_percentage &lt;= <a href="automation_registry.md#0x1_automation_registry_MAX_PERCENTAGE">MAX_PERCENTAGE</a>, <a href="automation_registry.md#0x1_automation_registry_EMAX_CONGESTION_THRESHOLD">EMAX_CONGESTION_THRESHOLD</a>);
    <b>assert</b>!(congestion_exponent &gt; 0, <a href="automation_registry.md#0x1_automation_registry_ECONGESTION_EXP_NON_ZERO">ECONGESTION_EXP_NON_ZERO</a>);
    <b>assert</b>!(task_duration_cap_in_secs &gt; epoch_interval_secs, <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_TASK_DURATION_CAP">EUNACCEPTABLE_TASK_DURATION_CAP</a>);
    <b>assert</b>!(registry_max_gas_cap &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_MAX_GAS_CAP_NON_ZERO">EREGISTRY_MAX_GAS_CAP_NON_ZERO</a>);
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

<a id="0x1_automation_registry_initialize"></a>

## Function `initialize`

Initialization of Automation Registry with configuration parameters is expected metrics.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch_interval_secs: u64, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, automation_base_fee_in_quants_per_sec: u64, flat_registration_fee_in_quants: u64, congestion_threshold_percentage: u8, congestion_base_fee_in_quants_per_sec: u64, congestion_exponent: u8, task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_initialize">initialize</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(
        epoch_interval_secs,
        task_duration_cap_in_secs,
        registry_max_gas_cap,
        congestion_threshold_percentage,
        congestion_exponent);

    <b>let</b> (registry_fee_resource_signer, registry_fee_address_signer_cap) = <a href="automation_registry.md#0x1_automation_registry_create_registry_resource_account">create_registry_resource_account</a>(
        supra_framework
    );

    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
        tasks: <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_new_map">enumerable_map::new_map</a>(),
        current_index: 0,
        gas_committed_for_next_epoch: 0,
        epoch_locked_fees: 0,
        gas_committed_for_this_epoch: 0,
        registry_fee_address: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&registry_fee_resource_signer),
        registry_fee_address_signer_cap,
        epoch_active_task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    });

    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
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
        next_epoch_registry_max_gas_cap: registry_max_gas_cap,
        registration_enabled: <b>true</b>,
    });

    <b>move_to</b>(supra_framework, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
        expected_epoch_duration: epoch_interval_secs,
        epoch_interval: epoch_interval_secs,
        start_time: 0,
    });

    <a href="automation_registry.md#0x1_automation_registry_initialize_refund_bookkeeping_resource">initialize_refund_bookkeeping_resource</a>(supra_framework)
}
</code></pre>



</details>

<a id="0x1_automation_registry_initialize_refund_bookkeeping_resource"></a>

## Function `initialize_refund_bookkeeping_resource`



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

<a id="0x1_automation_registry_on_new_epoch"></a>

## Function `on_new_epoch`

On new epoch this function will be triggered and update the automation registry state


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_new_epoch">on_new_epoch</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_new_epoch">on_new_epoch</a>(
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    // Unless registry in initialized, registry will not be updated on new epoch.
    // Here we need <b>to</b> be careful <b>as</b> well. If the feature is disabled for the current epoch then
    //  - refund for the previous epoch should be done <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> charges <b>has</b> been done.
    //  - all tasks should be removed from registry state
    // Note that <b>with</b> the current setup feature::on_new_epoch is called before <a href="automation_registry.md#0x1_automation_registry_on_new_epoch">automation_registry::on_new_epoch</a>
    <b>if</b> (!<a href="automation_registry.md#0x1_automation_registry_is_initialized">is_initialized</a>()) {
        <b>return</b>
    };
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> automation_epoch_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);

    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(
        @supra_framework
    ).main_config;

    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> intermediate_state = <a href="automation_registry.md#0x1_automation_registry_update_state_for_new_epoch">update_state_for_new_epoch</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        refund_bookkeeping,
        &automation_registry_config,
        automation_epoch_info,
        current_time
    );


    // Apply the latest configuration <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> parameter <b>has</b> been updated
    // only after refund <b>has</b> been done for previous epoch.
    <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>();

    // If feature is not enabled then we are not charging and tasks are cleared.
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>()) {
        <a href="automation_registry.md#0x1_automation_registry_finalize_epoch_change_for_feature_disabled_state">finalize_epoch_change_for_feature_disabled_state</a>(
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
            automation_epoch_info,
            refund_bookkeeping,
            current_time,
            intermediate_state);
        <b>return</b>
    };

    <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fees">try_withdraw_task_automation_fees</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        refund_bookkeeping,
        &automation_registry_config,
        automation_epoch_info.epoch_interval,
        current_time,
        &<b>mut</b> intermediate_state,
    );

    <a href="automation_registry.md#0x1_automation_registry_finalize_epoch_change">finalize_epoch_change</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, automation_epoch_info, current_time, intermediate_state);
}
</code></pre>



</details>

<a id="0x1_automation_registry_finalize_epoch_change"></a>

## Function `finalize_epoch_change`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_finalize_epoch_change">finalize_epoch_change</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, automation_epoch_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>, current_time: u64, intermediate_state: <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_finalize_epoch_change">finalize_epoch_change</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    automation_epoch_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>,
    current_time: u64,
    intermediate_state: <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a>
) {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
        gas_committed_for_new_epoch,
        gas_committed_for_next_epoch,
        epoch_locked_fees,
        removed_tasks,
    } = intermediate_state;

    <b>let</b> epoch_locked_fees_value = <a href="coin.md#0x1_coin_value">coin::value</a>(&epoch_locked_fees);
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address, epoch_locked_fees);

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = gas_committed_for_next_epoch;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees = epoch_locked_fees_value;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch = (gas_committed_for_new_epoch <b>as</b> u256);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);

    automation_epoch_info.start_time = current_time;
    automation_epoch_info.expected_epoch_duration = automation_epoch_info.epoch_interval;
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_RemovedTasks">RemovedTasks</a> {
        task_indexes: removed_tasks
    });
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_ActiveTasks">ActiveTasks</a> {
        task_indexes: <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_finalize_epoch_change_for_feature_disabled_state"></a>

## Function `finalize_epoch_change_for_feature_disabled_state`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_finalize_epoch_change_for_feature_disabled_state">finalize_epoch_change_for_feature_disabled_state</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, automation_epoch_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, current_time: u64, intermediate_state: <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_finalize_epoch_change_for_feature_disabled_state">finalize_epoch_change_for_feature_disabled_state</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    automation_epoch_info: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    current_time: u64,
    intermediate_state: <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a>
) {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
        gas_committed_for_new_epoch: _,
        gas_committed_for_next_epoch: _,
        epoch_locked_fees,
        removed_tasks,
    } = intermediate_state;

    destroy_zero(epoch_locked_fees);

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch = 0;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund_all">safe_deposit_refund_all</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, refund_bookkeeping);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(
        &<b>mut</b> removed_tasks,
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks));
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_RemovedTasks">RemovedTasks</a> { task_indexes: removed_tasks });
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_ActiveTasks">ActiveTasks</a> {
        task_indexes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
    });
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_clear">enumerable_map::clear</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);

    automation_epoch_info.start_time = current_time;
    automation_epoch_info.expected_epoch_duration = automation_epoch_info.epoch_interval;
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_state_for_new_epoch"></a>

## Function `update_state_for_new_epoch`

Checks all tasks for refunds, cancellation and expirations.
Cleans the stale tasks and calculates gas-committed for the new epoch.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_state_for_new_epoch">update_state_for_new_epoch</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, aei: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>, current_time: u64): <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_state_for_new_epoch">update_state_for_new_epoch</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    aei: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>,
    current_time: u64
): <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
    <b>let</b> previous_epoch_duration = current_time - aei.start_time;
    <b>let</b> refund_interval = 0;
    <b>let</b> refund_automation_fee_per_sec = 0;

    // If epoch actual duration is greater or equal <b>to</b> expected epoch-duration then there is nothing <b>to</b> refund.
    <b>if</b> (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees != 0 && previous_epoch_duration &lt; aei.expected_epoch_duration) {
        <b>let</b> previous_tcmg = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch;
        refund_interval = aei.expected_epoch_duration - previous_epoch_duration;
        // Compute the automation fee multiplier for ended epoch
        refund_automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(arc, previous_tcmg, arc.registry_max_gas_cap);
    };

    <b>if</b> (refund_automation_fee_per_sec != 0) {
        <a href="automation_registry.md#0x1_automation_registry_refund_cleanup_tasks">refund_cleanup_tasks</a>(
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
            refund_bookkeeping,
            current_time,
            arc,
            refund_automation_fee_per_sec,
            refund_interval)
    } <b>else</b> {
        <a href="automation_registry.md#0x1_automation_registry_cleanup_tasks">cleanup_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, refund_bookkeeping, current_time)
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_refund_cleanup_tasks"></a>

## Function `refund_cleanup_tasks`

Refunds active tasks of the previous epoch, cleans up expired and cancelled tasks.
Also calculates and returns the total committed max gas for the new epoch along with the task indexes
that have been removed from the registry.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_cleanup_tasks">refund_cleanup_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, current_time: u64, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, refund_automation_fee_per_sec: u256, refund_interval: u64): <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_cleanup_tasks">refund_cleanup_tasks</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    current_time: u64,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    refund_automation_fee_per_sec: u256,
    refund_interval: u64,
): <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
    <b>let</b> ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);
    <b>let</b> tcmg = 0;
    <b>let</b> removed_tasks = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
    );
    <b>let</b> epoch_locked_fees = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees;

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(ids, |task_index| {
        <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
        <b>if</b> (task.state != <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
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

        // Drop or activate task for this current epoch.
        <b>if</b> (task.expiry_time &lt;= current_time || task.state == <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>) {
            <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
                refund_bookkeeping,
                &resource_signer,
                <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
                task.task_index,
                task.owner,
                task.locked_fee_for_next_epoch,
            task.locked_fee_for_next_epoch);
            <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> removed_tasks, task_index);
        } <b>else</b> {
            tcmg = tcmg + task.max_gas_amount;
        }
    });
    <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
        removed_tasks,
        gas_committed_for_new_epoch: tcmg,
        gas_committed_for_next_epoch: 0,
        epoch_locked_fees: <a href="coin.md#0x1_coin_zero">coin::zero</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_cleanup_tasks"></a>

## Function `cleanup_tasks`

Cleans up expired and cancelled.
Also calculates and returns the total committed max gas for the new epoch along with the task indexes
that have been removed from the registry.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cleanup_tasks">cleanup_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, current_time: u64): <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cleanup_tasks">cleanup_tasks</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    current_time: u64
): <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
    <b>let</b> ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);
    <b>let</b> tcmg = 0;
    <b>let</b> removed_tasks = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
    );
    // Perform clean up and updation of state (we can't <b>use</b> enumerable_map::for_each, <b>as</b> actually we need value <b>as</b> mutable ref)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(ids, |task_index| {
        <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
        // Drop or activate task for this current epoch.
        <b>if</b> (task.expiry_time &lt;= current_time || task.state == <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>) {
            <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
                refund_bookkeeping,
                &resource_signer,
                <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
                task.task_index,
                task.owner,
                task.locked_fee_for_next_epoch,
                task.locked_fee_for_next_epoch);
            <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> removed_tasks, task_index);
        } <b>else</b> {
            tcmg = tcmg + task.max_gas_amount;
        }
    });

    <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a> {
        removed_tasks,
        gas_committed_for_new_epoch: tcmg,
        gas_committed_for_next_epoch: 0,
        epoch_locked_fees: <a href="coin.md#0x1_coin_zero">coin::zero</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_automation_registry_safe_deposit_refund_all"></a>

## Function `safe_deposit_refund_all`

Traverses through all existing tasks and refunds deposited fee upon registration fully.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund_all">safe_deposit_refund_all</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund_all">safe_deposit_refund_all</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>) {
    <b>let</b> ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
    );
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(ids, |task_index| {
        <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_ref">enumerable_map::get_value_ref</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);

        <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
            refund_bookkeeping,
            &resource_signer,
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
            task.task_index,
            task.owner,
            task.locked_fee_for_next_epoch,
            task.locked_fee_for_next_epoch);
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

<a id="0x1_automation_registry_try_withdraw_task_automation_fees"></a>

## Function `try_withdraw_task_automation_fees`

Processes automation task fees by checking user balances and task's commitment on automation-fee, i.e. automation-fee-cap
- If the user has sufficient balance, deducts the fee and emits a success event.
- If the balance is insufficient, removes the task and emits a cancellation event.
- If calculated fee for the epoch surpasses task's automation-fee-cap task is removed and cancellation event is emitted.
Return estimated committed gas for the next epoch, locked automation fee amount for this epoch, and list of active task indexes


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fees">try_withdraw_task_automation_fees</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, epoch_interval: u64, current_time: u64, intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fees">try_withdraw_task_automation_fees</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    epoch_interval: u64,
    current_time: u64,
    intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a>,
) {
    // Compute the automation fee multiplier for epoch
    <b>let</b> automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(
        arc,
        (intermediate_state.gas_committed_for_new_epoch <b>as</b> u256),
        arc.registry_max_gas_cap);

    <b>let</b> task_ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);
    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
    );
    <b>let</b> current_epoch_end_time = current_time + epoch_interval;

    // Sort task indexes <b>to</b> charge automation fees in the tasks chronological order
    <a href="automation_registry.md#0x1_automation_registry_sort_vector">sort_vector</a>(&<b>mut</b> task_ids);

    // Process each active task and calculate fee for the epoch for the tasks
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_ids, |task_index| {
        <b>let</b> task = {
            <b>let</b> task_meta = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
            <b>let</b> fee= <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(arc, task_meta, epoch_interval, current_time, automation_fee_per_sec);
            // If the task reached this phase that means it is valid active task for the new epoch.
            // During cleanup all expired tasks <b>has</b> been removed from the registry but the state of the tasks is not updated.
            // As here we need <b>to</b> distinguish new tasks from already existing active tasks,
            // <b>as</b> the fee calculation for them will be different based on their active duration in the epoch.
            // For more details see calculate_task_fee function.
            task_meta.state = <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a>;
            <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">AutomationTaskFeeMeta</a> {
                task_index,
                owner: task_meta.owner,
                fee,
                expiry_time: task_meta.expiry_time,
                automation_fee_cap: task_meta.automation_fee_cap_for_epoch,
                max_gas_amount: task_meta.max_gas_amount,
                locked_deposit_fee: task_meta.locked_fee_for_next_epoch,
            }
        };
        <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
            refund_bookkeeping,
            &resource_signer,
            task,
            current_epoch_end_time,
            intermediate_state
        );
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_try_withdraw_task_automation_fee"></a>

## Function `try_withdraw_task_automation_fee`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">automation_registry::AutomationRefundBookkeeping</a>, resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">automation_registry::AutomationTaskFeeMeta</a>, current_epoch_end_time: u64, intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">automation_registry::IntermediateStateOfEpochChange</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    refund_bookkeeping: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>,
    resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFeeMeta">AutomationTaskFeeMeta</a>,
    current_epoch_end_time: u64,
    intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateStateOfEpochChange">IntermediateStateOfEpochChange</a>) {
    // Remove the automation task <b>if</b> the epoch fee cap is exceeded
    <b>if</b> (task.fee &gt; task.automation_fee_cap) {
        <a href="automation_registry.md#0x1_automation_registry_safe_deposit_refund">safe_deposit_refund</a>(
            refund_bookkeeping,
            resource_signer,
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
            task.task_index,
            task.owner,
            task.locked_deposit_fee,
            task.locked_deposit_fee
        );
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task.task_index);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> intermediate_state.removed_tasks, task.task_index);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledCapacitySurpassed">TaskCancelledCapacitySurpassed</a> {
            task_index: task.task_index,
            owner: task.owner,
            fee: task.fee,
            automation_fee_cap: task.automation_fee_cap,
        });
        <b>return</b>
    };
    <b>let</b> user_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(task.owner);
    <b>if</b> (user_balance &lt; task.fee) {
        // If the user does not have enough balance, remove the task, DON'T refund the locked deposit, but simply unlock it
        // and emit an <a href="event.md#0x1_event">event</a>
        <a href="automation_registry.md#0x1_automation_registry_safe_unlock_locked_deposit">safe_unlock_locked_deposit</a>(refund_bookkeeping, task.locked_deposit_fee, task.task_index);
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task.task_index);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> intermediate_state.removed_tasks, task.task_index);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledInsufficentBalance">TaskCancelledInsufficentBalance</a> {
            task_index: task.task_index,
            owner: task.owner,
            fee: task.fee,
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
    <b>if</b> (task.expiry_time &gt; current_epoch_end_time) {
        intermediate_state.gas_committed_for_next_epoch = intermediate_state.gas_committed_for_next_epoch + task.max_gas_amount;
    };
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_config_from_buffer"></a>

## Function `update_config_from_buffer`

The function updates the ActiveAutomationRegistryConfig structure with values extracted from the buffer, if the buffer exists.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>() <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>&gt;()) {
        <b>let</b> buffer = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>&gt;();
        <b>let</b> automation_registry_config = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> , <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="automation_registry.md#0x1_automation_registry_transfer_fee_to_account_internal">transfer_fee_to_account_internal</a>(<b>to</b>, amount);
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_RegistryFeeWithdraw">RegistryFeeWithdraw</a> { <b>to</b>, amount });
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


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_transfer_fee_to_account_internal">transfer_fee_to_account_internal</a>(<b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    <b>let</b> resource_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address);

    <b>assert</b>!(resource_balance &gt;= amount, <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>);

    <b>assert</b>!((resource_balance - amount)
        &gt;= <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees + refund_bookkeeping.total_deposited_automation_fee,
        <a href="automation_registry.md#0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE">EREQUEST_EXCEEDS_LOCKED_BALANCE</a>);

    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap
    );
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(&resource_signer, <b>to</b>, amount);
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_config"></a>

## Function `update_config`

Update Automation Registry Config


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config">update_config</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, automation_base_fee_in_quants_per_sec: u64, flat_registration_fee_in_quants: u64, congestion_threshold_percentage: u8, congestion_base_fee_in_quants_per_sec: u64, congestion_exponent: u8, task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_config">update_config</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_duration_cap_in_secs: u64,
    registry_max_gas_cap: u64,
    automation_base_fee_in_quants_per_sec: u64,
    flat_registration_fee_in_quants: u64,
    congestion_threshold_percentage: u8,
    congestion_base_fee_in_quants_per_sec: u64,
    congestion_exponent: u8,
    task_capacity: u16,
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> automation_epoch_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);

    <a href="automation_registry.md#0x1_automation_registry_validate_configuration_parameters_common">validate_configuration_parameters_common</a>(
        automation_epoch_info.epoch_interval,
        task_duration_cap_in_secs,
        registry_max_gas_cap,
        congestion_threshold_percentage,
        congestion_exponent);

    <b>assert</b>!(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch &lt; registry_max_gas_cap,
        <a href="automation_registry.md#0x1_automation_registry_EUNACCEPTABLE_AUTOMATION_GAS_LIMIT">EUNACCEPTABLE_AUTOMATION_GAS_LIMIT</a>
    );

    <b>let</b> new_automation_registry_config = <a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a> {
        task_duration_cap_in_secs,
        registry_max_gas_cap,
        automation_base_fee_in_quants_per_sec,
        flat_registration_fee_in_quants,
        congestion_threshold_percentage,
        congestion_base_fee_in_quants_per_sec,
        congestion_exponent,
        task_capacity
    };
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<b>copy</b> new_automation_registry_config);

    // next_epoch_registry_max_gas_cap will be <b>update</b> instantly
    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework);
    automation_registry_config.next_epoch_registry_max_gas_cap = registry_max_gas_cap;

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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_enable_registration">enable_registration</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework);
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


<pre><code><b>public</b> <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_disable_registration">disable_registration</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework);
    automation_registry_config.registration_enabled = <b>false</b>;
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_DisabledRegistrationEvent">DisabledRegistrationEvent</a> {});
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    // Guarding registration <b>if</b> feature is not enabled.
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>(), <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&aux_data), <a href="automation_registry.md#0x1_automation_registry_ENO_AUX_DATA_SUPPORTED">ENO_AUX_DATA_SUPPORTED</a>);

    <b>let</b> automation_registry_config = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework);
    <b>assert</b>!(automation_registry_config.registration_enabled, <a href="automation_registry.md#0x1_automation_registry_ETASK_REGISTRATION_DISABLED">ETASK_REGISTRATION_DISABLED</a>);

    // If registry is full, reject task registration
    <b>assert</b>!((<a href="automation_registry.md#0x1_automation_registry_get_task_count">get_task_count</a>() <b>as</b> u16) &lt; automation_registry_config.main_config.task_capacity, <a href="automation_registry.md#0x1_automation_registry_EREGISTRY_IS_FULL">EREGISTRY_IS_FULL</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> automation_epoch_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);

    //Well-formedness check of payload_tx is done in <b>native</b> layer beforehand.

    <b>let</b> registration_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <a href="automation_registry.md#0x1_automation_registry_check_registration_task_duration">check_registration_task_duration</a>(
        expiry_time,
        registration_time,
        &automation_registry_config.main_config,
        automation_epoch_info
    );

    <b>assert</b>!(gas_price_cap &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EINVALID_GAS_PRICE">EINVALID_GAS_PRICE</a>);
    <b>assert</b>!(max_gas_amount &gt; 0, <a href="automation_registry.md#0x1_automation_registry_EINVALID_MAX_GAS_AMOUNT">EINVALID_MAX_GAS_AMOUNT</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&tx_hash) == <a href="automation_registry.md#0x1_automation_registry_TXN_HASH_LENGTH">TXN_HASH_LENGTH</a>, <a href="automation_registry.md#0x1_automation_registry_EINVALID_TXN_HASH">EINVALID_TXN_HASH</a>);

    <b>let</b> committed_gas = (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch <b>as</b> u128) + (max_gas_amount <b>as</b> u128);
    <b>assert</b>!(committed_gas &lt;= <a href="automation_registry.md#0x1_automation_registry_MAX_U64">MAX_U64</a>, <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_OVERFLOW">EGAS_COMMITTEED_VALUE_OVERFLOW</a>);

    <b>let</b> committed_gas = (committed_gas <b>as</b> u64);
    <b>assert</b>!(committed_gas &lt;= automation_registry_config.next_epoch_registry_max_gas_cap, <a href="automation_registry.md#0x1_automation_registry_EGAS_AMOUNT_UPPER">EGAS_AMOUNT_UPPER</a>);

    // Check the automation fee capacity
    <b>let</b> estimated_automation_fee_for_epoch = <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(
        max_gas_amount,
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch,
        automation_epoch_info,
        automation_registry_config);
    <b>assert</b>!(automation_fee_cap_for_epoch &gt;= estimated_automation_fee_for_epoch,
        <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH">EINSUFFICIENT_AUTOMATION_FEE_CAP_FOR_EPOCH</a>
    );

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = committed_gas;
    <b>let</b> task_index = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index;

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

<a id="0x1_automation_registry_check_registration_task_duration"></a>

## Function `check_registration_task_duration`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_check_registration_task_duration">check_registration_task_duration</a>(expiry_time: u64, registration_time: u64, automation_registry_config: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, automation_epoch_info: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_check_registration_task_duration">check_registration_task_duration</a>(
    expiry_time: u64,
    registration_time: u64,
    automation_registry_config: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    automation_epoch_info: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>
) {
    <b>assert</b>!(expiry_time &gt; registration_time, <a href="automation_registry.md#0x1_automation_registry_EINVALID_EXPIRY_TIME">EINVALID_EXPIRY_TIME</a>);
    <b>let</b> task_duration = expiry_time - registration_time;
    <b>assert</b>!(task_duration &lt;= automation_registry_config.task_duration_cap_in_secs, <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_TIME_UPPER">EEXPIRY_TIME_UPPER</a>);

    // Check that task is valid at least in the next epoch
    <b>assert</b>!(
        expiry_time &gt; (automation_epoch_info.start_time + automation_epoch_info.epoch_interval),
        <a href="automation_registry.md#0x1_automation_registry_EEXPIRY_BEFORE_NEXT_EPOCH">EEXPIRY_BEFORE_NEXT_EPOCH</a>
    );
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> , <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>{
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);

    <b>let</b> automation_task_metadata = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
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

    <b>let</b> epoch_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);
    // This check means the task was expected <b>to</b> be executed in the next epoch, but it <b>has</b> been cancelled.
    // We need <b>to</b> remove its gas commitment from `gas_committed_for_next_epoch` for this particular task.
    <b>if</b> (automation_task_metadata.expiry_time &gt; (epoch_info.start_time + epoch_info.expected_epoch_duration)) {
        <b>assert</b>!(
            <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch &gt;= automation_task_metadata.max_gas_amount,
            <a href="automation_registry.md#0x1_automation_registry_EGAS_COMMITTEED_VALUE_UNDERFLOW">EGAS_COMMITTEED_VALUE_UNDERFLOW</a>
        );
        // Adjust the gas committed for the next epoch by subtracting the gas amount of the cancelled task
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch - automation_task_metadata.max_gas_amount;
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelled">TaskCancelled</a> { task_index: automation_task_metadata.task_index, owner });
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a> {
    // Ensure that task indexes are provided
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&task_indexes), <a href="automation_registry.md#0x1_automation_registry_EEMPTY_TASK_INDEXES">EEMPTY_TASK_INDEXES</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> arc = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework).main_config;
    <b>let</b> epoch_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);
    <b>let</b> refund_bookkeeping = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRefundBookkeeping">AutomationRefundBookkeeping</a>&gt;(@supra_framework);

    <b>let</b> tcmg = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch;

    // Compute the automation fee multiplier for epoch
    <b>let</b> automation_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_multiplier_for_epoch">calculate_automation_fee_multiplier_for_epoch</a>(&arc, tcmg, arc.registry_max_gas_cap);

    <b>let</b> stopped_task_details = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> total_refund_fee = 0;
    <b>let</b> epoch_locked_fees = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees;

    // Calculate refundable fee for this remaining time task in current epoch
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> epoch_end_time = epoch_info.expected_epoch_duration + epoch_info.start_time;
    <b>let</b> residual_interval = <b>if</b> (epoch_end_time &lt;= current_time) {
        0
    } <b>else</b> {
        epoch_end_time - current_time
    };


    // Loop through each task index <b>to</b> validate and stop the task
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(task_indexes, |task_index| {
        <b>if</b> (<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index)) {
            // Remove task from registry
            <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);

            // Ensure only the task owner can stop it
            <b>assert</b>!(task.owner == owner, <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>);

            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove_value">vector::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids, &task_index);

            // This check means the task was expected <b>to</b> be executed in the next epoch, but it <b>has</b> been stopped.
            // We need <b>to</b> remove its gas commitment from `gas_committed_for_next_epoch` for this particular task.
            // Also it checks that task should not be cancelled.
            <b>if</b> (task.state != <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a> && task.expiry_time &gt; epoch_end_time) {
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
                <a href="automation_registry.md#0x1_automation_registry_TaskStopped">TaskStopped</a> { task_index, deposit_refund, epoch_fee_refund }
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
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TasksStopped">TasksStopped</a> {
            tasks: stopped_task_details,
            owner
        });
    };
}
</code></pre>



</details>

<a id="0x1_automation_registry_update_epoch_interval_in_registry"></a>

## Function `update_epoch_interval_in_registry`

Update epoch interval in registry while actually update happens in block module


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_epoch_interval_in_registry">update_epoch_interval_in_registry</a>(epoch_interval_microsecs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_update_epoch_interval_in_registry">update_epoch_interval_in_registry</a>(epoch_interval_microsecs: u64) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework)) {
        <b>let</b> automation_epoch_info = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);
        automation_epoch_info.epoch_interval = epoch_interval_microsecs / <a href="automation_registry.md#0x1_automation_registry_MICROSECS_CONVERSION_FACTOR">MICROSECS_CONVERSION_FACTOR</a>;
    };
}
</code></pre>



</details>

<a id="0x1_automation_registry_sort_vector"></a>

## Function `sort_vector`

Insertion sort implementation for vector


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_sort_vector">sort_vector</a>(input: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_sort_vector">sort_vector</a>(input: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(input);
    <b>let</b> i = 1;
    <b>while</b> (i &lt; len) {
        <b>let</b> j = i;
        <b>let</b> to_be_sorted = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(input, j);
        <b>while</b> (j &gt; 0 && to_be_sorted &lt; *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(input, j - 1)) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(input, j, j - 1);
            j = j - 1;
        };
        i = i + 1;
    };
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

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
