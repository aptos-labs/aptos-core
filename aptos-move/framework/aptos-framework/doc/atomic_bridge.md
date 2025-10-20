
<a id="0x1_atomic_bridge"></a>

# Module `0x1::atomic_bridge`



-  [Resource `AptosCoinBurnCapability`](#0x1_atomic_bridge_AptosCoinBurnCapability)
-  [Resource `AptosCoinMintCapability`](#0x1_atomic_bridge_AptosCoinMintCapability)
-  [Resource `AptosFABurnCapabilities`](#0x1_atomic_bridge_AptosFABurnCapabilities)
-  [Resource `AptosFAMintCapabilities`](#0x1_atomic_bridge_AptosFAMintCapabilities)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_initialize)
-  [Function `store_aptos_coin_burn_cap`](#0x1_atomic_bridge_store_aptos_coin_burn_cap)
-  [Function `store_aptos_coin_mint_cap`](#0x1_atomic_bridge_store_aptos_coin_mint_cap)
-  [Function `mint`](#0x1_atomic_bridge_mint)
-  [Function `burn`](#0x1_atomic_bridge_burn)


<pre><code><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
</code></pre>



<a id="0x1_atomic_bridge_AptosCoinBurnCapability"></a>

## Resource `AptosCoinBurnCapability`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_AptosCoinMintCapability"></a>

## Resource `AptosCoinMintCapability`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_AptosFABurnCapabilities"></a>

## Resource `AptosFABurnCapabilities`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosFABurnCapabilities">AptosFABurnCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_AptosFAMintCapabilities"></a>

## Resource `AptosFAMintCapabilities`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_AptosFAMintCapabilities">AptosFAMintCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: <a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>: u64 = 198461;
</code></pre>



<a id="0x1_atomic_bridge_EATOMIC_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_initialize"></a>

## Function `initialize`

Initializes the atomic bridge by setting up necessary configurations.

@param aptos_framework The signer representing the Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.

@param aptos_framework The signer representing the Aptos framework.
@param burn_cap The burn capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _burn_cap: BurnCapability&lt;AptosCoin&gt;) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Stores the mint capability for AptosCoin.

@param aptos_framework The signer representing the Aptos framework.
@param mint_cap The mint capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _mint_cap: MintCapability&lt;AptosCoin&gt;) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_mint"></a>

## Function `mint`

Mints a specified amount of AptosCoin to a recipient's address.

@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.
@abort If the mint capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_mint">mint</a>(_recipient: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_mint">mint</a>(_recipient: <b>address</b>, _amount: u64) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_burn"></a>

## Function `burn`

Burns a specified amount of AptosCoin from an address.

@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.
@abort If the burn capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_burn">burn</a>(_from: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_burn">burn</a>(_from: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>



<a id="0x1_atomic_bridge_configuration"></a>

# Module `0x1::atomic_bridge_configuration`



-  [Resource `BridgeConfig`](#0x1_atomic_bridge_configuration_BridgeConfig)
-  [Struct `BridgeConfigOperatorUpdated`](#0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated)
-  [Struct `InitiatorTimeLockUpdated`](#0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated)
-  [Struct `CounterpartyTimeLockUpdated`](#0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_configuration_initialize)
-  [Function `update_bridge_operator`](#0x1_atomic_bridge_configuration_update_bridge_operator)
-  [Function `set_initiator_time_lock_duration`](#0x1_atomic_bridge_configuration_set_initiator_time_lock_duration)
-  [Function `set_counterparty_time_lock_duration`](#0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration)
-  [Function `initiator_timelock_duration`](#0x1_atomic_bridge_configuration_initiator_timelock_duration)
-  [Function `counterparty_timelock_duration`](#0x1_atomic_bridge_configuration_counterparty_timelock_duration)
-  [Function `bridge_operator`](#0x1_atomic_bridge_configuration_bridge_operator)
-  [Function `assert_is_caller_operator`](#0x1_atomic_bridge_configuration_assert_is_caller_operator)


<pre><code></code></pre>



<a id="0x1_atomic_bridge_configuration_BridgeConfig"></a>

## Resource `BridgeConfig`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfig">BridgeConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>initiator_time_lock: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>counterparty_time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated"></a>

## Struct `BridgeConfigOperatorUpdated`

Event emitted when the bridge operator is updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_BridgeConfigOperatorUpdated">BridgeConfigOperatorUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_operator: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated"></a>

## Struct `InitiatorTimeLockUpdated`

Event emitted when the initiator time lock has been updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_InitiatorTimeLockUpdated">InitiatorTimeLockUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated"></a>

## Struct `CounterpartyTimeLockUpdated`

Event emitted when the initiator time lock has been updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_CounterpartyTimeLockUpdated">CounterpartyTimeLockUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED"></a>

Error code for atomic bridge disabled


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>: u64 = 2;
</code></pre>



<a id="0x1_atomic_bridge_configuration_COUNTERPARTY_TIME_LOCK_DUARTION"></a>

Counterparty time lock duration is 24 hours in seconds


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_COUNTERPARTY_TIME_LOCK_DUARTION">COUNTERPARTY_TIME_LOCK_DUARTION</a>: u64 = 86400;
</code></pre>



<a id="0x1_atomic_bridge_configuration_EINVALID_BRIDGE_OPERATOR"></a>

Error code for invalid bridge operator


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EINVALID_BRIDGE_OPERATOR">EINVALID_BRIDGE_OPERATOR</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_configuration_INITIATOR_TIME_LOCK_DUARTION"></a>

Initiator time lock duration is 48 hours in seconds


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_INITIATOR_TIME_LOCK_DUARTION">INITIATOR_TIME_LOCK_DUARTION</a>: u64 = 172800;
</code></pre>



<a id="0x1_atomic_bridge_configuration_initialize"></a>

## Function `initialize`

Initializes the bridge configuration with Aptos framework as the bridge operator.

@param aptos_framework The signer representing the Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_update_bridge_operator"></a>

## Function `update_bridge_operator`

Updates the bridge operator, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_operator The new address to be set as the bridge operator.
@abort If the current operator is the same as the new operator.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_update_bridge_operator">update_bridge_operator</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_operator: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_update_bridge_operator">update_bridge_operator</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_operator: <b>address</b>
) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_set_initiator_time_lock_duration"></a>

## Function `set_initiator_time_lock_duration`



<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_initiator_time_lock_duration">set_initiator_time_lock_duration</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _time_lock: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_initiator_time_lock_duration">set_initiator_time_lock_duration</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _time_lock: u64
) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration"></a>

## Function `set_counterparty_time_lock_duration`



<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration">set_counterparty_time_lock_duration</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _time_lock: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_set_counterparty_time_lock_duration">set_counterparty_time_lock_duration</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _time_lock: u64
) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_initiator_timelock_duration"></a>

## Function `initiator_timelock_duration`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initiator_timelock_duration">initiator_timelock_duration</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_initiator_timelock_duration">initiator_timelock_duration</a>() : u64 {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_counterparty_timelock_duration"></a>

## Function `counterparty_timelock_duration`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_counterparty_timelock_duration">counterparty_timelock_duration</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_counterparty_timelock_duration">counterparty_timelock_duration</a>() : u64 {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_bridge_operator"></a>

## Function `bridge_operator`

Retrieves the address of the current bridge operator.

@return The address of the current bridge operator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_bridge_operator">bridge_operator</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_bridge_operator">bridge_operator</a>(): <b>address</b> {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_configuration_assert_is_caller_operator"></a>

## Function `assert_is_caller_operator`

Asserts that the caller is the current bridge operator.

@param caller The signer whose authority is being checked.
@abort If the caller is not the current bridge operator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_assert_is_caller_operator">assert_is_caller_operator</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_assert_is_caller_operator">assert_is_caller_operator</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_configuration_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>



<a id="0x1_atomic_bridge_store"></a>

# Module `0x1::atomic_bridge_store`



-  [Struct `AddressPair`](#0x1_atomic_bridge_store_AddressPair)
-  [Resource `SmartTableWrapper`](#0x1_atomic_bridge_store_SmartTableWrapper)
-  [Struct `BridgeTransferDetails`](#0x1_atomic_bridge_store_BridgeTransferDetails)
-  [Resource `Nonce`](#0x1_atomic_bridge_store_Nonce)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_store_initialize)
-  [Function `now`](#0x1_atomic_bridge_store_now)
-  [Function `create_time_lock`](#0x1_atomic_bridge_store_create_time_lock)
-  [Function `create_details`](#0x1_atomic_bridge_store_create_details)
-  [Function `add`](#0x1_atomic_bridge_store_add)
-  [Function `assert_min_time_lock`](#0x1_atomic_bridge_store_assert_min_time_lock)
-  [Function `assert_pending`](#0x1_atomic_bridge_store_assert_pending)
-  [Function `assert_valid_hash_lock`](#0x1_atomic_bridge_store_assert_valid_hash_lock)
-  [Function `assert_valid_bridge_transfer_id`](#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id)
-  [Function `create_hashlock`](#0x1_atomic_bridge_store_create_hashlock)
-  [Function `assert_correct_hash_lock`](#0x1_atomic_bridge_store_assert_correct_hash_lock)
-  [Function `assert_timed_out_lock`](#0x1_atomic_bridge_store_assert_timed_out_lock)
-  [Function `assert_within_timelock`](#0x1_atomic_bridge_store_assert_within_timelock)
-  [Function `complete`](#0x1_atomic_bridge_store_complete)
-  [Function `cancel`](#0x1_atomic_bridge_store_cancel)
-  [Function `complete_details`](#0x1_atomic_bridge_store_complete_details)
-  [Function `complete_transfer`](#0x1_atomic_bridge_store_complete_transfer)
-  [Function `cancel_details`](#0x1_atomic_bridge_store_cancel_details)
-  [Function `cancel_transfer`](#0x1_atomic_bridge_store_cancel_transfer)
-  [Function `bridge_transfer_id`](#0x1_atomic_bridge_store_bridge_transfer_id)
-  [Function `get_bridge_transfer_details_initiator`](#0x1_atomic_bridge_store_get_bridge_transfer_details_initiator)
-  [Function `get_bridge_transfer_details_counterparty`](#0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty)
-  [Function `get_bridge_transfer_details`](#0x1_atomic_bridge_store_get_bridge_transfer_details)


<pre><code><b>use</b> <a href="ethereum.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_atomic_bridge_store_AddressPair"></a>

## Struct `AddressPair`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddressPair">AddressPair</a>&lt;Initiator: store, Recipient: store&gt; <b>has</b> <b>copy</b>, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>initiator: Initiator</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: Recipient</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_store_SmartTableWrapper"></a>

## Resource `SmartTableWrapper`

A smart table wrapper


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_SmartTableWrapper">SmartTableWrapper</a>&lt;K, V&gt; <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_store_BridgeTransferDetails"></a>

## Struct `BridgeTransferDetails`

Details on the transfer


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator: store, Recipient: store&gt; <b>has</b> <b>copy</b>, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addresses: <a href="atomic_bridge.md#0x1_atomic_bridge_store_AddressPair">atomic_bridge_store::AddressPair</a>&lt;Initiator, Recipient&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>state: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_store_Nonce"></a>

## Resource `Nonce`



<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_Nonce">Nonce</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_store_MAX_U64"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>: u64 = 198461;
</code></pre>



<a id="0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_NOT_ENABLED">EATOMIC_BRIDGE_NOT_ENABLED</a>: u64 = 9;
</code></pre>



<a id="0x1_atomic_bridge_store_CANCELLED_TRANSACTION"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_CANCELLED_TRANSACTION">CANCELLED_TRANSACTION</a>: u8 = 3;
</code></pre>



<a id="0x1_atomic_bridge_store_COMPLETED_TRANSACTION"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_COMPLETED_TRANSACTION">COMPLETED_TRANSACTION</a>: u8 = 2;
</code></pre>



<a id="0x1_atomic_bridge_store_EEXPIRED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EEXPIRED">EEXPIRED</a>: u64 = 3;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_BRIDGE_TRANSFER_ID"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>: u64 = 8;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_HASH_LOCK"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_HASH_LOCK">EINVALID_HASH_LOCK</a>: u64 = 5;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_PRE_IMAGE"></a>

Error codes


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_PRE_IMAGE">EINVALID_PRE_IMAGE</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_store_EINVALID_TIME_LOCK"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_TIME_LOCK">EINVALID_TIME_LOCK</a>: u64 = 6;
</code></pre>



<a id="0x1_atomic_bridge_store_ENOT_EXPIRED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_EXPIRED">ENOT_EXPIRED</a>: u64 = 4;
</code></pre>



<a id="0x1_atomic_bridge_store_ENOT_PENDING_TRANSACTION"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_PENDING_TRANSACTION">ENOT_PENDING_TRANSACTION</a>: u64 = 2;
</code></pre>



<a id="0x1_atomic_bridge_store_EZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 7;
</code></pre>



<a id="0x1_atomic_bridge_store_MIN_TIME_LOCK"></a>

Minimum time lock of 1 second


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_MIN_TIME_LOCK">MIN_TIME_LOCK</a>: u64 = 1;
</code></pre>



<a id="0x1_atomic_bridge_store_PENDING_TRANSACTION"></a>

Transaction states


<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>: u8 = 1;
</code></pre>



<a id="0x1_atomic_bridge_store_initialize"></a>

## Function `initialize`

Initializes the initiators and counterparties tables and nonce.

@param aptos_framework The signer for Aptos framework.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_now"></a>

## Function `now`

Returns the current time in seconds.

@return Current timestamp in seconds.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() : u64 {
    <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_create_time_lock"></a>

## Function `create_time_lock`

Creates a time lock by adding a duration to the current time.

@param lock The duration to lock.
@return The calculated time lock.
@abort If lock is not above MIN_TIME_LOCK


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_time_lock">create_time_lock</a>(_time_lock: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_time_lock">create_time_lock</a>(_time_lock: u64) : u64 {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_create_details"></a>

## Function `create_details`

Creates bridge transfer details with validation.

@param initiator The initiating party of the transfer.
@param recipient The receiving party of the transfer.
@param amount The amount to be transferred.
@param hash_lock The hash lock for the transfer.
@param time_lock The time lock for the transfer.
@return A <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a></code> object.
@abort If the amount is zero or locks are invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">create_details</a>&lt;Initiator: store, Recipient: store&gt;(_initiator: Initiator, _recipient: Recipient, _amount: u64, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _time_lock: u64): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_details">create_details</a>&lt;Initiator: store, Recipient: store&gt;(_initiator: Initiator, _recipient: Recipient, _amount: u64, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _time_lock: u64)
    : <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt; {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_add"></a>

## Function `add`

Record details of a transfer

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">add</a>&lt;Initiator: store, Recipient: store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_add">add</a>&lt;Initiator: store, Recipient: store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _details: <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_min_time_lock"></a>

## Function `assert_min_time_lock`

Asserts that the time lock is valid.

@param time_lock
@abort If the time lock is invalid.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_min_time_lock">assert_min_time_lock</a>(_time_lock: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_min_time_lock">assert_min_time_lock</a>(_time_lock: u64) {
    <b>assert</b>!(_time_lock &gt;= <a href="atomic_bridge.md#0x1_atomic_bridge_store_MIN_TIME_LOCK">MIN_TIME_LOCK</a>, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_TIME_LOCK">EINVALID_TIME_LOCK</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_pending"></a>

## Function `assert_pending`

Asserts that the details state is pending.

@param details The bridge transfer details to check.
@abort If the state is not pending.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>assert</b>!(_details.state == <a href="atomic_bridge.md#0x1_atomic_bridge_store_PENDING_TRANSACTION">PENDING_TRANSACTION</a>, <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_PENDING_TRANSACTION">ENOT_PENDING_TRANSACTION</a>)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_valid_hash_lock"></a>

## Function `assert_valid_hash_lock`

Asserts that the hash lock is valid.

@param hash_lock The hash lock to validate.
@abort If the hash lock is invalid.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(_hash_lock: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(_hash_lock: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(_hash_lock) == 32, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_HASH_LOCK">EINVALID_HASH_LOCK</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_valid_bridge_transfer_id"></a>

## Function `assert_valid_bridge_transfer_id`

Asserts that the bridge transfer ID is valid.

@param bridge_transfer_id The bridge transfer ID to validate.
@abort If the ID is invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(_bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(_bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(_bridge_transfer_id) == 32, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_create_hashlock"></a>

## Function `create_hashlock`

Creates a hash lock from a pre-image.

@param pre_image The pre-image to hash.
@return The generated hash lock.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_hashlock">create_hashlock</a>(_pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_create_hashlock">create_hashlock</a>(_pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_correct_hash_lock"></a>

## Function `assert_correct_hash_lock`

Asserts that the hash lock matches the expected value.

@param details The bridge transfer details.
@param hash_lock The hash lock to compare.
@abort If the hash lock is incorrect.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_correct_hash_lock">assert_correct_hash_lock</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_correct_hash_lock">assert_correct_hash_lock</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(&_hash_lock == &_details.hash_lock, <a href="atomic_bridge.md#0x1_atomic_bridge_store_EINVALID_PRE_IMAGE">EINVALID_PRE_IMAGE</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_timed_out_lock"></a>

## Function `assert_timed_out_lock`

Asserts that the time lock has expired.

@param details The bridge transfer details.
@abort If the time lock has not expired.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_timed_out_lock">assert_timed_out_lock</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_timed_out_lock">assert_timed_out_lock</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>assert</b>!(<a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() &gt; _details.time_lock, <a href="atomic_bridge.md#0x1_atomic_bridge_store_ENOT_EXPIRED">ENOT_EXPIRED</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_assert_within_timelock"></a>

## Function `assert_within_timelock`

Asserts we are still within the timelock.

@param details The bridge transfer details.
@abort If the time lock has expired.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_within_timelock">assert_within_timelock</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_within_timelock">assert_within_timelock</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    <b>assert</b>!(!(<a href="atomic_bridge.md#0x1_atomic_bridge_store_now">now</a>() &gt; _details.time_lock), <a href="atomic_bridge.md#0x1_atomic_bridge_store_EEXPIRED">EEXPIRED</a>);
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_complete"></a>

## Function `complete`

Completes the bridge transfer.

@param details The bridge transfer details to complete.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    _details.state = <a href="atomic_bridge.md#0x1_atomic_bridge_store_COMPLETED_TRANSACTION">COMPLETED_TRANSACTION</a>;
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_cancel"></a>

## Function `cancel`

Cancels the bridge transfer.

@param details The bridge transfer details to cancel.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) {
    _details.state = <a href="atomic_bridge.md#0x1_atomic_bridge_store_CANCELLED_TRANSACTION">CANCELLED_TRANSACTION</a>;
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_complete_details"></a>

## Function `complete_details`

Validates and completes a bridge transfer by confirming the hash lock and state.

@param hash_lock The hash lock used to validate the transfer.
@param details The mutable reference to the bridge transfer details to be completed.
@return A tuple containing the recipient and the amount of the transfer.
@abort If the hash lock is invalid, the transfer is not pending, or the hash lock does not match.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_details">complete_details</a>&lt;Initiator: store, Recipient: <b>copy</b>, store&gt;(_hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): (Recipient, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_details">complete_details</a>&lt;Initiator: store, Recipient: store + <b>copy</b>&gt;(_hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) : (Recipient, u64) {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_valid_hash_lock">assert_valid_hash_lock</a>(&_hash_lock);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>(_details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_correct_hash_lock">assert_correct_hash_lock</a>(_details, _hash_lock);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_within_timelock">assert_within_timelock</a>(_details);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete">complete</a>(_details);

    (_details.addresses.recipient, _details.amount)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_complete_transfer"></a>

## Function `complete_transfer`

Completes a bridge transfer by validating the hash lock and updating the transfer state.

@param bridge_transfer_id The ID of the bridge transfer to complete.
@param hash_lock The hash lock used to validate the transfer.
@return A tuple containing the recipient of the transfer and the amount transferred.
@abort If the bridge transfer details are not found or if the completion checks in <code>complete_details</code> fail.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">complete_transfer</a>&lt;Initiator: store, Recipient: <b>copy</b>, store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (Recipient, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_complete_transfer">complete_transfer</a>&lt;Initiator: store, Recipient: <b>copy</b> + store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : (Recipient, u64) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_cancel_details"></a>

## Function `cancel_details`

Cancels a pending bridge transfer if the time lock has expired.

@param details A mutable reference to the bridge transfer details to be canceled.
@return A tuple containing the initiator of the transfer and the amount to be refunded.
@abort If the transfer is not in a pending state or the time lock has not expired.


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_details">cancel_details</a>&lt;Initiator: <b>copy</b>, store, Recipient: store&gt;(_details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): (Initiator, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_details">cancel_details</a>&lt;Initiator: store + <b>copy</b>, Recipient: store&gt;(_details: &<b>mut</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) : (Initiator, u64) {
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_pending">assert_pending</a>(_details);
    <a href="atomic_bridge.md#0x1_atomic_bridge_store_assert_timed_out_lock">assert_timed_out_lock</a>(_details);

    <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel">cancel</a>(_details);

    (_details.addresses.initiator, _details.amount)
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_cancel_transfer"></a>

## Function `cancel_transfer`

Cancels a bridge transfer if it is pending and the time lock has expired.

@param bridge_transfer_id The ID of the bridge transfer to cancel.
@return A tuple containing the initiator of the transfer and the amount to be refunded.
@abort If the bridge transfer details are not found or if the cancellation conditions in <code>cancel_details</code> fail.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_transfer">cancel_transfer</a>&lt;Initiator: <b>copy</b>, store, Recipient: store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (Initiator, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_cancel_transfer">cancel_transfer</a>&lt;Initiator: store + <b>copy</b>, Recipient: store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : (Initiator, u64) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_bridge_transfer_id"></a>

## Function `bridge_transfer_id`

Generates a unique bridge transfer ID based on transfer details and nonce.

@param details The bridge transfer details.
@return The generated bridge transfer ID.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_bridge_transfer_id">bridge_transfer_id</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_bridge_transfer_id">bridge_transfer_id</a>&lt;Initiator: store, Recipient: store&gt;(_details: &<a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_get_bridge_transfer_details_initiator"></a>

## Function `get_bridge_transfer_details_initiator`

Gets initiator bridge transfer details given a bridge transfer ID

@param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
@return A <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a></code> struct.
@abort If there is no transfer in the atomic bridge store.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_initiator">get_bridge_transfer_details_initiator</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;<b>address</b>, <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_initiator">get_bridge_transfer_details_initiator</a>(
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;<b>address</b>, EthereumAddress&gt; {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty"></a>

## Function `get_bridge_transfer_details_counterparty`

Gets counterparty bridge transfer details given a bridge transfer ID

@param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
@return A <code><a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a></code> struct.
@abort If there is no transfer in the atomic bridge store.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty">get_bridge_transfer_details_counterparty</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;<a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, <b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details_counterparty">get_bridge_transfer_details_counterparty</a>(
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;EthereumAddress, <b>address</b>&gt; {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_store_get_bridge_transfer_details"></a>

## Function `get_bridge_transfer_details`



<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details">get_bridge_transfer_details</a>&lt;Initiator: <b>copy</b>, store, Recipient: <b>copy</b>, store&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">atomic_bridge_store::BridgeTransferDetails</a>&lt;Initiator, Recipient&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_get_bridge_transfer_details">get_bridge_transfer_details</a>&lt;Initiator: store + <b>copy</b>, Recipient: store + <b>copy</b>&gt;(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="atomic_bridge.md#0x1_atomic_bridge_store_BridgeTransferDetails">BridgeTransferDetails</a>&lt;Initiator, Recipient&gt; {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_store_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>



<a id="0x1_atomic_bridge_counterparty"></a>

# Module `0x1::atomic_bridge_counterparty`



-  [Struct `BridgeTransferLockedEvent`](#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent)
-  [Struct `BridgeTransferCancelledEvent`](#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent)
-  [Resource `BridgeCounterpartyEvents`](#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_counterparty_initialize)
-  [Function `lock_bridge_transfer_assets`](#0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets)
-  [Function `complete_bridge_transfer`](#0x1_atomic_bridge_counterparty_complete_bridge_transfer)
-  [Function `abort_bridge_transfer`](#0x1_atomic_bridge_counterparty_abort_bridge_transfer)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
</code></pre>



<a id="0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent"></a>

## Struct `BridgeTransferLockedEvent`

An event triggered upon locking assets for a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">BridgeTransferLockedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent"></a>

## Struct `BridgeTransferCompletedEvent`

An event triggered upon completing a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent"></a>

## Struct `BridgeTransferCancelledEvent`

An event triggered upon cancelling a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">BridgeTransferCancelledEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents"></a>

## Resource `BridgeCounterpartyEvents`

This struct will store the event handles for bridge events.


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_locked_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">atomic_bridge_counterparty::BridgeTransferLockedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_completed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">atomic_bridge_counterparty::BridgeTransferCompletedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_cancelled_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">atomic_bridge_counterparty::BridgeTransferCancelledEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_counterparty_EATOMIC_BRIDGE_DISABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>: u64 = 198461;
</code></pre>



<a id="0x1_atomic_bridge_counterparty_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(aptos_framework, <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeCounterpartyEvents">BridgeCounterpartyEvents</a> {
        bridge_transfer_locked_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferLockedEvent">BridgeTransferLockedEvent</a>&gt;(aptos_framework),
        bridge_transfer_completed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a>&gt;(aptos_framework),
        bridge_transfer_cancelled_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_BridgeTransferCancelledEvent">BridgeTransferCancelledEvent</a>&gt;(aptos_framework),
    });
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets"></a>

## Function `lock_bridge_transfer_assets`

Locks assets for a bridge transfer by the initiator.

@param caller The signer representing the bridge operator.
@param initiator The initiator's Ethereum address as a vector of bytes.
@param bridge_transfer_id The unique identifier for the bridge transfer.
@param hash_lock The hash lock for securing the transfer.
@param time_lock The time lock duration for the transfer.
@param recipient The address of the recipient on the Aptos blockchain.
@param amount The amount of assets to be locked.
@abort If the caller is not the bridge operator.


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets">lock_bridge_transfer_assets</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _recipient: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_lock_bridge_transfer_assets">lock_bridge_transfer_assets</a> (
    _caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _recipient: <b>address</b>,
    _amount: u64
) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_counterparty_complete_bridge_transfer"></a>

## Function `complete_bridge_transfer`

Completes a bridge transfer by revealing the pre-image.

@param bridge_transfer_id The unique identifier for the bridge transfer.
@param pre_image The pre-image that matches the hash lock to complete the transfer.
@abort If the caller is not the bridge operator or the hash lock validation fails.


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_complete_bridge_transfer">complete_bridge_transfer</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_complete_bridge_transfer">complete_bridge_transfer</a> (
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_counterparty_abort_bridge_transfer"></a>

## Function `abort_bridge_transfer`

Aborts a bridge transfer if the time lock has expired.

@param caller The signer representing the bridge operator.
@param bridge_transfer_id The unique identifier for the bridge transfer.
@abort If the caller is not the bridge operator or if the time lock has not expired.


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_abort_bridge_transfer">abort_bridge_transfer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_abort_bridge_transfer">abort_bridge_transfer</a> (
    _caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_counterparty_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>



<a id="0x1_atomic_bridge_initiator"></a>

# Module `0x1::atomic_bridge_initiator`



-  [Struct `BridgeTransferInitiatedEvent`](#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent)
-  [Struct `BridgeTransferRefundedEvent`](#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent)
-  [Resource `BridgeInitiatorEvents`](#0x1_atomic_bridge_initiator_BridgeInitiatorEvents)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_atomic_bridge_initiator_initialize)
-  [Function `initiate_bridge_transfer`](#0x1_atomic_bridge_initiator_initiate_bridge_transfer)
-  [Function `complete_bridge_transfer`](#0x1_atomic_bridge_initiator_complete_bridge_transfer)
-  [Function `refund_bridge_transfer`](#0x1_atomic_bridge_initiator_refund_bridge_transfer)


<pre><code><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
</code></pre>



<a id="0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent"></a>

## Struct `BridgeTransferInitiatedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>initiator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>time_lock: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent"></a>

## Struct `BridgeTransferCompletedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent"></a>

## Struct `BridgeTransferRefundedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent">BridgeTransferRefundedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_atomic_bridge_initiator_BridgeInitiatorEvents"></a>

## Resource `BridgeInitiatorEvents`

This struct will store the event handles for bridge events.


<pre><code><b>struct</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeInitiatorEvents">BridgeInitiatorEvents</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_initiated_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferInitiatedEvent">atomic_bridge_initiator::BridgeTransferInitiatedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_completed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferCompletedEvent">atomic_bridge_initiator::BridgeTransferCompletedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_refunded_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="atomic_bridge.md#0x1_atomic_bridge_initiator_BridgeTransferRefundedEvent">atomic_bridge_initiator::BridgeTransferRefundedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_atomic_bridge_initiator_EATOMIC_BRIDGE_DISABLED"></a>



<pre><code><b>const</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>: u64 = 198461;
</code></pre>



<a id="0x1_atomic_bridge_initiator_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {

}
</code></pre>



</details>

<a id="0x1_atomic_bridge_initiator_initiate_bridge_transfer"></a>

## Function `initiate_bridge_transfer`

Initiate a bridge transfer of ETH from Movement to the base layer
Anyone can initiate a bridge transfer from the source chain
The amount is burnt from the initiator


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initiate_bridge_transfer">initiate_bridge_transfer</a>(_initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_initiate_bridge_transfer">initiate_bridge_transfer</a>(
    _initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _hash_lock: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _amount: u64
) {
    <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_initiator_complete_bridge_transfer"></a>

## Function `complete_bridge_transfer`

Bridge operator can complete the transfer


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_complete_bridge_transfer">complete_bridge_transfer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_complete_bridge_transfer">complete_bridge_transfer</a> (
    _caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _pre_image: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>

<a id="0x1_atomic_bridge_initiator_refund_bridge_transfer"></a>

## Function `refund_bridge_transfer`

Anyone can refund the transfer on the source chain once time lock has passed


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_refund_bridge_transfer">refund_bridge_transfer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_refund_bridge_transfer">refund_bridge_transfer</a> (
    _caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
   <b>abort</b> <a href="atomic_bridge.md#0x1_atomic_bridge_initiator_EATOMIC_BRIDGE_DISABLED">EATOMIC_BRIDGE_DISABLED</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
