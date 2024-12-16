
<a id="0x1_native_bridge"></a>

# Module `0x1::native_bridge`



-  [Struct `BridgeConfigRelayerUpdated`](#0x1_native_bridge_BridgeConfigRelayerUpdated)
-  [Struct `BridgeFeeChangedEvent`](#0x1_native_bridge_BridgeFeeChangedEvent)
-  [Struct `BridgeRiskDenominatorChangedEvent`](#0x1_native_bridge_BridgeRiskDenominatorChangedEvent)
-  [Struct `BridgeInsuranceFundChangedEvent`](#0x1_native_bridge_BridgeInsuranceFundChangedEvent)
-  [Struct `BridgeTransferInitiatedEvent`](#0x1_native_bridge_BridgeTransferInitiatedEvent)
-  [Struct `BridgeTransferCompletedEvent`](#0x1_native_bridge_BridgeTransferCompletedEvent)
-  [Resource `BridgeEvents`](#0x1_native_bridge_BridgeEvents)
-  [Resource `AptosCoinBurnCapability`](#0x1_native_bridge_AptosCoinBurnCapability)
-  [Resource `AptosCoinMintCapability`](#0x1_native_bridge_AptosCoinMintCapability)
-  [Resource `AptosFABurnCapabilities`](#0x1_native_bridge_AptosFABurnCapabilities)
-  [Resource `AptosFAMintCapabilities`](#0x1_native_bridge_AptosFAMintCapabilities)
-  [Resource `Nonce`](#0x1_native_bridge_Nonce)
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
-  [Function `risk_denominator`](#0x1_native_bridge_risk_denominator)
-  [Function `bridge_fee`](#0x1_native_bridge_bridge_fee)
-  [Function `get_bridge_transfer_details_from_nonce`](#0x1_native_bridge_get_bridge_transfer_details_from_nonce)
-  [Function `get_inbound_nonce_from_bridge_transfer_id`](#0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id)
-  [Function `increment_and_get_nonce`](#0x1_native_bridge_increment_and_get_nonce)
-  [Function `store_aptos_coin_burn_cap`](#0x1_native_bridge_store_aptos_coin_burn_cap)
-  [Function `store_aptos_coin_mint_cap`](#0x1_native_bridge_store_aptos_coin_mint_cap)
-  [Function `mint`](#0x1_native_bridge_mint)
-  [Function `burn`](#0x1_native_bridge_burn)
-  [Function `initiate_bridge_transfer`](#0x1_native_bridge_initiate_bridge_transfer)
-  [Function `complete_bridge_transfer`](#0x1_native_bridge_complete_bridge_transfer)
-  [Function `charge_bridge_fee`](#0x1_native_bridge_charge_bridge_fee)
-  [Function `update_bridge_relayer`](#0x1_native_bridge_update_bridge_relayer)
-  [Function `update_bridge_fee`](#0x1_native_bridge_update_bridge_fee)
-  [Function `update_insurance_fund`](#0x1_native_bridge_update_insurance_fund)
-  [Function `update_risk_denominator`](#0x1_native_bridge_update_risk_denominator)
-  [Function `assert_is_caller_relayer`](#0x1_native_bridge_assert_is_caller_relayer)
-  [Function `assert_rate_limit_budget_not_exceeded`](#0x1_native_bridge_assert_rate_limit_budget_not_exceeded)
-  [Function `test_normalize_u64_to_32_bytes_helper`](#0x1_native_bridge_test_normalize_u64_to_32_bytes_helper)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="ethereum.md#0x1_ethereum">0x1::ethereum</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
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

<a id="0x1_native_bridge_BridgeRiskDenominatorChangedEvent"></a>

## Struct `BridgeRiskDenominatorChangedEvent`

An event triggered upon change of risk denominator


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_bridge.md#0x1_native_bridge_BridgeRiskDenominatorChangedEvent">BridgeRiskDenominatorChangedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_risk_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_risk_denominator: u64</code>
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
<code>risk_denominator: u64</code>
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


<a id="0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>: u64 = 8;
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



<a id="0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>: u64 = 2;
</code></pre>



<a id="0x1_native_bridge_EINVALID_NONCE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EINVALID_NONCE">EINVALID_NONCE</a>: u64 = 4;
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



<a id="0x1_native_bridge_ESAME_VALUE"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ESAME_VALUE">ESAME_VALUE</a>: u64 = 3;
</code></pre>



<a id="0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED">ETRANSFER_ALREADY_PROCESSED</a>: u64 = 1;
</code></pre>



<a id="0x1_native_bridge_EZERO_AMOUNT"></a>



<pre><code><b>const</b> <a href="native_bridge.md#0x1_native_bridge_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 7;
</code></pre>



<a id="0x1_native_bridge_initialize"></a>

## Function `initialize`

Initializes the module and stores the <code>EventHandle</code>s in the resource.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> bridge_config = <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
        bridge_relayer: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework),
        insurance_fund: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework),
        risk_denominator: 4,
        bridge_fee: 40_000_000_000,
    };
    <b>move_to</b>(aptos_framework, bridge_config);

    // Ensure the nonce is not already initialized
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)),
        2
    );

    // Create the <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> resource <b>with</b> an initial value of 0
    <b>move_to</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> {
        value: 0
    });

    // Create the InboundRateLimitBudget resource


    <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a> {
        bridge_transfer_initiated_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a>&gt;(aptos_framework),
        bridge_transfer_completed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a>&gt;(aptos_framework),
    });
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> inbound_rate_limit_budget = <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, u64&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, inbound_rate_limit_budget);

    <b>let</b> nonces_to_details = <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a>&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, nonces_to_details);

    <b>let</b> ids_to_inbound_nonces = <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt; {
        inner: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };

    <b>move_to</b>(aptos_framework, ids_to_inbound_nonces);
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(value: &u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(value: &u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> r = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&(*value <b>as</b> u256));
    // BCS returns the bytes in reverse order, so we reverse the result.
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> r);
    r
}
</code></pre>



</details>

<a id="0x1_native_bridge_is_inbound_nonce_set"></a>

## Function `is_inbound_nonce_set`

Checks if a bridge transfer ID is associated with an inbound nonce.
@param bridge_transfer_id The bridge transfer ID.
@return <code><b>true</b></code> if the ID is associated with an existing inbound nonce, <code><b>false</b></code> otherwise.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_is_inbound_nonce_set">is_inbound_nonce_set</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_is_inbound_nonce_set">is_inbound_nonce_set</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id)
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_create_details">create_details</a>(initiator: <b>address</b>, recipient: <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, amount: u64, nonce: u64): <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">native_bridge::OutboundTransfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_create_details">create_details</a>(initiator: <b>address</b>, recipient: EthereumAddress, amount: u64, nonce: u64)
    : <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a> {
    <b>assert</b>!(amount &gt; 0, <a href="native_bridge.md#0x1_native_bridge_EZERO_AMOUNT">EZERO_AMOUNT</a>);

    // Create a bridge transfer ID algorithmically
    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&initiator));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&recipient));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amount));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&nonce));
    <b>let</b> bridge_transfer_id = keccak256(combined_bytes);

    <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a> {
        bridge_transfer_id,
        initiator,
        recipient,
        amount,
    }
}
</code></pre>



</details>

<a id="0x1_native_bridge_add"></a>

## Function `add`

Record details of an initiated transfer for quick lookup of details, mapping bridge transfer ID to transfer details

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_add">add</a>(nonce: u64, details: <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">native_bridge::OutboundTransfer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_add">add</a>(nonce: u64, details: <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a>) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, nonce, details);
}
</code></pre>



</details>

<a id="0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce"></a>

## Function `set_bridge_transfer_id_to_inbound_nonce`

Record details of a completed transfer, mapping bridge transfer ID to inbound nonce

@param bridge_transfer_id Bridge transfer ID.
@param details The bridge transfer details


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, inbound_nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, inbound_nonce: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <a href="native_bridge.md#0x1_native_bridge_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(&bridge_transfer_id);
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id, inbound_nonce);
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_valid_bridge_transfer_id"></a>

## Function `assert_valid_bridge_transfer_id`

Asserts that the bridge transfer ID is valid.

@param bridge_transfer_id The bridge transfer ID to validate.
@abort If the ID is invalid.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_valid_bridge_transfer_id">assert_valid_bridge_transfer_id</a>(bridge_transfer_id: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bridge_transfer_id) == 32, <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>);
}
</code></pre>



</details>

<a id="0x1_native_bridge_bridge_transfer_id"></a>

## Function `bridge_transfer_id`

Generates a unique outbound bridge transfer ID based on transfer details and nonce.

@param details The bridge transfer details.
@return The generated bridge transfer ID.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_transfer_id">bridge_transfer_id</a>(initiator: <b>address</b>, recipient: <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>, amount: u64, nonce: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_transfer_id">bridge_transfer_id</a>(initiator: <b>address</b>, recipient: EthereumAddress, amount: u64, nonce: u64) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    // Serialize each param
    <b>let</b> initiator_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>&lt;<b>address</b>&gt;(&initiator);
    <b>let</b> recipient_bytes = <a href="ethereum.md#0x1_ethereum_get_inner_ethereum_address">ethereum::get_inner_ethereum_address</a>(recipient);
    <b>let</b> amount_bytes = <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(&amount);
    <b>let</b> nonce_bytes = <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(&nonce);
    //Contatenate then <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> and <b>return</b> bridge transfer ID
    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, initiator_bytes);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, recipient_bytes);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, amount_bytes);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, nonce_bytes);
    keccak256(combined_bytes)
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


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_relayer">bridge_relayer</a>(): <b>address</b> <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer
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


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_insurance_fund">insurance_fund</a>(): <b>address</b> <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).insurance_fund
}
</code></pre>



</details>

<a id="0x1_native_bridge_risk_denominator"></a>

## Function `risk_denominator`

Retrieves the current risk denominator.

@return The current risk denominator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_risk_denominator">risk_denominator</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_risk_denominator">risk_denominator</a>(): u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).risk_denominator
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


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_bridge_fee">bridge_fee</a>(): u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_fee
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
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_bridge_transfer_details_from_nonce">get_bridge_transfer_details_from_nonce</a>(nonce: u64): <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">native_bridge::OutboundTransfer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_bridge_transfer_details_from_nonce">get_bridge_transfer_details_from_nonce</a>(nonce: u64): <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a> <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, <a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a>&gt;&gt;(@aptos_framework);

    // Check <b>if</b> the nonce <b>exists</b> in the <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, nonce), <a href="native_bridge.md#0x1_native_bridge_ENONCE_NOT_FOUND">ENONCE_NOT_FOUND</a>);

    // If it <b>exists</b>, <b>return</b> the associated `<a href="native_bridge.md#0x1_native_bridge_OutboundTransfer">OutboundTransfer</a>` details
    *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, nonce)
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
<b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id">get_inbound_nonce_from_bridge_transfer_id</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_get_inbound_nonce_from_bridge_transfer_id">get_inbound_nonce_from_bridge_transfer_id</a>(bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a> {
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u64&gt;&gt;(@aptos_framework);

     // Check <b>if</b> the nonce <b>exists</b> in the <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id), <a href="native_bridge.md#0x1_native_bridge_ENONCE_NOT_FOUND">ENONCE_NOT_FOUND</a>);

    // If it <b>exists</b>, <b>return</b> the associated nonce
    *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, bridge_transfer_id)
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


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>(): u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a> {
    <b>let</b> nonce_ref = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>&gt;(@aptos_framework);
    nonce_ref.value = nonce_ref.value + 1;
    nonce_ref.value
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.

@param aptos_framework The signer representing the Aptos framework.
@param burn_cap The burn capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: BurnCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
        <b>let</b> burn_ref = <a href="coin.md#0x1_coin_convert_and_take_paired_burn_ref">coin::convert_and_take_paired_burn_ref</a>(burn_cap);
        <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_AptosFABurnCapabilities">AptosFABurnCapabilities</a> { burn_ref });
    } <b>else</b> {
        <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> { burn_cap })
    }
}
</code></pre>



</details>

<a id="0x1_native_bridge_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Stores the mint capability for AptosCoin.

@param aptos_framework The signer representing the Aptos framework.
@param mint_cap The mint capability for AptosCoin.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_store_aptos_coin_mint_cap">store_aptos_coin_mint_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: MintCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>(aptos_framework, <a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> { mint_cap })
}
</code></pre>



</details>

<a id="0x1_native_bridge_mint"></a>

## Function `mint`

Mints a specified amount of AptosCoin to a recipient's address.

@param recipient The address of the recipient to mint coins to.
@param amount The amount of AptosCoin to mint.
@abort If the mint capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint">mint</a>(recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_mint">mint</a>(recipient: <b>address</b>, amount: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(recipient, <a href="coin.md#0x1_coin_mint">coin::mint</a>(
        amount,
        &<b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a>&gt;(@aptos_framework).mint_cap
    ));
}
</code></pre>



</details>

<a id="0x1_native_bridge_burn"></a>

## Function `burn`

Burns a specified amount of AptosCoin from an address.

@param from The address from which to burn AptosCoin.
@param amount The amount of AptosCoin to burn.
@abort If the burn capability is not available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn">burn</a>(from: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_burn">burn</a>(from: <b>address</b>, amount: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_native_bridge_enabled">features::abort_native_bridge_enabled</a>(), <a href="native_bridge.md#0x1_native_bridge_ENATIVE_BRIDGE_NOT_ENABLED">ENATIVE_BRIDGE_NOT_ENABLED</a>);

    <a href="coin.md#0x1_coin_burn_from">coin::burn_from</a>(
        from,
        amount,
        &<b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a>&gt;(@aptos_framework).burn_cap,
    );
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


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_initiate_bridge_transfer">initiate_bridge_transfer</a>(
    initiator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    amount: u64
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>, <a href="native_bridge.md#0x1_native_bridge_Nonce">Nonce</a>, <a href="native_bridge.md#0x1_native_bridge_AptosCoinBurnCapability">AptosCoinBurnCapability</a>, <a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a>, <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>, <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>let</b> initiator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(initiator);
    <b>let</b> ethereum_address = <a href="ethereum.md#0x1_ethereum_ethereum_address_20_bytes">ethereum::ethereum_address_20_bytes</a>(recipient);

    // Ensure the amount is enough for the bridge fee and charge for it
    <b>let</b> new_amount = <a href="native_bridge.md#0x1_native_bridge_charge_bridge_fee">charge_bridge_fee</a>(amount);

    // Increment and retrieve the nonce
    <b>let</b> nonce = <a href="native_bridge.md#0x1_native_bridge_increment_and_get_nonce">increment_and_get_nonce</a>();

    // Create bridge transfer details
    <b>let</b> details = <a href="native_bridge.md#0x1_native_bridge_create_details">create_details</a>(
        initiator_address,
        ethereum_address,
        new_amount,
        nonce
    );

    <b>let</b> bridge_transfer_id = <a href="native_bridge.md#0x1_native_bridge_bridge_transfer_id">bridge_transfer_id</a>(
        initiator_address,
        ethereum_address,
        new_amount,
        nonce
    );

    // Add the transfer details <b>to</b> storage
    <a href="native_bridge.md#0x1_native_bridge_add">add</a>(nonce, details);

    // Burn the amount from the initiator
    <a href="native_bridge.md#0x1_native_bridge_burn">burn</a>(initiator_address, amount);

    <b>let</b> bridge_events = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework);

    // Emit an <a href="event.md#0x1_event">event</a> <b>with</b> nonce
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
         &<b>mut</b> bridge_events.bridge_transfer_initiated_events,
        <a href="native_bridge.md#0x1_native_bridge_BridgeTransferInitiatedEvent">BridgeTransferInitiatedEvent</a> {
            bridge_transfer_id,
            initiator: initiator_address,
            recipient,
            amount: new_amount,
            nonce,
        }
    );
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


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient: <b>address</b>, amount: u64, nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_complete_bridge_transfer">complete_bridge_transfer</a>(
    caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    bridge_transfer_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    initiator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recipient: <b>address</b>,
    amount: u64,
    nonce: u64
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>, <a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a>, <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>, <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    // Ensure the caller is the bridge relayer
    <a href="native_bridge.md#0x1_native_bridge_assert_is_caller_relayer">assert_is_caller_relayer</a>(caller);
    <a href="native_bridge.md#0x1_native_bridge_assert_rate_limit_budget_not_exceeded">assert_rate_limit_budget_not_exceeded</a>(amount);

    // Check <b>if</b> the bridge transfer ID is already associated <b>with</b> an inbound nonce
    <b>let</b> inbound_nonce_exists = <a href="native_bridge.md#0x1_native_bridge_is_inbound_nonce_set">is_inbound_nonce_set</a>(bridge_transfer_id);
    <b>assert</b>!(!inbound_nonce_exists, <a href="native_bridge.md#0x1_native_bridge_ETRANSFER_ALREADY_PROCESSED">ETRANSFER_ALREADY_PROCESSED</a>);
    <b>assert</b>!(nonce &gt; 0, <a href="native_bridge.md#0x1_native_bridge_EINVALID_NONCE">EINVALID_NONCE</a>);

    // Validate the bridge_transfer_id by reconstructing the <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
    <b>let</b> recipient_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&recipient);
    <b>let</b> amount_bytes = <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(&amount);
    <b>let</b> nonce_bytes = <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(&nonce);

    <b>let</b> combined_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, initiator);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, recipient_bytes);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, amount_bytes);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> combined_bytes, nonce_bytes);

    <b>assert</b>!(keccak256(combined_bytes) == bridge_transfer_id, <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_TRANSFER_ID">EINVALID_BRIDGE_TRANSFER_ID</a>);

    // Record the transfer <b>as</b> completed by associating the bridge_transfer_id <b>with</b> the inbound nonce
    <a href="native_bridge.md#0x1_native_bridge_set_bridge_transfer_id_to_inbound_nonce">set_bridge_transfer_id_to_inbound_nonce</a>(bridge_transfer_id, nonce);

    // Mint <b>to</b> the recipient
    <a href="native_bridge.md#0x1_native_bridge_mint">mint</a>(recipient, amount);

    // Emit the <a href="event.md#0x1_event">event</a>
    <b>let</b> bridge_events = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeEvents">BridgeEvents</a>&gt;(@aptos_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> bridge_events.bridge_transfer_completed_events,
        <a href="native_bridge.md#0x1_native_bridge_BridgeTransferCompletedEvent">BridgeTransferCompletedEvent</a> {
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
            nonce,
        },
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_charge_bridge_fee"></a>

## Function `charge_bridge_fee`

Charge bridge fee to the initiate bridge transfer.

@param initiator The signer representing the initiator.
@param amount The amount to be charged.
@return The new amount after deducting the bridge fee.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_charge_bridge_fee">charge_bridge_fee</a>(amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_charge_bridge_fee">charge_bridge_fee</a>(amount: u64) : u64 <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_AptosCoinMintCapability">AptosCoinMintCapability</a>, <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>let</b> bridge_fee = <a href="native_bridge.md#0x1_native_bridge_bridge_fee">bridge_fee</a>();
    <b>let</b> bridge_relayer = <a href="native_bridge.md#0x1_native_bridge_bridge_relayer">bridge_relayer</a>();
    <b>assert</b>!(amount &gt; bridge_fee, <a href="native_bridge.md#0x1_native_bridge_EINVALID_AMOUNT">EINVALID_AMOUNT</a>);
    <b>let</b> new_amount = amount - bridge_fee;
    <a href="native_bridge.md#0x1_native_bridge_mint">mint</a>(bridge_relayer, bridge_fee);
    new_amount
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_bridge_relayer"></a>

## Function `update_bridge_relayer`

Updates the bridge relayer, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_relayer The new address to be set as the bridge relayer.
@abort If the current relayer is the same as the new relayer.


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_relayer">update_bridge_relayer</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_relayer: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_relayer">update_bridge_relayer</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_relayer: <b>address</b>
)   <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_relayer = bridge_config.bridge_relayer;
    <b>assert</b>!(old_relayer != new_relayer, <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_RELAYER">EINVALID_BRIDGE_RELAYER</a>);

    bridge_config.bridge_relayer = new_relayer;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="native_bridge.md#0x1_native_bridge_BridgeConfigRelayerUpdated">BridgeConfigRelayerUpdated</a> {
            old_relayer,
            new_relayer,
        },
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_bridge_fee"></a>

## Function `update_bridge_fee`

Updates the bridge fee, requiring relayer validation.

@param relayer The signer representing the Relayer.
@param new_bridge_fee The new bridge fee to be set.
@abort If the new bridge fee is the same as the old bridge fee.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_fee">update_bridge_fee</a>(relayer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_bridge_fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_bridge_fee">update_bridge_fee</a>(relayer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_bridge_fee: u64
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <a href="native_bridge.md#0x1_native_bridge_assert_is_caller_relayer">assert_is_caller_relayer</a>(relayer);
    <b>let</b> bridge_config = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_bridge_fee = bridge_config.bridge_fee;
    <b>assert</b>!(old_bridge_fee != new_bridge_fee, <a href="native_bridge.md#0x1_native_bridge_ESAME_FEE">ESAME_FEE</a>);
    bridge_config.bridge_fee = new_bridge_fee;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="native_bridge.md#0x1_native_bridge_BridgeFeeChangedEvent">BridgeFeeChangedEvent</a> {
            old_bridge_fee,
            new_bridge_fee,
        },
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_insurance_fund"></a>

## Function `update_insurance_fund`

Updates the insurance fund, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_insurance_fund The new insurance fund to be set.
@abort If the new insurance fund is the same as the old insurance fund.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_insurance_fund">update_insurance_fund</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_insurance_fund: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_insurance_fund">update_insurance_fund</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_insurance_fund: <b>address</b>
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_insurance_fund = bridge_config.insurance_fund;
    <b>assert</b>!(old_insurance_fund != new_insurance_fund, <a href="native_bridge.md#0x1_native_bridge_ESAME_VALUE">ESAME_VALUE</a>);
    bridge_config.insurance_fund = new_insurance_fund;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="native_bridge.md#0x1_native_bridge_BridgeInsuranceFundChangedEvent">BridgeInsuranceFundChangedEvent</a> {
            old_insurance_fund,
            new_insurance_fund,
        },
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_update_risk_denominator"></a>

## Function `update_risk_denominator`

Updates the risk denominator, requiring governance validation.

@param aptos_framework The signer representing the Aptos framework.
@param new_risk_denominator The new risk denominator to be set.
@abort If the new risk denominator is the same as the old risk denominator.


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_risk_denominator">update_risk_denominator</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_risk_denominator: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_update_risk_denominator">update_risk_denominator</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_risk_denominator: u64
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> bridge_config = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_risk_denominator = bridge_config.risk_denominator;
    <b>assert</b>!(old_risk_denominator != new_risk_denominator, <a href="native_bridge.md#0x1_native_bridge_ESAME_VALUE">ESAME_VALUE</a>);
    bridge_config.risk_denominator = new_risk_denominator;

    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="native_bridge.md#0x1_native_bridge_BridgeRiskDenominatorChangedEvent">BridgeRiskDenominatorChangedEvent</a> {
            old_risk_denominator,
            new_risk_denominator,
        },
    );
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_is_caller_relayer"></a>

## Function `assert_is_caller_relayer`

Asserts that the caller is the current bridge relayer.

@param caller The signer whose authority is being checked.
@abort If the caller is not the current bridge relayer.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_is_caller_relayer">assert_is_caller_relayer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_is_caller_relayer">assert_is_caller_relayer</a>(caller: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>assert</b>!(<b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).bridge_relayer == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(caller), <a href="native_bridge.md#0x1_native_bridge_EINVALID_BRIDGE_RELAYER">EINVALID_BRIDGE_RELAYER</a>);
}
</code></pre>



</details>

<a id="0x1_native_bridge_assert_rate_limit_budget_not_exceeded"></a>

## Function `assert_rate_limit_budget_not_exceeded`

Asserts that the rate limit budget is not exceeded.

@param amount The amount to be transferred.


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_rate_limit_budget_not_exceeded">assert_rate_limit_budget_not_exceeded</a>(amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_assert_rate_limit_budget_not_exceeded">assert_rate_limit_budget_not_exceeded</a>(amount: u64) <b>acquires</b> <a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>, <a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a> {
    <b>let</b> insurance_fund = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).insurance_fund;
    <b>let</b> risk_denominator = <b>borrow_global</b>&lt;<a href="native_bridge.md#0x1_native_bridge_BridgeConfig">BridgeConfig</a>&gt;(@aptos_framework).risk_denominator;
    <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <b>borrow_global_mut</b>&lt;<a href="native_bridge.md#0x1_native_bridge_SmartTableWrapper">SmartTableWrapper</a>&lt;u64, u64&gt;&gt;(@aptos_framework);

    <b>let</b> day = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() / 86400;
    <b>let</b> current_budget = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, day, 0);
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_upsert">smart_table::upsert</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, day, *current_budget + amount);
    <b>let</b> rate_limit = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(insurance_fund) / risk_denominator;
    <b>assert</b>!(*<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.inner, day) &lt; rate_limit, <a href="native_bridge.md#0x1_native_bridge_ERATE_LIMIT_EXCEEDED">ERATE_LIMIT_EXCEEDED</a>);
}
</code></pre>



</details>

<a id="0x1_native_bridge_test_normalize_u64_to_32_bytes_helper"></a>

## Function `test_normalize_u64_to_32_bytes_helper`

Test serialization of u64 to 32 bytes


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_test_normalize_u64_to_32_bytes_helper">test_normalize_u64_to_32_bytes_helper</a>(x: u64, expected: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="native_bridge.md#0x1_native_bridge_test_normalize_u64_to_32_bytes_helper">test_normalize_u64_to_32_bytes_helper</a>(x: u64, expected: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> r = <a href="native_bridge.md#0x1_native_bridge_normalize_u64_to_32_bytes">normalize_u64_to_32_bytes</a>(&x);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&r) == 32, 0);
    <b>assert</b>!(r == expected, 0);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
