
<a id="0x1_automation_registry"></a>

# Module `0x1::automation_registry`

Supra Automation Registry

This contract is part of the Supra Framework and is designed to manage automated task entries


-  [Resource `ActiveAutomationRegistryConfig`](#0x1_automation_registry_ActiveAutomationRegistryConfig)
-  [Resource `AutomationRegistryConfig`](#0x1_automation_registry_AutomationRegistryConfig)
-  [Resource `AutomationRegistry`](#0x1_automation_registry_AutomationRegistry)
-  [Resource `AutomationEpochInfo`](#0x1_automation_registry_AutomationEpochInfo)
-  [Resource `AutomationTaskMetaData`](#0x1_automation_registry_AutomationTaskMetaData)
-  [Struct `TaskRegistrationFeeWithdraw`](#0x1_automation_registry_TaskRegistrationFeeWithdraw)
-  [Struct `RegistryFeeWithdraw`](#0x1_automation_registry_RegistryFeeWithdraw)
-  [Struct `TaskEpochFeeWithdraw`](#0x1_automation_registry_TaskEpochFeeWithdraw)
-  [Struct `TaskFeeRefund`](#0x1_automation_registry_TaskFeeRefund)
-  [Struct `TaskCancelled`](#0x1_automation_registry_TaskCancelled)
-  [Struct `TasksStopped`](#0x1_automation_registry_TasksStopped)
-  [Struct `TaskStopped`](#0x1_automation_registry_TaskStopped)
-  [Struct `TaskCancelledInsufficentBalance`](#0x1_automation_registry_TaskCancelledInsufficentBalance)
-  [Struct `TaskCancelledCapacitySurpassed`](#0x1_automation_registry_TaskCancelledCapacitySurpassed)
-  [Struct `ErrorTaskDoesNotExist`](#0x1_automation_registry_ErrorTaskDoesNotExist)
-  [Struct `ErrorTaskDoesNotExistForWithdrawal`](#0x1_automation_registry_ErrorTaskDoesNotExistForWithdrawal)
-  [Struct `EnabledRegistrationEvent`](#0x1_automation_registry_EnabledRegistrationEvent)
-  [Struct `DisabledRegistrationEvent`](#0x1_automation_registry_DisabledRegistrationEvent)
-  [Struct `AutomationTaskFee`](#0x1_automation_registry_AutomationTaskFee)
-  [Struct `IntermediateState`](#0x1_automation_registry_IntermediateState)
-  [Constants](#@Constants_0)
-  [Function `active_task_ids`](#0x1_automation_registry_active_task_ids)
-  [Function `is_initialized`](#0x1_automation_registry_is_initialized)
-  [Function `is_feature_enabled_and_initialized`](#0x1_automation_registry_is_feature_enabled_and_initialized)
-  [Function `get_next_task_index`](#0x1_automation_registry_get_next_task_index)
-  [Function `get_task_count`](#0x1_automation_registry_get_task_count)
-  [Function `get_task_ids`](#0x1_automation_registry_get_task_ids)
-  [Function `get_epoch_locked_balance`](#0x1_automation_registry_get_epoch_locked_balance)
-  [Function `get_active_task_ids`](#0x1_automation_registry_get_active_task_ids)
-  [Function `get_task_details`](#0x1_automation_registry_get_task_details)
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
-  [Function `on_new_epoch`](#0x1_automation_registry_on_new_epoch)
-  [Function `adjust_tasks_epoch_fee_refund`](#0x1_automation_registry_adjust_tasks_epoch_fee_refund)
-  [Function `refund_tasks_fee`](#0x1_automation_registry_refund_tasks_fee)
-  [Function `cleanup_and_activate_tasks`](#0x1_automation_registry_cleanup_and_activate_tasks)
-  [Function `calculate_tasks_automation_fees`](#0x1_automation_registry_calculate_tasks_automation_fees)
-  [Function `calculate_task_fee`](#0x1_automation_registry_calculate_task_fee)
-  [Function `calculate_automation_fee_for_interval`](#0x1_automation_registry_calculate_automation_fee_for_interval)
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
-  [Function `sort_by_task_index`](#0x1_automation_registry_sort_by_task_index)
-  [Function `upscale_from_u8`](#0x1_automation_registry_upscale_from_u8)
-  [Function `upscale_from_u64`](#0x1_automation_registry_upscale_from_u64)
-  [Function `upscale_from_u256`](#0x1_automation_registry_upscale_from_u256)
-  [Function `downscale_to_u64`](#0x1_automation_registry_downscale_to_u64)
-  [Function `downscale_to_u256`](#0x1_automation_registry_downscale_to_u256)


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
 Fee locked for the task estimated for the next epoch at the start of the current epoch.
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

<a id="0x1_automation_registry_AutomationTaskFee"></a>

## Struct `AutomationTaskFee`

Represents the fee charged for an automation task execution and some additional information.


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

<a id="0x1_automation_registry_IntermediateState"></a>

## Struct `IntermediateState`

Represents intermediate state of the registry on epoch change.


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



<a id="0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE"></a>

Supra native automation feature is not initialized or enabled


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EDISABLED_AUTOMATION_FEATURE">EDISABLED_AUTOMATION_FEATURE</a>: u64 = 15;
</code></pre>



<a id="0x1_automation_registry_EEMPTY_TASK_INDEXES"></a>

Task index list is empty.


<pre><code><b>const</b> <a href="automation_registry.md#0x1_automation_registry_EEMPTY_TASK_INDEXES">EEMPTY_TASK_INDEXES</a>: u64 = 25;
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



<a id="0x1_automation_registry_active_task_ids"></a>

## Function `active_task_ids`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_active_task_ids">active_task_ids</a>(intermediate_state: <a href="automation_registry.md#0x1_automation_registry_IntermediateState">automation_registry::IntermediateState</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_active_task_ids">active_task_ids</a>(intermediate_state: <a href="automation_registry.md#0x1_automation_registry_IntermediateState">IntermediateState</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    intermediate_state.active_task_ids
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
    <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework)
        && <b>exists</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework)
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

Get locked balance of the resource account


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
    <a href="automation_registry.md#0x1_automation_registry_estimate_automation_fee_with_committed_occupancy_internal">estimate_automation_fee_with_committed_occupancy_internal</a>(task_occupancy, committed_occupancy, epoch_info, config)
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

    <b>let</b> congestion_base_fee_per_sec = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_congestion_fee">calculate_automation_congestion_fee</a>(
        &active_config.main_config,
        (total_committed_max_gas <b>as</b> u256),
        active_config.next_epoch_registry_max_gas_cap);

    <b>let</b> automation_fee_per_sec = (active_config.main_config.automation_base_fee_in_quants_per_sec <b>as</b> u256) +
        congestion_base_fee_per_sec;

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
){
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

    <b>let</b> (registry_fee_resource_signer, registry_fee_address_signer_cap) = <a href="automation_registry.md#0x1_automation_registry_create_registry_resource_account">create_registry_resource_account</a>(supra_framework);

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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="automation_registry.md#0x1_automation_registry_on_new_epoch">on_new_epoch</a>() <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
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

    <b>let</b> automation_registry_config = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(
        @supra_framework
    ).main_config;

    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();

    // Refund the task <b>if</b> the epoch was shorter than expected.
    <a href="automation_registry.md#0x1_automation_registry_adjust_tasks_epoch_fee_refund">adjust_tasks_epoch_fee_refund</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        &automation_registry_config,
        automation_epoch_info,
        current_time
    );

    // Apply the latest configuration <b>if</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> parameter <b>has</b> been updated
    // only after refund <b>has</b> been done for previous epoch.
    <a href="automation_registry.md#0x1_automation_registry_update_config_from_buffer">update_config_from_buffer</a>();

    // If feature is not enabled then we are not charging and tasks are cleared.
    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_native_automation_enabled">features::supra_native_automation_enabled</a>()) {

        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = 0;
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees = 0;
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch = 0;
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_clear">enumerable_map::clear</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);

        automation_epoch_info.start_time = current_time;
        automation_epoch_info.expected_epoch_duration = automation_epoch_info.epoch_interval;
        <b>return</b>
    };

    // Accumulated maximum gas amount of the registered tasks for the current epoch
    <b>let</b> tcmg = <a href="automation_registry.md#0x1_automation_registry_cleanup_and_activate_tasks">cleanup_and_activate_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, current_time);


    <b>let</b> tasks_automation_fees = <a href="automation_registry.md#0x1_automation_registry_calculate_tasks_automation_fees">calculate_tasks_automation_fees</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        &automation_registry_config,
        automation_epoch_info.epoch_interval,
        current_time,
        tcmg,
        <b>false</b>
    );

    <b>let</b> intermediate_state = <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fees">try_withdraw_task_automation_fees</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        tasks_automation_fees,
        current_time,
        automation_epoch_info.epoch_interval
    );

    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_next_epoch = intermediate_state.gas_committed_for_next_epoch;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees = intermediate_state.epoch_locked_fees;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch = tcmg;
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_active_task_ids = <a href="automation_registry.md#0x1_automation_registry_active_task_ids">active_task_ids</a>(intermediate_state);

    automation_epoch_info.start_time = current_time;
    automation_epoch_info.expected_epoch_duration = automation_epoch_info.epoch_interval;
}
</code></pre>



</details>

<a id="0x1_automation_registry_adjust_tasks_epoch_fee_refund"></a>

## Function `adjust_tasks_epoch_fee_refund`

Adjusts task fees and processes refunds when there's a change in epoch duration.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_adjust_tasks_epoch_fee_refund">adjust_tasks_epoch_fee_refund</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, aei: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">automation_registry::AutomationEpochInfo</a>, current_time: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_adjust_tasks_epoch_fee_refund">adjust_tasks_epoch_fee_refund</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    aei: &<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>,
    current_time: u64
) {
    // If no funds were locked for the previous epoch then there is nothing <b>to</b> refund.
    // This may happen when feature was disabled, and no automation task was registered and charged for the next epoch.
    <b>if</b> (<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees == 0) {
        <b>return</b>
    };

    // If epoch actual duration is greater or equal <b>to</b> expected epoch-duration then there is nothing <b>to</b> refund.
    <b>let</b> epoch_duration = current_time - aei.start_time;
    <b>if</b> (aei.expected_epoch_duration &lt;= epoch_duration) {
        <b>return</b>
    };

    <b>let</b> residual_time = aei.expected_epoch_duration - epoch_duration;
    <b>let</b> tcmg = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch;
    <b>let</b> registry_fee_address_signer_cap = &<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address_signer_cap;
    <b>let</b> tasks_automation_refund_fees = <a href="automation_registry.md#0x1_automation_registry_calculate_tasks_automation_fees">calculate_tasks_automation_fees</a>(
        <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>,
        arc,
        residual_time,
        current_time,
        tcmg,
        <b>true</b>
    );
    <a href="automation_registry.md#0x1_automation_registry_refund_tasks_fee">refund_tasks_fee</a>(registry_fee_address_signer_cap, tasks_automation_refund_fees);
}
</code></pre>



</details>

<a id="0x1_automation_registry_refund_tasks_fee"></a>

## Function `refund_tasks_fee`

Processes refunds for automation task fees.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_tasks_fee">refund_tasks_fee</a>(resource_signer_cap: &<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, tasks_automation_refund_fees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">automation_registry::AutomationTaskFee</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_refund_tasks_fee">refund_tasks_fee</a>(
    resource_signer_cap: &SignerCapability,
    tasks_automation_refund_fees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a>&gt;
) {
    <b>let</b> resource_signer = <a href="account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(resource_signer_cap);

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(tasks_automation_refund_fees, |task| {
        <b>let</b> task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a> = task;
        <b>if</b> (task.fee != 0) {
            <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(&resource_signer, task.owner, task.fee);
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskFeeRefund">TaskFeeRefund</a> { task_index: task.task_index, owner: task.owner, amount: task.fee });
        }
    });
}
</code></pre>



</details>

<a id="0x1_automation_registry_cleanup_and_activate_tasks"></a>

## Function `cleanup_and_activate_tasks`

Cleanup and activate the automation task also it's calculate and return total committed max gas


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cleanup_and_activate_tasks">cleanup_and_activate_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, current_time: u64): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_cleanup_and_activate_tasks">cleanup_and_activate_tasks</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, current_time: u64): u256 {
    <b>let</b> ids = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_map_list">enumerable_map::get_map_list</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks);
    <b>let</b> tcmg = 0;

    // Perform clean up and updation of state (we can't <b>use</b> enumerable_map::for_each, <b>as</b> actually we need value <b>as</b> mutable ref)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(ids, |task_index| {
        <b>if</b> (!<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index)) {
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_ErrorTaskDoesNotExist">ErrorTaskDoesNotExist</a> { task_index })
        } <b>else</b> {
            <b>let</b> task = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value_mut">enumerable_map::get_value_mut</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);

            // Drop or activate task for this current epoch.
            <b>if</b> (task.expiry_time &lt;= current_time || task.state == <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>) {
                <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
            } <b>else</b> {
                task.state = <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a>;
                tcmg = tcmg + (task.max_gas_amount <b>as</b> u256);
            }
        }
    });
    tcmg
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_tasks_automation_fees"></a>

## Function `calculate_tasks_automation_fees`

Calculates automation task fees for the active tasks for the provided interval with provided tcmg occupancy.
The CANCELLED tasks are also taken into account if include_cancelled_task is true.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_tasks_automation_fees">calculate_tasks_automation_fees</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, interval: u64, current_time: u64, tcmg: u256, include_cancelled_task: bool): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">automation_registry::AutomationTaskFee</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_tasks_automation_fees">calculate_tasks_automation_fees</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    interval: u64,
    current_time: u64,
    tcmg: u256,
    include_cancelled_task: bool
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a>&gt; {
    // Compute the automation congestion fee (acf) for the epoch
    <b>let</b> acf = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_congestion_fee">calculate_automation_congestion_fee</a>(arc, tcmg, arc.registry_max_gas_cap);
    <b>let</b> task_with_fees = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    // Automation fee per second is the sum of the automation base fee per second and congeation fee per second
    // calculated based on the current registry occupancy.
    <b>let</b> automation_fee_per_sec = acf + (arc.automation_base_fee_in_quants_per_sec <b>as</b> u256);

    // Return early <b>if</b> automation fee per second is 0
    <b>if</b> (automation_fee_per_sec == 0) {
        <b>return</b> task_with_fees
    };

    // Process each active task and calculate fee for the epoch for the tasks
    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_for_each_value_ref">enumerable_map::for_each_value_ref</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, |task| {
        <b>let</b> task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a> = task;
        <b>if</b> (task.state == <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a> || (include_cancelled_task && task.state == <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>)) {
            <b>let</b> task_fee = <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(arc, task, interval, current_time, automation_fee_per_sec);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> task_with_fees, <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a> {
                task_index: task.task_index,
                owner: task.owner,
                fee: task_fee,
            });
        }
    });
    task_with_fees
}
</code></pre>



</details>

<a id="0x1_automation_registry_calculate_task_fee"></a>

## Function `calculate_task_fee`

Calculates automation task fees for a single task at the time of new epoch.
This is supposed to be called only after removing expired task and must not be called for expired task.
It returns calculated task fee for the interval the task will be active.


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">automation_registry::AutomationRegistryConfig</a>, task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">automation_registry::AutomationTaskMetaData</a>, interval: u64, current_time: u64, automation_fee_per_sec: u256): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_calculate_task_fee">calculate_task_fee</a>(
    arc: &<a href="automation_registry.md#0x1_automation_registry_AutomationRegistryConfig">AutomationRegistryConfig</a>,
    task: &<a href="automation_registry.md#0x1_automation_registry_AutomationTaskMetaData">AutomationTaskMetaData</a>,
    interval: u64,
    current_time: u64,
    automation_fee_per_sec: u256
): u64 {
    <b>if</b> (task.expiry_time &lt;= current_time) { <b>return</b> 0 };
    // Subtraction is safe here, <b>as</b> we already excluded expired tasks
    <b>let</b> remaining_time = task.expiry_time - current_time;
    <b>let</b> min_interval = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_min">math64::min</a>(remaining_time, interval);
    <a href="automation_registry.md#0x1_automation_registry_calculate_automation_fee_for_interval">calculate_automation_fee_for_interval</a>(
        min_interval,
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
    <b>if</b> (threshold_usage &lt; threshold_percentage) 0
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


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fees">try_withdraw_task_automation_fees</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, tasks_automation_fees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">automation_registry::AutomationTaskFee</a>&gt;, current_time: u64, epoch_interval: u64): <a href="automation_registry.md#0x1_automation_registry_IntermediateState">automation_registry::IntermediateState</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fees">try_withdraw_task_automation_fees</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    tasks_automation_fees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a>&gt;,
    current_time: u64,
    epoch_interval: u64,
): <a href="automation_registry.md#0x1_automation_registry_IntermediateState">IntermediateState</a> {
    <b>let</b> intermediate_state = <a href="automation_registry.md#0x1_automation_registry_IntermediateState">IntermediateState</a> {
        gas_committed_for_next_epoch: 0,
        epoch_locked_fees: 0,
        active_task_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
    };

    <a href="automation_registry.md#0x1_automation_registry_sort_by_task_index">sort_by_task_index</a>(&<b>mut</b> tasks_automation_fees);

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(tasks_automation_fees, |task| {
        <b>let</b> task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a> = task;
        <b>if</b> (!<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task.task_index)) {
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_ErrorTaskDoesNotExistForWithdrawal">ErrorTaskDoesNotExistForWithdrawal</a> {task_index: task.task_index})
        } <b>else</b> {
            <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>, task, current_time, epoch_interval, &<b>mut</b> intermediate_state);
        };
    });
    intermediate_state
}
</code></pre>



</details>

<a id="0x1_automation_registry_try_withdraw_task_automation_fee"></a>

## Function `try_withdraw_task_automation_fee`



<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">automation_registry::AutomationRegistry</a>, task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">automation_registry::AutomationTaskFee</a>, current_time: u64, epoch_interval: u64, intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateState">automation_registry::IntermediateState</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_try_withdraw_task_automation_fee">try_withdraw_task_automation_fee</a>(
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>,
    task: <a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a>,
    current_time: u64,
    epoch_interval: u64,
    intermediate_state: &<b>mut</b> <a href="automation_registry.md#0x1_automation_registry_IntermediateState">IntermediateState</a>) {

    <b>let</b> task_metadata = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task.task_index);

    // Remove the automation task <b>if</b> the epoch fee cap is exceeded
    <b>if</b> (task.fee &gt; task_metadata.automation_fee_cap_for_epoch) {
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task.task_index);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledCapacitySurpassed">TaskCancelledCapacitySurpassed</a> {
            task_index: task.task_index,
            owner: task_metadata.owner,
            fee: task.fee,
            automation_fee_cap: task_metadata.automation_fee_cap_for_epoch,
        });
    } <b>else</b> {
        <b>let</b> user_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(task_metadata.owner);
        <b>if</b> (user_balance &lt; task.fee) {
            // If the user does not have enough balance, remove the task and emit an <a href="event.md#0x1_event">event</a>
            <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task.task_index);
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskCancelledInsufficentBalance">TaskCancelledInsufficentBalance</a> {
                task_index: task.task_index,
                owner: task_metadata.owner,
                fee: task.fee,
            });
        } <b>else</b> {
            // Charge the fee and emit a success <a href="event.md#0x1_event">event</a>
            <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(
                &<a href="create_signer.md#0x1_create_signer">create_signer</a>(task_metadata.owner),
                <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address,
                task.fee
            );
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskEpochFeeWithdraw">TaskEpochFeeWithdraw</a> {
                task_index: task.task_index,
                owner: task_metadata.owner,
                fee: task.fee,
            });
            // Total task fees deducted from the user's <a href="account.md#0x1_account">account</a>
            intermediate_state.epoch_locked_fees = intermediate_state.epoch_locked_fees + task.fee;
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> intermediate_state.active_task_ids, task.task_index);

            // Calculate gas commitment for the next epoch only for valid active tasks
            <b>if</b> (task_metadata.expiry_time &gt; (current_time + epoch_interval)) {
                intermediate_state.gas_committed_for_next_epoch = intermediate_state.gas_committed_for_next_epoch + task_metadata.max_gas_amount;
            };
        };
    }
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
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


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_transfer_fee_to_account_internal">transfer_fee_to_account_internal</a>(<b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> resource_balance = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address);

    <b>assert</b>!(resource_balance &gt;= amount, <a href="automation_registry.md#0x1_automation_registry_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>);

    <b>assert</b>!((resource_balance - amount) &gt;= <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.epoch_locked_fees, <a href="automation_registry.md#0x1_automation_registry_EREQUEST_EXCEEDS_LOCKED_BALANCE">EREQUEST_EXCEEDS_LOCKED_BALANCE</a>);

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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a> {
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
        committed_gas,
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
        locked_fee_for_next_epoch: 0
    };

    <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_add_value">enumerable_map::add_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index, automation_task_metadata);
    <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.current_index + 1;

    // Charge flat registration fee from the user at the time of registration
    <b>let</b> fee = automation_registry_config.main_config.flat_registration_fee_in_quants;
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;SupraCoin&gt;(owner_signer, <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.registry_fee_address, fee);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="automation_registry.md#0x1_automation_registry_TaskRegistrationFeeWithdraw">TaskRegistrationFeeWithdraw</a> { task_index, owner, fee });
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_contains">enumerable_map::contains</a>(&<a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index), <a href="automation_registry.md#0x1_automation_registry_EAUTOMATION_TASK_NOT_FOUND">EAUTOMATION_TASK_NOT_FOUND</a>);

    <b>let</b> automation_task_metadata = <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_get_value">enumerable_map::get_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>assert</b>!(automation_task_metadata.owner == owner, <a href="automation_registry.md#0x1_automation_registry_EUNAUTHORIZED_TASK_OWNER">EUNAUTHORIZED_TASK_OWNER</a>);
    <b>assert</b>!(automation_task_metadata.state != <a href="automation_registry.md#0x1_automation_registry_CANCELLED">CANCELLED</a>, <a href="automation_registry.md#0x1_automation_registry_EALREADY_CANCELLED">EALREADY_CANCELLED</a>);
    <b>if</b> (automation_task_metadata.state == <a href="automation_registry.md#0x1_automation_registry_PENDING">PENDING</a>) {
        <a href="../../supra-stdlib/doc/enumerable_map.md#0x1_enumerable_map_remove_value">enumerable_map::remove_value</a>(&<b>mut</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.tasks, task_index);
    } <b>else</b> <b>if</b> (automation_task_metadata.state == <a href="automation_registry.md#0x1_automation_registry_ACTIVE">ACTIVE</a>) {
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
) <b>acquires</b> <a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>, <a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>, <a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a> {
    // Ensure that task indexes are provided
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&task_indexes), <a href="automation_registry.md#0x1_automation_registry_EEMPTY_TASK_INDEXES">EEMPTY_TASK_INDEXES</a>);

    <b>let</b> owner = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer);
    <b>let</b> <a href="automation_registry.md#0x1_automation_registry">automation_registry</a> = <b>borrow_global_mut</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationRegistry">AutomationRegistry</a>&gt;(@supra_framework);
    <b>let</b> arc = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_ActiveAutomationRegistryConfig">ActiveAutomationRegistryConfig</a>&gt;(@supra_framework).main_config;
    <b>let</b> epoch_info = <b>borrow_global</b>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationEpochInfo">AutomationEpochInfo</a>&gt;(@supra_framework);

    <b>let</b> tcmg = <a href="automation_registry.md#0x1_automation_registry">automation_registry</a>.gas_committed_for_this_epoch;

    // Calculate the automation congestion fee
    <b>let</b> acf = <a href="automation_registry.md#0x1_automation_registry_calculate_automation_congestion_fee">calculate_automation_congestion_fee</a>(
        &arc,
        tcmg,
        arc.registry_max_gas_cap
    );

    // Total fee per second (base + congestion fee)
    <b>let</b> automation_fee_per_sec = acf + (arc.automation_base_fee_in_quants_per_sec <b>as</b> u256);

    <b>let</b> stopped_task_details = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> total_refund_fee = 0;

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

<a id="0x1_automation_registry_sort_by_task_index"></a>

## Function `sort_by_task_index`

Sorting vector implementation


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_sort_by_task_index">sort_by_task_index</a>(v: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">automation_registry::AutomationTaskFee</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="automation_registry.md#0x1_automation_registry_sort_by_task_index">sort_by_task_index</a>(v: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="automation_registry.md#0x1_automation_registry_AutomationTaskFee">AutomationTaskFee</a>&gt;) {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <b>let</b> j = i + 1;
        <b>while</b> (j &lt; len) {
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(v, i).task_index &gt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(v, j).task_index) {
                <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(v, i, j)
            };
            j = j + 1;
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


[move-book]: https://aptos.dev/move/book/SUMMARY
