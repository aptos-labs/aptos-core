
<a id="0x1_native_bridge"></a>

# Module `0x1::native_bridge`



-  [Struct `BridgeConfigRelayerUpdated`](#0x1_native_bridge_BridgeConfigRelayerUpdated)
-  [Struct `BridgeFeeChangedEvent`](#0x1_native_bridge_BridgeFeeChangedEvent)
-  [Struct `BridgeInsuranceBudgetDividerChangedEvent`](#0x1_native_bridge_BridgeInsuranceBudgetDividerChangedEvent)
-  [Struct `BridgeInsuranceFundChangedEvent`](#0x1_native_bridge_BridgeInsuranceFundChangedEvent)
-  [Struct `BridgeTransferInitiatedEvent`](#0x1_native_bridge_BridgeTransferInitiatedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_native_bridge_BridgeTransferCompletedEvent)
-  [Resource `BridgeEvents`](#0x1_native_bridge_BridgeEvents)
-  [Resource `AptosCoinBurnCapability`](#0x1_native_bridge_AptosCoinBurnCapability)
-  [Resource `AptosCoinMintCapability`](#0x1_native_bridge_AptosCoinMintCapability)
-  [Resource `AptosFABurnCapabilities`](#0x1_native_bridge_AptosFABurnCapabilities)
-  [Resource `AptosFAMintCapabilities`](#0x1_native_bridge_AptosFAMintCapabilities)
-  [Resource `Nonce`](#0x1_native_bridge_Nonce)
-  [Resource `OutboundRateLimitBudget`](#0x1_native_bridge_OutboundRateLimitBudget)
-  [Resource `InboundRateLimitBudget`](#0x1_native_bridge_InboundRateLimitBudget)
-  [Resource `SmartTableWrapper`](#0x1_native_bridge_SmartTableWrapper)
-  [Struct `OutboundTransfer`](#0x1_native_bridge_OutboundTransfer)
-  [Resource `BridgeConfig`](#0x1_native_bridge_BridgeConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_native_bridge_initialize)
-  [Function `normalize_u64_to_32_bytes`](#0x1_native_bridge_normalize_u64_to_32_bytes)
-  [Function `is_inbound_nonce_set`](#0x1_native_bridge_is_inbound_nonce_set)
-  [Function `create_details`](#0x1_native_bridge_create_details)
-  [Function `add`](#0x1_native_bridge_add)
-  [Function `set_bridge_transfer_id_to_inbound_nonce`](#0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce)
-  [Function `assert_valid_bridge_transfer_id`](#0x1_native_bridge_assert_valid_bridge_transfer_id)
-  [Function `bridge_transfer_id`](#0x1_native_bridge_bridge_transfer_id)
-  [Function `bridge_relayer`](#0x1_native_bridge_bridge_relayer)
-  [Function `insurance_fund`](#0x1_native_bridge_insurance_fund)
-  [Function `insurance_budget_divider`](#0x1_native_bridge_insurance_budget_divider)
-  [Function `bridge_fee`](#0x1_native_bridge_bridge_fee)
-  [Function `get_bridge_transfer_details_from_nonce`](#0x1_native_bridge_get_bridge_transfer_details_from_nonce)
-  [Function `get_inbound_nonce_from_bridge_transfer_id`](#0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id)
-  [Function `increment_and_get_nonce`](#0x1_native_bridge_increment_and_get_nonce)
-  [Function `store_aptos_coin_burn_cap`](#0x1_native_bridge_store_aptos_coin_burn_cap)
-  [Function `store_aptos_coin_mint_cap`](#0x1_native_bridge_store_aptos_coin_mint_cap)
-  [Function `mint_to`](#0x1_native_bridge_mint_to)
-  [Function `mint`](#0x1_native_bridge_mint)
-  [Function `mint_internal`](#0x1_native_bridge_mint_internal)
-  [Function `burn_from`](#0x1_native_bridge_burn_from)
-  [Function `burn`](#0x1_native_bridge_burn)
-  [Function `burn_internal`](#0x1_native_bridge_burn_internal)
-  [Function `initiate_bridge_transfer`](#0x1_native_bridge_initiate_bridge_transfer)
-  [Function `complete_bridge_transfer`](#0x1_native_bridge_complete_bridge_transfer)
-  [Function `charge_bridge_fee`](#0x1_native_bridge_charge_bridge_fee)
-  [Function `update_bridge_relayer`](#0x1_native_bridge_update_bridge_relayer)
-  [Function `update_bridge_fee`](#0x1_native_bridge_update_bridge_fee)
-  [Function `update_insurance_fund`](#0x1_native_bridge_update_insurance_fund)
-  [Function `update_insurance_budget_divider`](#0x1_native_bridge_update_insurance_budget_divider)
-  [Function `assert_is_caller_relayer`](#0x1_native_bridge_assert_is_caller_relayer)
-  [Function `assert_outbound_rate_limit_budget_not_exceeded`](#0x1_native_bridge_assert_outbound_rate_limit_budget_not_exceeded)
-  [Function `assert_inbound_rate_limit_budget_not_exceeded`](#0x1_native_bridge_assert_inbound_rate_limit_budget_not_exceeded)
-  [Function `test_normalize_u64_to_32_bytes_helper`](#0x1_native_bridge_test_normalize_u64_to_32_bytes_helper)


<pre><code><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="ethereum.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
</code></pre>



<a id="0x1_native_bridge_BridgeConfigRelayerUpdated"></a>

## Struct `BridgeConfigRelayerUpdated`

Event emitted when the bridge relayer is updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfigRelayerUpdated">BridgeConfigRelayerUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_relayer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_relayer: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeFeeChangedEvent"></a>

## Struct `BridgeFeeChangedEvent`

An event triggered upon change of bridgefee


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeFeeChangedEvent">BridgeFeeChangedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_bridge_fee: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_bridge_fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeInsuranceBudgetDividerChangedEvent"></a>

## Struct `BridgeInsuranceBudgetDividerChangedEvent`

An event triggered upon change of insurance budget divider


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeInsuranceBudgetDividerChangedEvent">BridgeInsuranceBudgetDividerChangedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_insurance_budget_divider: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_insurance_budget_divider: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeInsuranceFundChangedEvent"></a>

## Struct `BridgeInsuranceFundChangedEvent`

An event triggered upon change of insurance fund


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeInsuranceFundChangedEvent">BridgeInsuranceFundChangedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_insurance_fund: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_insurance_fund: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeTransferInitiatedEvent"></a>

## Struct `BridgeTransferInitiatedEvent`

An event triggered upon initiating a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> <b>has</b> drop, store
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
<code>nonce: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeTransferCompletedEvent"></a>

## Struct `BridgeTransferCompletedEvent`

An event triggered upon completing a bridge transfer


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> <b>has</b> drop, store
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
<code>nonce: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_BridgeEvents"></a>

## Resource `BridgeEvents`

This struct will store the event handles for bridge events.


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_transfer_initiated_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">native_bridge::BridgeTransferInitiatedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_transfer_completed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">native_bridge::BridgeTransferCompletedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_AptosCoinBurnCapability"></a>

## Resource `AptosCoinBurnCapability`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> <b>has</b> key
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

<a id="0x1_native_bridge_AptosCoinMintCapability"></a>

## Resource `AptosCoinMintCapability`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> <b>has</b> key
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

<a id="0x1_native_bridge_AptosFABurnCapabilities"></a>

## Resource `AptosFABurnCapabilities`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_AptosFABurnCapabilities">AptosFABurnCapabilities</a> <b>has</b> key
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

<a id="0x1_native_bridge_AptosFAMintCapabilities"></a>

## Resource `AptosFAMintCapabilities`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_AptosFAMintCapabilities">AptosFAMintCapabilities</a> <b>has</b> key
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

<a id="0x1_native_bridge_Nonce"></a>

## Resource `Nonce`

A nonce to ensure the uniqueness of bridge transfers


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_OutboundRateLimitBudget"></a>

## Resource `OutboundRateLimitBudget`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_OutboundRateLimitBudget">OutboundRateLimitBudget</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>day: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;u64, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_InboundRateLimitBudget"></a>

## Resource `InboundRateLimitBudget`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_InboundRateLimitBudget">InboundRateLimitBudget</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>day: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;u64, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_native_bridge_SmartTableWrapper"></a>

## Resource `SmartTableWrapper`

A smart table wrapper


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;K, V&gt; <b>has</b> store, key
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

<a id="0x1_native_bridge_OutboundTransfer"></a>

## Struct `OutboundTransfer`

Details on the outbound transfer


<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a> <b>has</b> <b>copy</b>, store
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
<code>recipient: <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a></code>
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

<a id="0x1_native_bridge_BridgeConfig"></a>

## Resource `BridgeConfig`



<pre><code><b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bridge_relayer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>insurance_fund: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>insurance_budget_divider: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bridge_fee: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>: u64 = 2;
</code></pre>



<a id="0x1_native_bridge_EZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 7;
</code></pre>



<a id="0x1_native_bridge_EEVENT_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EEVENT_NOT_FOUND">EEVENT_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x1_native_bridge_EID_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EID_NOT_FOUND">EID_NOT_FOUND</a>: u64 = 10;
</code></pre>



<a id="0x1_native_bridge_EINCORRECT_NONCE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINCORRECT_NONCE">EINCORRECT_NONCE</a>: u64 = 9;
</code></pre>



<a id="0x1_native_bridge_EINVALID_AMOUNT"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_AMOUNT">EINVALID_AMOUNT</a>: u64 = 5;
</code></pre>



<a id="0x1_native_bridge_EINVALID_BRIDGE_RELAYER"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_RELAYER">EINVALID_BRIDGE_RELAYER</a>: u64 = 11;
</code></pre>



<a id="0x1_native_bridge_EINVALID_NONCE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_NONCE">EINVALID_NONCE</a>: u64 = 4;
</code></pre>



<a id="0x1_native_bridge_EINVALID_VALUE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_VALUE">EINVALID_VALUE</a>: u64 = 3;
</code></pre>



<a id="0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>: u64 = 8;
</code></pre>



<a id="0x1_native_bridge_ENONCE_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ENONCE_NOT_FOUND">ENONCE_NOT_FOUND</a>: u64 = 6;
</code></pre>



<a id="0x1_native_bridge_ERATE_LIMIT_EXCEEDED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ERATE_LIMIT_EXCEEDED">ERATE_LIMIT_EXCEEDED</a>: u64 = 4;
</code></pre>



<a id="0x1_native_bridge_ESAME_FEE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ESAME_FEE">ESAME_FEE</a>: u64 = 2;
</code></pre>



<a id="0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED">ETRANSFER_ALREADY_PROCESSED</a>: u64 = 1;
</code></pre>



<a id="0x1_native_bridge_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {

}
</code></pre>



</details>

<a id="0x1_native_bridge_normalize_u64_to_32_bytes"></a>

## Function `normalize_u64_to_32_bytes`

Converts a u64 to a 32-byte vector.

@param value The u64 value to convert.
@return A 32-byte vector containing the u64 value in little-endian order.

How BCS works: https://github.com/zefchain/bcs?tab=readme-ov-file#booleans-and-integers

@example: a u64 value 0x12_34_56_78_ab_cd_ef_00 is converted to a 32-byte vector:
[0x00, 0x00, ..., 0x00, 0x12, 0x34, 0x56, 0x78, 0xab, 0xcd, 0xef, 0x00]


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(_value: &u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(_value: &u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_is_inbound_nonce_set"></a>

## Function `is_inbound_nonce_set`

Checks if a bridge transfer ID is associated with an inbound nonce.
@param bridge_transfer_id The bridge transfer ID.
@return <code><b>true</b></code> if the ID is associated with an existing inbound nonce, <code><b>false</b></code> otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_is_inbound_nonce_set">is_inbound_nonce_set</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_is_inbound_nonce_set">is_inbound_nonce_set</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_create_details"></a>

## Function `create_details`

Creates bridge transfer details with validation.

@param initiator The initiating party of the transfer.
@param recipient The receiving party of the transfer.
@param amount The amount to be transferred.
@param nonce The unique nonce for the transfer.
@return A <code>BridgeTransferDetails</code> object.
@abort If the amount is zero or locks are invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_create_details">create_details</a>(_initiator: <b>address</b>, _recipient: <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, _amount: u64, _nonce: u64): <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">native_bridge::OutboundTransfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_create_details">create_details</a>(_initiator: <b>address</b>, _recipient: EthereumAddress, _amount: u64, _nonce: u64)
    : <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a> {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_add"></a>

## Function `add`

Record details of an initiated transfer for quick lookup of details, mapping bridge transfer ID to transfer details

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_add">add</a>(_nonce: u64, _details: <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">native_bridge::OutboundTransfer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_add">add</a>(_nonce: u64, _details: <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a>)  {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce"></a>

## Function `set_bridge_transfer_id_to_inbound_nonce`

Record details of a completed transfer, mapping bridge transfer ID to inbound nonce

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _inbound_nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _inbound_nonce: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_valid_bridge_transfer_id"></a>

## Function `assert_valid_bridge_transfer_id`

Asserts that the bridge transfer ID is valid.

@param bridge_transfer_id The bridge transfer ID to validate.
@abort If the ID is invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(_bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(_bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_bridge_transfer_id"></a>

## Function `bridge_transfer_id`

Generates a unique outbound bridge transfer ID based on transfer details and nonce.

@param details The bridge transfer details.
@return The generated bridge transfer ID.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_transfer_id">bridge_transfer_id</a>(_initiator: <b>address</b>, _recipient: <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, _amount: u64, _nonce: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_transfer_id">bridge_transfer_id</a>(_initiator: <b>address</b>, _recipient: EthereumAddress, _amount: u64, _nonce: u64) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_bridge_relayer"></a>

## Function `bridge_relayer`

Retrieves the address of the current bridge relayer.

@return The address of the current bridge relayer.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_relayer">bridge_relayer</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_relayer">bridge_relayer</a>(): <b>address</b> {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_insurance_fund"></a>

## Function `insurance_fund`

Retrieves the address of the current insurance fund.

@return The address of the current insurance fund.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_insurance_fund">insurance_fund</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_insurance_fund">insurance_fund</a>(): <b>address</b> {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_insurance_budget_divider"></a>

## Function `insurance_budget_divider`

Retrieves the current insurance budget divider.

@return The current insurance budget divider.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_insurance_budget_divider">insurance_budget_divider</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_insurance_budget_divider">insurance_budget_divider</a>(): u64 {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_bridge_fee"></a>

## Function `bridge_fee`

Retrieves the current bridge fee.

@return The current bridge fee.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_fee">bridge_fee</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_fee">bridge_fee</a>(): u64 {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_get_bridge_transfer_details_from_nonce"></a>

## Function `get_bridge_transfer_details_from_nonce`

Gets the bridge transfer details (<code><a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a></code>) from the given nonce.
@param nonce The nonce of the bridge transfer.
@return The <code><a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a></code> struct containing the transfer details.
@abort If the nonce is not found in the smart table.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_bridge_transfer_details_from_nonce">get_bridge_transfer_details_from_nonce</a>(_nonce: u64): <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">native_bridge::OutboundTransfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_bridge_transfer_details_from_nonce">get_bridge_transfer_details_from_nonce</a>(_nonce: u64): <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a> {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id"></a>

## Function `get_inbound_nonce_from_bridge_transfer_id`

Gets inbound <code>nonce</code> from <code>bridge_transfer_id</code>
@param bridge_transfer_id The ID bridge transfer.
@return the nonce
@abort If the nonce is not found in the smart table.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id">get_inbound_nonce_from_bridge_transfer_id</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id">get_inbound_nonce_from_bridge_transfer_id</a>(_bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_increment_and_get_nonce"></a>

## Function `increment_and_get_nonce`

Increment and get the current nonce


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>(): u64 {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.

@param aptos_framework The signer representing the Aptos framework.
@param burn_cap The burn capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _burn_cap: BurnCapability&lt;AptosCoin&gt;) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Stores the mint capability for AptosCoin.

@param aptos_framework The signer representing the Aptos framework.
@param mint_cap The mint capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _mint_cap: MintCapability&lt;AptosCoin&gt;) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_mint_to"></a>

## Function `mint_to`

Mints a specified amount of AptosCoin to a recipient's address.

@param core_resource The signer representing the core resource account.
@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint_to">mint_to</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _recipient: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint_to">mint_to</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _recipient: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_mint"></a>

## Function `mint`

Mints a specified amount of AptosCoin to a recipient's address.

@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.
@abort If the mint capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint">mint</a>(_recipient: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint">mint</a>(_recipient: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_mint_internal"></a>

## Function `mint_internal`

Mints a specified amount of AptosCoin to a recipient's address.

@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint_internal">mint_internal</a>(_recipient: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint_internal">mint_internal</a>(_recipient: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_burn_from"></a>

## Function `burn_from`

Burns a specified amount of AptosCoin from an address.

@param core_resource The signer representing the core resource account.
@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.
@abort If the burn capability is not available.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn_from">burn_from</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _from: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn_from">burn_from</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _from: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_burn"></a>

## Function `burn`

Burns a specified amount of AptosCoin from an address.

@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.
@abort If the burn capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn">burn</a>(_from: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn">burn</a>(_from: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_burn_internal"></a>

## Function `burn_internal`

Burns a specified amount of AptosCoin from an address.

@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn_internal">burn_internal</a>(_from: <b>address</b>, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn_internal">burn_internal</a>(_from: <b>address</b>, _amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_initiate_bridge_transfer"></a>

## Function `initiate_bridge_transfer`

Initiate a bridge transfer of MOVE from Movement to Ethereum
Anyone can initiate a bridge transfer from the source chain
The amount is burnt from the initiator and the module-level nonce is incremented
@param initiator The initiator's Ethereum address as a vector of bytes.
@param recipient The address of the recipient on the Aptos blockchain.
@param amount The amount of assets to be locked.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(_initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(
    _initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _amount: u64
) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_complete_bridge_transfer"></a>

## Function `complete_bridge_transfer`

Completes a bridge transfer on the destination chain.

@param caller The signer representing the bridge relayer.
@param initiator The initiator's Ethereum address as a vector of bytes.
@param bridge_transfer_id The unique identifier for the bridge transfer.
@param recipient The address of the recipient on the Aptos blockchain.
@param amount The amount of assets to be locked.
@param nonce The unique nonce for the transfer.
@abort If the caller is not the bridge relayer or the transfer has already been processed.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, _recipient: <b>address</b>, _amount: u64, _nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(
    _caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    _recipient: <b>address</b>,
    _amount: u64,
    _nonce: u64
)  {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_charge_bridge_fee"></a>

## Function `charge_bridge_fee`

Charge bridge fee to the initiate bridge transfer.

@param initiator The signer representing the initiator.
@param amount The amount to be charged.
@return The new amount after deducting the bridge fee.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_charge_bridge_fee">charge_bridge_fee</a>(_amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_charge_bridge_fee">charge_bridge_fee</a>(_amount: u64): u64 {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_bridge_relayer"></a>

## Function `update_bridge_relayer`

Updates the bridge relayer, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_relayer The new address to be set as the bridge relayer.
@abort If the current relayer is the same as the new relayer.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_relayer">update_bridge_relayer</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_relayer: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_relayer">update_bridge_relayer</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_relayer: <b>address</b>) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_bridge_fee"></a>

## Function `update_bridge_fee`

Updates the bridge fee, requiring relayer validation.

@param relayer The signer representing the Relayer.
@param new_bridge_fee The new bridge fee to be set.
@abort If the new bridge fee is the same as the old bridge fee.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_fee">update_bridge_fee</a>(_relayer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_bridge_fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_fee">update_bridge_fee</a>(_relayer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_bridge_fee: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_insurance_fund"></a>

## Function `update_insurance_fund`

Updates the insurance fund, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_insurance_fund The new insurance fund to be set.
@abort If the new insurance fund is the same as the old insurance fund.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_insurance_fund">update_insurance_fund</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_insurance_fund: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_insurance_fund">update_insurance_fund</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_insurance_fund: <b>address</b>) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_insurance_budget_divider"></a>

## Function `update_insurance_budget_divider`

Updates the insurance budget divider, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_insurance_budget_divider The new insurance budget divider to be set.
@abort If the new insurance budget divider is the same as the old insurance budget divider.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_insurance_budget_divider">update_insurance_budget_divider</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_insurance_budget_divider: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_insurance_budget_divider">update_insurance_budget_divider</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_insurance_budget_divider: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_is_caller_relayer"></a>

## Function `assert_is_caller_relayer`

Asserts that the caller is the current bridge relayer.

@param caller The signer whose authority is being checked.
@abort If the caller is not the current bridge relayer.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_is_caller_relayer">assert_is_caller_relayer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_is_caller_relayer">assert_is_caller_relayer</a>(_caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_outbound_rate_limit_budget_not_exceeded"></a>

## Function `assert_outbound_rate_limit_budget_not_exceeded`

Asserts that the rate limit budget is not exceeded.

@param amount The amount to be transferred.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_outbound_rate_limit_budget_not_exceeded">assert_outbound_rate_limit_budget_not_exceeded</a>(_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_outbound_rate_limit_budget_not_exceeded">assert_outbound_rate_limit_budget_not_exceeded</a>(_amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_inbound_rate_limit_budget_not_exceeded"></a>

## Function `assert_inbound_rate_limit_budget_not_exceeded`

Asserts that the rate limit budget is not exceeded.

@param amount The amount to be transferred.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_inbound_rate_limit_budget_not_exceeded">assert_inbound_rate_limit_budget_not_exceeded</a>(_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_inbound_rate_limit_budget_not_exceeded">assert_inbound_rate_limit_budget_not_exceeded</a>(_amount: u64) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>

<a id="0x1_native_bridge_test_normalize_u64_to_32_bytes_helper"></a>

## Function `test_normalize_u64_to_32_bytes_helper`

Test serialization of u64 to 32 bytes


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_test_normalize_u64_to_32_bytes_helper">test_normalize_u64_to_32_bytes_helper</a>(_x: u64, _expected: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_test_normalize_u64_to_32_bytes_helper">test_normalize_u64_to_32_bytes_helper</a>(_x: u64, _expected: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>abort</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
