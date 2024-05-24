
<a id="0x1_coin"></a>

# Module `0x1::coin`

This module provides the foundation for typesafe Coins.


-  [Struct `Coin`](#0x1_coin_Coin)
-  [Struct `AggregatableCoin`](#0x1_coin_AggregatableCoin)
-  [Resource `CoinStore`](#0x1_coin_CoinStore)
-  [Resource `SupplyConfig`](#0x1_coin_SupplyConfig)
-  [Resource `CoinInfo`](#0x1_coin_CoinInfo)
-  [Struct `DepositEvent`](#0x1_coin_DepositEvent)
-  [Struct `Deposit`](#0x1_coin_Deposit)
-  [Struct `WithdrawEvent`](#0x1_coin_WithdrawEvent)
-  [Struct `Withdraw`](#0x1_coin_Withdraw)
-  [Struct `CoinEventHandleDeletion`](#0x1_coin_CoinEventHandleDeletion)
-  [Struct `PairCreation`](#0x1_coin_PairCreation)
-  [Resource `MigrationFlag`](#0x1_coin_MigrationFlag)
-  [Struct `MintCapability`](#0x1_coin_MintCapability)
-  [Struct `FreezeCapability`](#0x1_coin_FreezeCapability)
-  [Struct `BurnCapability`](#0x1_coin_BurnCapability)
-  [Resource `CoinConversionMap`](#0x1_coin_CoinConversionMap)
-  [Resource `PairedCoinType`](#0x1_coin_PairedCoinType)
-  [Resource `PairedFungibleAssetRefs`](#0x1_coin_PairedFungibleAssetRefs)
-  [Struct `MintRefReceipt`](#0x1_coin_MintRefReceipt)
-  [Struct `TransferRefReceipt`](#0x1_coin_TransferRefReceipt)
-  [Struct `BurnRefReceipt`](#0x1_coin_BurnRefReceipt)
-  [Resource `Ghost$supply`](#0x1_coin_Ghost$supply)
-  [Resource `Ghost$aggregate_supply`](#0x1_coin_Ghost$aggregate_supply)
-  [Constants](#@Constants_0)
-  [Function `paired_metadata`](#0x1_coin_paired_metadata)
-  [Function `create_coin_conversion_map`](#0x1_coin_create_coin_conversion_map)
-  [Function `create_pairing`](#0x1_coin_create_pairing)
-  [Function `is_apt`](#0x1_coin_is_apt)
-  [Function `create_and_return_paired_metadata_if_not_exist`](#0x1_coin_create_and_return_paired_metadata_if_not_exist)
-  [Function `ensure_paired_metadata`](#0x1_coin_ensure_paired_metadata)
-  [Function `paired_coin`](#0x1_coin_paired_coin)
-  [Function `coin_to_fungible_asset`](#0x1_coin_coin_to_fungible_asset)
-  [Function `fungible_asset_to_coin`](#0x1_coin_fungible_asset_to_coin)
-  [Function `assert_paired_metadata_exists`](#0x1_coin_assert_paired_metadata_exists)
-  [Function `paired_mint_ref_exists`](#0x1_coin_paired_mint_ref_exists)
-  [Function `get_paired_mint_ref`](#0x1_coin_get_paired_mint_ref)
-  [Function `return_paired_mint_ref`](#0x1_coin_return_paired_mint_ref)
-  [Function `paired_transfer_ref_exists`](#0x1_coin_paired_transfer_ref_exists)
-  [Function `get_paired_transfer_ref`](#0x1_coin_get_paired_transfer_ref)
-  [Function `return_paired_transfer_ref`](#0x1_coin_return_paired_transfer_ref)
-  [Function `paired_burn_ref_exists`](#0x1_coin_paired_burn_ref_exists)
-  [Function `get_paired_burn_ref`](#0x1_coin_get_paired_burn_ref)
-  [Function `return_paired_burn_ref`](#0x1_coin_return_paired_burn_ref)
-  [Function `borrow_paired_burn_ref`](#0x1_coin_borrow_paired_burn_ref)
-  [Function `initialize_supply_config`](#0x1_coin_initialize_supply_config)
-  [Function `allow_supply_upgrades`](#0x1_coin_allow_supply_upgrades)
-  [Function `initialize_aggregatable_coin`](#0x1_coin_initialize_aggregatable_coin)
-  [Function `is_aggregatable_coin_zero`](#0x1_coin_is_aggregatable_coin_zero)
-  [Function `drain_aggregatable_coin`](#0x1_coin_drain_aggregatable_coin)
-  [Function `merge_aggregatable_coin`](#0x1_coin_merge_aggregatable_coin)
-  [Function `collect_into_aggregatable_coin`](#0x1_coin_collect_into_aggregatable_coin)
-  [Function `calculate_amount_to_withdraw`](#0x1_coin_calculate_amount_to_withdraw)
-  [Function `maybe_convert_to_fungible_store`](#0x1_coin_maybe_convert_to_fungible_store)
-  [Function `migrate_to_fungible_store`](#0x1_coin_migrate_to_fungible_store)
-  [Function `coin_address`](#0x1_coin_coin_address)
-  [Function `balance`](#0x1_coin_balance)
-  [Function `is_balance_at_least`](#0x1_coin_is_balance_at_least)
-  [Function `coin_balance`](#0x1_coin_coin_balance)
-  [Function `is_coin_initialized`](#0x1_coin_is_coin_initialized)
-  [Function `is_coin_store_frozen`](#0x1_coin_is_coin_store_frozen)
-  [Function `is_account_registered`](#0x1_coin_is_account_registered)
-  [Function `name`](#0x1_coin_name)
-  [Function `symbol`](#0x1_coin_symbol)
-  [Function `decimals`](#0x1_coin_decimals)
-  [Function `supply`](#0x1_coin_supply)
-  [Function `coin_supply`](#0x1_coin_coin_supply)
-  [Function `burn`](#0x1_coin_burn)
-  [Function `burn_from`](#0x1_coin_burn_from)
-  [Function `deposit`](#0x1_coin_deposit)
-  [Function `migrated_primary_fungible_store_exists`](#0x1_coin_migrated_primary_fungible_store_exists)
-  [Function `force_deposit`](#0x1_coin_force_deposit)
-  [Function `destroy_zero`](#0x1_coin_destroy_zero)
-  [Function `extract`](#0x1_coin_extract)
-  [Function `extract_all`](#0x1_coin_extract_all)
-  [Function `freeze_coin_store`](#0x1_coin_freeze_coin_store)
-  [Function `unfreeze_coin_store`](#0x1_coin_unfreeze_coin_store)
-  [Function `upgrade_supply`](#0x1_coin_upgrade_supply)
-  [Function `initialize`](#0x1_coin_initialize)
-  [Function `initialize_with_parallelizable_supply`](#0x1_coin_initialize_with_parallelizable_supply)
-  [Function `initialize_internal`](#0x1_coin_initialize_internal)
-  [Function `merge`](#0x1_coin_merge)
-  [Function `mint`](#0x1_coin_mint)
-  [Function `register`](#0x1_coin_register)
-  [Function `transfer`](#0x1_coin_transfer)
-  [Function `value`](#0x1_coin_value)
-  [Function `withdraw`](#0x1_coin_withdraw)
-  [Function `zero`](#0x1_coin_zero)
-  [Function `destroy_freeze_cap`](#0x1_coin_destroy_freeze_cap)
-  [Function `destroy_mint_cap`](#0x1_coin_destroy_mint_cap)
-  [Function `destroy_burn_cap`](#0x1_coin_destroy_burn_cap)
-  [Function `mint_internal`](#0x1_coin_mint_internal)
-  [Function `burn_internal`](#0x1_coin_burn_internal)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Struct `AggregatableCoin`](#@Specification_1_AggregatableCoin)
    -  [Function `coin_to_fungible_asset`](#@Specification_1_coin_to_fungible_asset)
    -  [Function `fungible_asset_to_coin`](#@Specification_1_fungible_asset_to_coin)
    -  [Function `initialize_supply_config`](#@Specification_1_initialize_supply_config)
    -  [Function `allow_supply_upgrades`](#@Specification_1_allow_supply_upgrades)
    -  [Function `initialize_aggregatable_coin`](#@Specification_1_initialize_aggregatable_coin)
    -  [Function `is_aggregatable_coin_zero`](#@Specification_1_is_aggregatable_coin_zero)
    -  [Function `drain_aggregatable_coin`](#@Specification_1_drain_aggregatable_coin)
    -  [Function `merge_aggregatable_coin`](#@Specification_1_merge_aggregatable_coin)
    -  [Function `collect_into_aggregatable_coin`](#@Specification_1_collect_into_aggregatable_coin)
    -  [Function `maybe_convert_to_fungible_store`](#@Specification_1_maybe_convert_to_fungible_store)
    -  [Function `coin_address`](#@Specification_1_coin_address)
    -  [Function `balance`](#@Specification_1_balance)
    -  [Function `is_coin_initialized`](#@Specification_1_is_coin_initialized)
    -  [Function `is_account_registered`](#@Specification_1_is_account_registered)
    -  [Function `name`](#@Specification_1_name)
    -  [Function `symbol`](#@Specification_1_symbol)
    -  [Function `decimals`](#@Specification_1_decimals)
    -  [Function `supply`](#@Specification_1_supply)
    -  [Function `coin_supply`](#@Specification_1_coin_supply)
    -  [Function `burn`](#@Specification_1_burn)
    -  [Function `burn_from`](#@Specification_1_burn_from)
    -  [Function `deposit`](#@Specification_1_deposit)
    -  [Function `force_deposit`](#@Specification_1_force_deposit)
    -  [Function `destroy_zero`](#@Specification_1_destroy_zero)
    -  [Function `extract`](#@Specification_1_extract)
    -  [Function `extract_all`](#@Specification_1_extract_all)
    -  [Function `freeze_coin_store`](#@Specification_1_freeze_coin_store)
    -  [Function `unfreeze_coin_store`](#@Specification_1_unfreeze_coin_store)
    -  [Function `upgrade_supply`](#@Specification_1_upgrade_supply)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initialize_with_parallelizable_supply`](#@Specification_1_initialize_with_parallelizable_supply)
    -  [Function `initialize_internal`](#@Specification_1_initialize_internal)
    -  [Function `merge`](#@Specification_1_merge)
    -  [Function `mint`](#@Specification_1_mint)
    -  [Function `register`](#@Specification_1_register)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `withdraw`](#@Specification_1_withdraw)
    -  [Function `mint_internal`](#@Specification_1_mint_internal)
    -  [Function `burn_internal`](#@Specification_1_burn_internal)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aggregator.md#0x1_aggregator">0x1::aggregator</a>;<br /><b>use</b> <a href="aggregator_factory.md#0x1_aggregator_factory">0x1::aggregator_factory</a>;<br /><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;<br /><b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;<br /><b>use</b> <a href="object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="optional_aggregator.md#0x1_optional_aggregator">0x1::optional_aggregator</a>;<br /><b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;<br /></code></pre>



<a id="0x1_coin_Coin"></a>

## Struct `Coin`

Core data structures
Main structure representing a coin/token in an account&apos;s custody.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>
 Amount of coin this address has.
</dd>
</dl>


</details>

<a id="0x1_coin_AggregatableCoin"></a>

## Struct `AggregatableCoin`

Represents a coin with aggregator as its value. This allows to update
the coin in every transaction avoiding read&#45;modify&#45;write conflicts. Only
used for gas fees distribution by Aptos Framework (0x1).


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt; <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a></code>
</dt>
<dd>
 Amount of aggregatable coin this address has.
</dd>
</dl>


</details>

<a id="0x1_coin_CoinStore"></a>

## Resource `CoinStore`

A holder of a specific coin types and associated event handles.
These are kept in a single resource to ensure locality of data.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt; <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>frozen: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="coin.md#0x1_coin_DepositEvent">coin::DepositEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="coin.md#0x1_coin_WithdrawEvent">coin::WithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_SupplyConfig"></a>

## Resource `SupplyConfig`

Configuration that controls the behavior of total coin supply. If the field
is set, coin creators are allowed to upgrade to parallelizable implementations.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allow_upgrades: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_CoinInfo"></a>

## Resource `CoinInfo`

Information about a specific coin type. Stored on the creator of the coin&apos;s account.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt; <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Symbol of the coin, usually a shorter version of the name.
 For example, Singapore Dollar is SGD.
</dd>
<dt>
<code>decimals: u8</code>
</dt>
<dd>
 Number of decimals used to get its user representation.
 For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
 be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).
</dd>
<dt>
<code><a href="coin.md#0x1_coin_supply">supply</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>&gt;</code>
</dt>
<dd>
 Amount of this coin type in existence.
</dd>
</dl>


</details>

<a id="0x1_coin_DepositEvent"></a>

## Struct `DepositEvent`

Event emitted when some amount of a coin is deposited into an account.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_DepositEvent">DepositEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_Deposit"></a>

## Struct `Deposit`

Module event emitted when some amount of a coin is deposited into an account.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="coin.md#0x1_coin_Deposit">Deposit</a>&lt;CoinType&gt; <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
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

<a id="0x1_coin_WithdrawEvent"></a>

## Struct `WithdrawEvent`

Event emitted when some amount of a coin is withdrawn from an account.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_Withdraw"></a>

## Struct `Withdraw`

Module event emitted when some amount of a coin is withdrawn from an account.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="coin.md#0x1_coin_Withdraw">Withdraw</a>&lt;CoinType&gt; <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
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

<a id="0x1_coin_CoinEventHandleDeletion"></a>

## Struct `CoinEventHandleDeletion`

Module event emitted when the event handles related to coin store is deleted.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="coin.md#0x1_coin_CoinEventHandleDeletion">CoinEventHandleDeletion</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>event_handle_creation_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>deleted_deposit_event_handle_creation_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>deleted_withdraw_event_handle_creation_number: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_PairCreation"></a>

## Struct `PairCreation`

Module event emitted when a new pair of coin and fungible asset is created.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="coin.md#0x1_coin_PairCreation">PairCreation</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>fungible_asset_metadata_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_MigrationFlag"></a>

## Resource `MigrationFlag`

The flag the existence of which indicates the primary fungible store is created by the migration from CoinStore.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="coin.md#0x1_coin_MigrationFlag">MigrationFlag</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_coin_MintCapability"></a>

## Struct `MintCapability`

Capability required to mint coins.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt; <b>has</b> <b>copy</b>, store<br /></code></pre>



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

<a id="0x1_coin_FreezeCapability"></a>

## Struct `FreezeCapability`

Capability required to freeze a coin store.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt; <b>has</b> <b>copy</b>, store<br /></code></pre>



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

<a id="0x1_coin_BurnCapability"></a>

## Struct `BurnCapability`

Capability required to burn coins.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt; <b>has</b> <b>copy</b>, store<br /></code></pre>



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

<a id="0x1_coin_CoinConversionMap"></a>

## Resource `CoinConversionMap`

The mapping between coin and fungible asset.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin_to_fungible_asset_map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a>, <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_PairedCoinType"></a>

## Resource `PairedCoinType`

The paired coin type info stored in fungible asset metadata object.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_PairedFungibleAssetRefs"></a>

## Resource `PairedFungibleAssetRefs`

The refs of the paired fungible asset.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_ref_opt: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transfer_ref_opt: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_ref_opt: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_MintRefReceipt"></a>

## Struct `MintRefReceipt`

The hot potato receipt for flash borrowing MintRef.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_MintRefReceipt">MintRefReceipt</a><br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_TransferRefReceipt"></a>

## Struct `TransferRefReceipt`

The hot potato receipt for flash borrowing TransferRef.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_TransferRefReceipt">TransferRefReceipt</a><br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_BurnRefReceipt"></a>

## Struct `BurnRefReceipt`

The hot potato receipt for flash borrowing BurnRef.


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_BurnRefReceipt">BurnRefReceipt</a><br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_Ghost$supply"></a>

## Resource `Ghost$supply`



<pre><code><b>struct</b> Ghost$<a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: num</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_Ghost$aggregate_supply"></a>

## Resource `Ghost$aggregate_supply`



<pre><code><b>struct</b> Ghost$<a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt; <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: num</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_coin_MAX_U64"></a>

Maximum possible aggregatable coin value.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>: u128 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_coin_MAX_U128"></a>

Maximum possible coin supply.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_MAX_U128">MAX_U128</a>: u128 &#61; 340282366920938463463374607431768211455;<br /></code></pre>



<a id="0x1_coin_EINSUFFICIENT_BALANCE"></a>

Not enough coins to complete transaction


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_coin_EAGGREGATABLE_COIN_VALUE_TOO_LARGE"></a>

The value of aggregatable coin used for transaction fees redistribution does not fit in u64.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EAGGREGATABLE_COIN_VALUE_TOO_LARGE">EAGGREGATABLE_COIN_VALUE_TOO_LARGE</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_coin_EAPT_PAIRING_IS_NOT_ENABLED"></a>

APT pairing is not eanbled yet.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EAPT_PAIRING_IS_NOT_ENABLED">EAPT_PAIRING_IS_NOT_ENABLED</a>: u64 &#61; 28;<br /></code></pre>



<a id="0x1_coin_EBURN_REF_NOT_FOUND"></a>

The BurnRef does not exist.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EBURN_REF_NOT_FOUND">EBURN_REF_NOT_FOUND</a>: u64 &#61; 25;<br /></code></pre>



<a id="0x1_coin_EBURN_REF_RECEIPT_MISMATCH"></a>

The BurnRefReceipt does not match the BurnRef to be returned.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EBURN_REF_RECEIPT_MISMATCH">EBURN_REF_RECEIPT_MISMATCH</a>: u64 &#61; 24;<br /></code></pre>



<a id="0x1_coin_ECOIN_CONVERSION_MAP_NOT_FOUND"></a>

The coin converison map is not created yet.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_CONVERSION_MAP_NOT_FOUND">ECOIN_CONVERSION_MAP_NOT_FOUND</a>: u64 &#61; 27;<br /></code></pre>



<a id="0x1_coin_ECOIN_INFO_ADDRESS_MISMATCH"></a>

Address of account which is used to initialize a coin <code>CoinType</code> doesn&apos;t match the deployer of module


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_INFO_ADDRESS_MISMATCH">ECOIN_INFO_ADDRESS_MISMATCH</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_coin_ECOIN_INFO_ALREADY_PUBLISHED"></a>

<code>CoinType</code> is already initialized as a coin


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_INFO_ALREADY_PUBLISHED">ECOIN_INFO_ALREADY_PUBLISHED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_coin_ECOIN_INFO_NOT_PUBLISHED"></a>

<code>CoinType</code> hasn&apos;t been initialized as a coin


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_INFO_NOT_PUBLISHED">ECOIN_INFO_NOT_PUBLISHED</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_coin_ECOIN_NAME_TOO_LONG"></a>

Name of the coin is too long


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_NAME_TOO_LONG">ECOIN_NAME_TOO_LONG</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_coin_ECOIN_STORE_ALREADY_PUBLISHED"></a>

Deprecated. Account already has <code><a href="coin.md#0x1_coin_CoinStore">CoinStore</a></code> registered for <code>CoinType</code>


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_STORE_ALREADY_PUBLISHED">ECOIN_STORE_ALREADY_PUBLISHED</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_coin_ECOIN_STORE_NOT_PUBLISHED"></a>

Account hasn&apos;t registered <code><a href="coin.md#0x1_coin_CoinStore">CoinStore</a></code> for <code>CoinType</code>


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_STORE_NOT_PUBLISHED">ECOIN_STORE_NOT_PUBLISHED</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_coin_ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED"></a>

Cannot upgrade the total supply of coins to different implementation.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED">ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_coin_ECOIN_SYMBOL_TOO_LONG"></a>

Symbol of the coin is too long


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_SYMBOL_TOO_LONG">ECOIN_SYMBOL_TOO_LONG</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_coin_ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED"></a>

The feature of migration from coin to fungible asset is not enabled.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED">ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED</a>: u64 &#61; 18;<br /></code></pre>



<a id="0x1_coin_ECOIN_TYPE_MISMATCH"></a>

The coin type from the map does not match the calling function type argument.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ECOIN_TYPE_MISMATCH">ECOIN_TYPE_MISMATCH</a>: u64 &#61; 17;<br /></code></pre>



<a id="0x1_coin_EDESTRUCTION_OF_NONZERO_TOKEN"></a>

Cannot destroy non&#45;zero coins


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EDESTRUCTION_OF_NONZERO_TOKEN">EDESTRUCTION_OF_NONZERO_TOKEN</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_coin_EFROZEN"></a>

CoinStore is frozen. Coins cannot be deposited or withdrawn


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EFROZEN">EFROZEN</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_coin_EMIGRATION_FRAMEWORK_NOT_ENABLED"></a>

The migration process from coin to fungible asset is not enabled yet.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EMIGRATION_FRAMEWORK_NOT_ENABLED">EMIGRATION_FRAMEWORK_NOT_ENABLED</a>: u64 &#61; 26;<br /></code></pre>



<a id="0x1_coin_EMINT_REF_NOT_FOUND"></a>

The MintRef does not exist.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EMINT_REF_NOT_FOUND">EMINT_REF_NOT_FOUND</a>: u64 &#61; 21;<br /></code></pre>



<a id="0x1_coin_EMINT_REF_RECEIPT_MISMATCH"></a>

The MintRefReceipt does not match the MintRef to be returned.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EMINT_REF_RECEIPT_MISMATCH">EMINT_REF_RECEIPT_MISMATCH</a>: u64 &#61; 20;<br /></code></pre>



<a id="0x1_coin_EPAIRED_COIN"></a>

Error regarding paired coin type of the fungible asset metadata.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EPAIRED_COIN">EPAIRED_COIN</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x1_coin_EPAIRED_FUNGIBLE_ASSET"></a>

Error regarding paired fungible asset metadata of a coin type.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET">EPAIRED_FUNGIBLE_ASSET</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND"></a>

PairedFungibleAssetRefs resource does not exist.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>: u64 &#61; 19;<br /></code></pre>



<a id="0x1_coin_ETRANSFER_REF_NOT_FOUND"></a>

The TransferRef does not exist.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ETRANSFER_REF_NOT_FOUND">ETRANSFER_REF_NOT_FOUND</a>: u64 &#61; 23;<br /></code></pre>



<a id="0x1_coin_ETRANSFER_REF_RECEIPT_MISMATCH"></a>

The TransferRefReceipt does not match the TransferRef to be returned.


<pre><code><b>const</b> <a href="coin.md#0x1_coin_ETRANSFER_REF_RECEIPT_MISMATCH">ETRANSFER_REF_RECEIPT_MISMATCH</a>: u64 &#61; 22;<br /></code></pre>



<a id="0x1_coin_MAX_COIN_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="coin.md#0x1_coin_MAX_COIN_NAME_LENGTH">MAX_COIN_NAME_LENGTH</a>: u64 &#61; 32;<br /></code></pre>



<a id="0x1_coin_MAX_COIN_SYMBOL_LENGTH"></a>



<pre><code><b>const</b> <a href="coin.md#0x1_coin_MAX_COIN_SYMBOL_LENGTH">MAX_COIN_SYMBOL_LENGTH</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_coin_paired_metadata"></a>

## Function `paired_metadata`

Get the paired fungible asset metadata object of a coin type. If not exist, return option::none().


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;(): Option&lt;Object&lt;Metadata&gt;&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework) &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_coin_to_fungible_asset_migration_feature_enabled">features::coin_to_fungible_asset_migration_feature_enabled</a>(<br />    )) &#123;<br />        <b>let</b> map &#61; &amp;<b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework).coin_to_fungible_asset_map;<br />        <b>let</b> type &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;();<br />        <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(map, type)) &#123;<br />            <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(&#42;<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(map, type))<br />        &#125;<br />    &#125;;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_create_coin_conversion_map"></a>

## Function `create_coin_conversion_map`



<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_create_coin_conversion_map">create_coin_conversion_map</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_create_coin_conversion_map">create_coin_conversion_map</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework)) &#123;<br />        <b>move_to</b>(aptos_framework, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> &#123;<br />            coin_to_fungible_asset_map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),<br />        &#125;)<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_create_pairing"></a>

## Function `create_pairing`

Create APT pairing by passing <code>AptosCoin</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_create_pairing">create_pairing</a>&lt;CoinType&gt;(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_create_pairing">create_pairing</a>&lt;CoinType&gt;(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <a href="coin.md#0x1_coin_create_and_return_paired_metadata_if_not_exist">create_and_return_paired_metadata_if_not_exist</a>&lt;CoinType&gt;(<b>true</b>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_is_apt"></a>

## Function `is_apt`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_is_apt">is_apt</a>&lt;CoinType&gt;(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_is_apt">is_apt</a>&lt;CoinType&gt;(): bool &#123;<br />    <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;CoinType&gt;() &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">0x1::aptos_coin::AptosCoin</a>&quot;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_create_and_return_paired_metadata_if_not_exist"></a>

## Function `create_and_return_paired_metadata_if_not_exist`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_create_and_return_paired_metadata_if_not_exist">create_and_return_paired_metadata_if_not_exist</a>&lt;CoinType&gt;(allow_apt_creation: bool): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_create_and_return_paired_metadata_if_not_exist">create_and_return_paired_metadata_if_not_exist</a>&lt;CoinType&gt;(allow_apt_creation: bool): Object&lt;Metadata&gt; &#123;<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_coin_to_fungible_asset_migration_feature_enabled">features::coin_to_fungible_asset_migration_feature_enabled</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="coin.md#0x1_coin_EMIGRATION_FRAMEWORK_NOT_ENABLED">EMIGRATION_FRAMEWORK_NOT_ENABLED</a>)<br />    );<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_ECOIN_CONVERSION_MAP_NOT_FOUND">ECOIN_CONVERSION_MAP_NOT_FOUND</a>));<br />    <b>let</b> map &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework);<br />    <b>let</b> type &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;();<br />    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;map.coin_to_fungible_asset_map, type)) &#123;<br />        <b>let</b> is_apt &#61; <a href="coin.md#0x1_coin_is_apt">is_apt</a>&lt;CoinType&gt;();<br />        <b>assert</b>!(!is_apt &#124;&#124; allow_apt_creation, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="coin.md#0x1_coin_EAPT_PAIRING_IS_NOT_ENABLED">EAPT_PAIRING_IS_NOT_ENABLED</a>));<br />        <b>let</b> metadata_object_cref &#61;<br />            <b>if</b> (is_apt) &#123;<br />                <a href="object.md#0x1_object_create_sticky_object_at_address">object::create_sticky_object_at_address</a>(@aptos_framework, @aptos_fungible_asset)<br />            &#125; <b>else</b> &#123;<br />                <a href="object.md#0x1_object_create_named_object">object::create_named_object</a>(<br />                    &amp;<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(@aptos_fungible_asset),<br />                    &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&amp;<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;CoinType&gt;())<br />                )<br />            &#125;;<br />        <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset">primary_fungible_store::create_primary_store_enabled_fungible_asset</a>(<br />            &amp;metadata_object_cref,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_map">option::map</a>(<a href="coin.md#0x1_coin_coin_supply">coin_supply</a>&lt;CoinType&gt;(), &#124;_&#124; <a href="coin.md#0x1_coin_MAX_U128">MAX_U128</a>),<br />            <a href="coin.md#0x1_coin_name">name</a>&lt;CoinType&gt;(),<br />            <a href="coin.md#0x1_coin_symbol">symbol</a>&lt;CoinType&gt;(),<br />            <a href="coin.md#0x1_coin_decimals">decimals</a>&lt;CoinType&gt;(),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;&quot;),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;&quot;),<br />        );<br /><br />        <b>let</b> metadata_object_signer &#61; &amp;<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(&amp;metadata_object_cref);<br />        <b>let</b> type &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;();<br />        <b>move_to</b>(metadata_object_signer, <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> &#123; type &#125;);<br />        <b>let</b> metadata_obj &#61; <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&amp;metadata_object_cref);<br /><br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> map.coin_to_fungible_asset_map, type, metadata_obj);<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="coin.md#0x1_coin_PairCreation">PairCreation</a> &#123;<br />            coin_type: type,<br />            fungible_asset_metadata_address: object_address(&amp;metadata_obj)<br />        &#125;);<br /><br />        // Generates all three refs<br />        <b>let</b> mint_ref &#61; <a href="fungible_asset.md#0x1_fungible_asset_generate_mint_ref">fungible_asset::generate_mint_ref</a>(&amp;metadata_object_cref);<br />        <b>let</b> transfer_ref &#61; <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">fungible_asset::generate_transfer_ref</a>(&amp;metadata_object_cref);<br />        <b>let</b> burn_ref &#61; <a href="fungible_asset.md#0x1_fungible_asset_generate_burn_ref">fungible_asset::generate_burn_ref</a>(&amp;metadata_object_cref);<br />        <b>move_to</b>(metadata_object_signer,<br />            <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />                mint_ref_opt: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mint_ref),<br />                transfer_ref_opt: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(transfer_ref),<br />                burn_ref_opt: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(burn_ref),<br />            &#125;<br />        );<br />    &#125;;<br />    &#42;<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;map.coin_to_fungible_asset_map, type)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_ensure_paired_metadata"></a>

## Function `ensure_paired_metadata`

Get the paired fungible asset metadata object of a coin type, create if not exist.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_ensure_paired_metadata">ensure_paired_metadata</a>&lt;CoinType&gt;(): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_ensure_paired_metadata">ensure_paired_metadata</a>&lt;CoinType&gt;(): Object&lt;Metadata&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <a href="coin.md#0x1_coin_create_and_return_paired_metadata_if_not_exist">create_and_return_paired_metadata_if_not_exist</a>&lt;CoinType&gt;(<b>false</b>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_paired_coin"></a>

## Function `paired_coin`

Get the paired coin type of a fungible asset metadata object.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_coin">paired_coin</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_coin">paired_coin</a>(metadata: Object&lt;Metadata&gt;): Option&lt;TypeInfo&gt; <b>acquires</b> <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> &#123;<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;metadata);<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a>&gt;(metadata_addr)) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a>&gt;(metadata_addr).type)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_coin_to_fungible_asset"></a>

## Function `coin_to_fungible_asset`

Conversion from coin to fungible asset


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_coin_to_fungible_asset">coin_to_fungible_asset</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_coin_to_fungible_asset">coin_to_fungible_asset</a>&lt;CoinType&gt;(<br />    <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;<br />): FungibleAsset <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_ensure_paired_metadata">ensure_paired_metadata</a>&lt;CoinType&gt;();<br />    <b>let</b> amount &#61; <a href="coin.md#0x1_coin_burn_internal">burn_internal</a>(<a href="coin.md#0x1_coin">coin</a>);<br />    <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">fungible_asset::mint_internal</a>(metadata, amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_fungible_asset_to_coin"></a>

## Function `fungible_asset_to_coin`

Conversion from fungible asset to coin. Not public to push the migration to FA.


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_fungible_asset_to_coin">fungible_asset_to_coin</a>&lt;CoinType&gt;(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_fungible_asset_to_coin">fungible_asset_to_coin</a>&lt;CoinType&gt;(<br />    <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: FungibleAsset<br />): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> &#123;<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&amp;<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>));<br />    <b>assert</b>!(<br />        <a href="object.md#0x1_object_object_exists">object::object_exists</a>&lt;<a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a>&gt;(metadata_addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_EPAIRED_COIN">EPAIRED_COIN</a>)<br />    );<br />    <b>let</b> coin_type_info &#61; <b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a>&gt;(metadata_addr).type;<br />    <b>assert</b>!(coin_type_info &#61;&#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_TYPE_MISMATCH">ECOIN_TYPE_MISMATCH</a>));<br />    <b>let</b> amount &#61; <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">fungible_asset::burn_internal</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>);<br />    <a href="coin.md#0x1_coin_mint_internal">mint_internal</a>&lt;CoinType&gt;(amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_assert_paired_metadata_exists"></a>

## Function `assert_paired_metadata_exists`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;(): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;(): Object&lt;Metadata&gt; &#123;<br />    <b>let</b> metadata_opt &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;metadata_opt), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET">EPAIRED_FUNGIBLE_ASSET</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(metadata_opt)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_paired_mint_ref_exists"></a>

## Function `paired_mint_ref_exists`

Check whether <code>MintRef</code> has not been taken.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_mint_ref_exists">paired_mint_ref_exists</a>&lt;CoinType&gt;(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_mint_ref_exists">paired_mint_ref_exists</a>&lt;CoinType&gt;(): bool <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).mint_ref_opt)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_get_paired_mint_ref"></a>

## Function `get_paired_mint_ref`

Get the <code>MintRef</code> of paired fungible asset of a coin type from <code><a href="coin.md#0x1_coin_MintCapability">MintCapability</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_get_paired_mint_ref">get_paired_mint_ref</a>&lt;CoinType&gt;(_: &amp;<a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;): (<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, <a href="coin.md#0x1_coin_MintRefReceipt">coin::MintRefReceipt</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_get_paired_mint_ref">get_paired_mint_ref</a>&lt;CoinType&gt;(<br />    _: &amp;<a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;<br />): (MintRef, <a href="coin.md#0x1_coin_MintRefReceipt">MintRefReceipt</a>) <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <b>let</b> mint_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).mint_ref_opt;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(mint_ref_opt), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_EMINT_REF_NOT_FOUND">EMINT_REF_NOT_FOUND</a>));<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(mint_ref_opt), <a href="coin.md#0x1_coin_MintRefReceipt">MintRefReceipt</a> &#123; metadata &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_return_paired_mint_ref"></a>

## Function `return_paired_mint_ref`

Return the <code>MintRef</code> with the hot potato receipt.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_return_paired_mint_ref">return_paired_mint_ref</a>(mint_ref: <a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, receipt: <a href="coin.md#0x1_coin_MintRefReceipt">coin::MintRefReceipt</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_return_paired_mint_ref">return_paired_mint_ref</a>(mint_ref: MintRef, receipt: <a href="coin.md#0x1_coin_MintRefReceipt">MintRefReceipt</a>) <b>acquires</b> <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin_MintRefReceipt">MintRefReceipt</a> &#123; metadata &#125; &#61; receipt;<br />    <b>assert</b>!(<br />        <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">fungible_asset::mint_ref_metadata</a>(&amp;mint_ref) &#61;&#61; metadata,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_EMINT_REF_RECEIPT_MISMATCH">EMINT_REF_RECEIPT_MISMATCH</a>)<br />    );<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>let</b> mint_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).mint_ref_opt;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(mint_ref_opt, mint_ref);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_paired_transfer_ref_exists"></a>

## Function `paired_transfer_ref_exists`

Check whether <code>TransferRef</code> still exists.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_transfer_ref_exists">paired_transfer_ref_exists</a>&lt;CoinType&gt;(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_transfer_ref_exists">paired_transfer_ref_exists</a>&lt;CoinType&gt;(): bool <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).transfer_ref_opt)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_get_paired_transfer_ref"></a>

## Function `get_paired_transfer_ref`

Get the TransferRef of paired fungible asset of a coin type from <code><a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_get_paired_transfer_ref">get_paired_transfer_ref</a>&lt;CoinType&gt;(_: &amp;<a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;): (<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, <a href="coin.md#0x1_coin_TransferRefReceipt">coin::TransferRefReceipt</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_get_paired_transfer_ref">get_paired_transfer_ref</a>&lt;CoinType&gt;(<br />    _: &amp;<a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;<br />): (TransferRef, <a href="coin.md#0x1_coin_TransferRefReceipt">TransferRefReceipt</a>) <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <b>let</b> transfer_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).transfer_ref_opt;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(transfer_ref_opt), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_ETRANSFER_REF_NOT_FOUND">ETRANSFER_REF_NOT_FOUND</a>));<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(transfer_ref_opt), <a href="coin.md#0x1_coin_TransferRefReceipt">TransferRefReceipt</a> &#123; metadata &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_return_paired_transfer_ref"></a>

## Function `return_paired_transfer_ref`

Return the <code>TransferRef</code> with the hot potato receipt.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_return_paired_transfer_ref">return_paired_transfer_ref</a>(transfer_ref: <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, receipt: <a href="coin.md#0x1_coin_TransferRefReceipt">coin::TransferRefReceipt</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_return_paired_transfer_ref">return_paired_transfer_ref</a>(<br />    transfer_ref: TransferRef,<br />    receipt: <a href="coin.md#0x1_coin_TransferRefReceipt">TransferRefReceipt</a><br />) <b>acquires</b> <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin_TransferRefReceipt">TransferRefReceipt</a> &#123; metadata &#125; &#61; receipt;<br />    <b>assert</b>!(<br />        <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(&amp;transfer_ref) &#61;&#61; metadata,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ETRANSFER_REF_RECEIPT_MISMATCH">ETRANSFER_REF_RECEIPT_MISMATCH</a>)<br />    );<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>let</b> transfer_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).transfer_ref_opt;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(transfer_ref_opt, transfer_ref);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_paired_burn_ref_exists"></a>

## Function `paired_burn_ref_exists`

Check whether <code>BurnRef</code> has not been taken.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_burn_ref_exists">paired_burn_ref_exists</a>&lt;CoinType&gt;(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_paired_burn_ref_exists">paired_burn_ref_exists</a>&lt;CoinType&gt;(): bool <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).burn_ref_opt)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_get_paired_burn_ref"></a>

## Function `get_paired_burn_ref`

Get the <code>BurnRef</code> of paired fungible asset of a coin type from <code><a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_get_paired_burn_ref">get_paired_burn_ref</a>&lt;CoinType&gt;(_: &amp;<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;): (<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, <a href="coin.md#0x1_coin_BurnRefReceipt">coin::BurnRefReceipt</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_get_paired_burn_ref">get_paired_burn_ref</a>&lt;CoinType&gt;(<br />    _: &amp;<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;<br />): (BurnRef, <a href="coin.md#0x1_coin_BurnRefReceipt">BurnRefReceipt</a>) <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <b>let</b> burn_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).burn_ref_opt;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(burn_ref_opt), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_EBURN_REF_NOT_FOUND">EBURN_REF_NOT_FOUND</a>));<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(burn_ref_opt), <a href="coin.md#0x1_coin_BurnRefReceipt">BurnRefReceipt</a> &#123; metadata &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_return_paired_burn_ref"></a>

## Function `return_paired_burn_ref`

Return the <code>BurnRef</code> with the hot potato receipt.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_return_paired_burn_ref">return_paired_burn_ref</a>(burn_ref: <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, receipt: <a href="coin.md#0x1_coin_BurnRefReceipt">coin::BurnRefReceipt</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_return_paired_burn_ref">return_paired_burn_ref</a>(<br />    burn_ref: BurnRef,<br />    receipt: <a href="coin.md#0x1_coin_BurnRefReceipt">BurnRefReceipt</a><br />) <b>acquires</b> <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin_BurnRefReceipt">BurnRefReceipt</a> &#123; metadata &#125; &#61; receipt;<br />    <b>assert</b>!(<br />        <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">fungible_asset::burn_ref_metadata</a>(&amp;burn_ref) &#61;&#61; metadata,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_EBURN_REF_RECEIPT_MISMATCH">EBURN_REF_RECEIPT_MISMATCH</a>)<br />    );<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>let</b> burn_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).burn_ref_opt;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(burn_ref_opt, burn_ref);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_borrow_paired_burn_ref"></a>

## Function `borrow_paired_burn_ref`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_borrow_paired_burn_ref">borrow_paired_burn_ref</a>&lt;CoinType&gt;(_: &amp;<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;): &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_borrow_paired_burn_ref">borrow_paired_burn_ref</a>&lt;CoinType&gt;(<br />    _: &amp;<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;<br />): &amp;BurnRef <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_assert_paired_metadata_exists">assert_paired_metadata_exists</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata_addr &#61; object_address(&amp;metadata);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="coin.md#0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND">EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND</a>));<br />    <b>let</b> burn_ref_opt &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a>&gt;(metadata_addr).burn_ref_opt;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(burn_ref_opt), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_EBURN_REF_NOT_FOUND">EBURN_REF_NOT_FOUND</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(burn_ref_opt)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_initialize_supply_config"></a>

## Function `initialize_supply_config`

Publishes supply configuration. Initially, upgrading is not allowed.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_supply_config">initialize_supply_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_supply_config">initialize_supply_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>move_to</b>(aptos_framework, <a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a> &#123; allow_upgrades: <b>false</b> &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_allow_supply_upgrades"></a>

## Function `allow_supply_upgrades`

This should be called by on&#45;chain governance to update the config and allow
or disallow upgradability of total supply.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_allow_supply_upgrades">allow_supply_upgrades</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allowed: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_allow_supply_upgrades">allow_supply_upgrades</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allowed: bool) <b>acquires</b> <a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>let</b> allow_upgrades &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(@aptos_framework).allow_upgrades;<br />    &#42;allow_upgrades &#61; allowed;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_initialize_aggregatable_coin"></a>

## Function `initialize_aggregatable_coin`

Creates a new aggregatable coin with value overflowing on <code>limit</code>. Note that this function can
only be called by Aptos Framework (0x1) account for now because of <code>create_aggregator</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_aggregatable_coin">initialize_aggregatable_coin</a>&lt;CoinType&gt;(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_aggregatable_coin">initialize_aggregatable_coin</a>&lt;CoinType&gt;(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt; &#123;<br />    <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> &#61; <a href="aggregator_factory.md#0x1_aggregator_factory_create_aggregator">aggregator_factory::create_aggregator</a>(aptos_framework, <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>);<br />    <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt; &#123;<br />        value: <a href="aggregator.md#0x1_aggregator">aggregator</a>,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_is_aggregatable_coin_zero"></a>

## Function `is_aggregatable_coin_zero`

Returns true if the value of aggregatable coin is zero.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_is_aggregatable_coin_zero">is_aggregatable_coin_zero</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_is_aggregatable_coin_zero">is_aggregatable_coin_zero</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt;): bool &#123;<br />    <b>let</b> amount &#61; <a href="aggregator.md#0x1_aggregator_read">aggregator::read</a>(&amp;<a href="coin.md#0x1_coin">coin</a>.value);<br />    amount &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_drain_aggregatable_coin"></a>

## Function `drain_aggregatable_coin`

Drains the aggregatable coin, setting it to zero and returning a standard coin.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_drain_aggregatable_coin">drain_aggregatable_coin</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_drain_aggregatable_coin">drain_aggregatable_coin</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />    <b>spec</b> &#123;<br />        // TODO: The data <b>invariant</b> is not properly assumed from CollectedFeesPerBlock.<br />        <b>assume</b> <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="coin.md#0x1_coin">coin</a>.value) &#61;&#61; <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>let</b> amount &#61; <a href="aggregator.md#0x1_aggregator_read">aggregator::read</a>(&amp;<a href="coin.md#0x1_coin">coin</a>.value);<br />    <b>assert</b>!(amount &lt;&#61; <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="coin.md#0x1_coin_EAGGREGATABLE_COIN_VALUE_TOO_LARGE">EAGGREGATABLE_COIN_VALUE_TOO_LARGE</a>));<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt; &#45; amount;<br />    &#125;;<br />    <a href="aggregator.md#0x1_aggregator_sub">aggregator::sub</a>(&amp;<b>mut</b> <a href="coin.md#0x1_coin">coin</a>.value, amount);<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; amount;<br />    &#125;;<br />    <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />        value: (amount <b>as</b> u64),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_merge_aggregatable_coin"></a>

## Function `merge_aggregatable_coin`

Merges <code><a href="coin.md#0x1_coin">coin</a></code> into aggregatable coin (<code>dst_coin</code>).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_merge_aggregatable_coin">merge_aggregatable_coin</a>&lt;CoinType&gt;(dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_merge_aggregatable_coin">merge_aggregatable_coin</a>&lt;CoinType&gt;(<br />    dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt;,<br />    <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;<br />) &#123;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#45; <a href="coin.md#0x1_coin">coin</a>.value;<br />    &#125;;<br />    <b>let</b> <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value &#125; &#61; <a href="coin.md#0x1_coin">coin</a>;<br />    <b>let</b> amount &#61; (value <b>as</b> u128);<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt; &#43; amount;<br />    &#125;;<br />    <a href="aggregator.md#0x1_aggregator_add">aggregator::add</a>(&amp;<b>mut</b> dst_coin.value, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_collect_into_aggregatable_coin"></a>

## Function `collect_into_aggregatable_coin`

Collects a specified amount of coin form an account into aggregatable coin.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">collect_into_aggregatable_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>, amount: u64, dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">collect_into_aggregatable_coin</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    amount: u64,<br />    dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt;,<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> &#123;<br />    // Skip collecting <b>if</b> amount is zero.<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    <b>let</b> (coin_amount_to_collect, fa_amount_to_collect) &#61; <a href="coin.md#0x1_coin_calculate_amount_to_withdraw">calculate_amount_to_withdraw</a>&lt;CoinType&gt;(<br />        account_addr,<br />        amount<br />    );<br />    <b>let</b> <a href="coin.md#0x1_coin">coin</a> &#61; <b>if</b> (coin_amount_to_collect &gt; 0) &#123;<br />        <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />        <a href="coin.md#0x1_coin_extract">extract</a>(&amp;<b>mut</b> coin_store.<a href="coin.md#0x1_coin">coin</a>, coin_amount_to_collect)<br />    &#125; <b>else</b> &#123;<br />        <a href="coin.md#0x1_coin_zero">zero</a>()<br />    &#125;;<br />    <b>if</b> (fa_amount_to_collect &gt; 0) &#123;<br />        <b>let</b> store_addr &#61; <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_fungible_store::primary_store_address</a>(<br />            account_addr,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(<a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;())<br />        );<br />        <b>let</b> fa &#61; <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">fungible_asset::withdraw_internal</a>(store_addr, fa_amount_to_collect);<br />        <a href="coin.md#0x1_coin_merge">merge</a>(&amp;<b>mut</b> <a href="coin.md#0x1_coin">coin</a>, <a href="coin.md#0x1_coin_fungible_asset_to_coin">fungible_asset_to_coin</a>&lt;CoinType&gt;(fa));<br />    &#125;;<br />    <a href="coin.md#0x1_coin_merge_aggregatable_coin">merge_aggregatable_coin</a>(dst_coin, <a href="coin.md#0x1_coin">coin</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_calculate_amount_to_withdraw"></a>

## Function `calculate_amount_to_withdraw`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_calculate_amount_to_withdraw">calculate_amount_to_withdraw</a>&lt;CoinType&gt;(account_addr: <b>address</b>, amount: u64): (u64, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_calculate_amount_to_withdraw">calculate_amount_to_withdraw</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    amount: u64<br />): (u64, u64) &#123;<br />    <b>let</b> coin_balance &#61; <a href="coin.md#0x1_coin_coin_balance">coin_balance</a>&lt;CoinType&gt;(account_addr);<br />    <b>if</b> (coin_balance &gt;&#61; amount) &#123;<br />        (amount, 0)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;metadata) &amp;&amp; <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_fungible_store::primary_store_exists</a>(<br />            account_addr,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(metadata)<br />        ))<br />            (coin_balance, amount &#45; coin_balance)<br />        <b>else</b><br />            <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_maybe_convert_to_fungible_store"></a>

## Function `maybe_convert_to_fungible_store`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_maybe_convert_to_fungible_store">maybe_convert_to_fungible_store</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_maybe_convert_to_fungible_store">maybe_convert_to_fungible_store</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_coin_to_fungible_asset_migration_feature_enabled">features::coin_to_fungible_asset_migration_feature_enabled</a>()) &#123;<br />        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="coin.md#0x1_coin_ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED">ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED</a>)<br />    &#125;;<br />    <b>assert</b>!(<a href="coin.md#0x1_coin_is_coin_initialized">is_coin_initialized</a>&lt;CoinType&gt;(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_INFO_NOT_PUBLISHED">ECOIN_INFO_NOT_PUBLISHED</a>));<br /><br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_ensure_paired_metadata">ensure_paired_metadata</a>&lt;CoinType&gt;();<br />    <b>let</b> store &#61; <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(<a href="account.md#0x1_account">account</a>, metadata);<br />    <b>let</b> store_address &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store);<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(<a href="account.md#0x1_account">account</a>)) &#123;<br />        <b>let</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt; &#123; <a href="coin.md#0x1_coin">coin</a>, frozen, deposit_events, withdraw_events &#125; &#61; <b>move_from</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(<br />            <a href="account.md#0x1_account">account</a><br />        );<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="coin.md#0x1_coin_CoinEventHandleDeletion">CoinEventHandleDeletion</a> &#123;<br />                event_handle_creation_address: <a href="guid.md#0x1_guid_creator_address">guid::creator_address</a>(<br />                    <a href="event.md#0x1_event_guid">event::guid</a>(&amp;deposit_events)<br />                ),<br />                deleted_deposit_event_handle_creation_number: <a href="guid.md#0x1_guid_creation_num">guid::creation_num</a>(<a href="event.md#0x1_event_guid">event::guid</a>(&amp;deposit_events)),<br />                deleted_withdraw_event_handle_creation_number: <a href="guid.md#0x1_guid_creation_num">guid::creation_num</a>(<a href="event.md#0x1_event_guid">event::guid</a>(&amp;withdraw_events))<br />            &#125;<br />        );<br />        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(deposit_events);<br />        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(withdraw_events);<br />        <b>if</b> (<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; 0) &#123;<br />            <a href="coin.md#0x1_coin_destroy_zero">destroy_zero</a>(<a href="coin.md#0x1_coin">coin</a>);<br />        &#125; <b>else</b> &#123;<br />            <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(store, <a href="coin.md#0x1_coin_coin_to_fungible_asset">coin_to_fungible_asset</a>(<a href="coin.md#0x1_coin">coin</a>));<br />        &#125;;<br />        // Note:<br />        // It is possible the primary fungible store may already exist before this function call.<br />        // In this case, <b>if</b> the <a href="account.md#0x1_account">account</a> owns a frozen <a href="coin.md#0x1_coin_CoinStore">CoinStore</a> and an unfrozen primary fungible store, this<br />        // function would convert and deposit the rest <a href="coin.md#0x1_coin">coin</a> into the primary store and <b>freeze</b> it <b>to</b> make the<br />        // `frozen` semantic <b>as</b> consistent <b>as</b> possible.<br />        <b>if</b> (frozen !&#61; <a href="fungible_asset.md#0x1_fungible_asset_is_frozen">fungible_asset::is_frozen</a>(store)) &#123;<br />            <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">fungible_asset::set_frozen_flag_internal</a>(store, frozen);<br />        &#125;<br />    &#125;;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="coin.md#0x1_coin_MigrationFlag">MigrationFlag</a>&gt;(store_address)) &#123;<br />        <b>move_to</b>(&amp;<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(store_address), <a href="coin.md#0x1_coin_MigrationFlag">MigrationFlag</a> &#123;&#125;);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_migrate_to_fungible_store"></a>

## Function `migrate_to_fungible_store`

Voluntarily migrate to fungible store for <code>CoinType</code> if not yet.


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_migrate_to_fungible_store">migrate_to_fungible_store</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_migrate_to_fungible_store">migrate_to_fungible_store</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <a href="coin.md#0x1_coin_maybe_convert_to_fungible_store">maybe_convert_to_fungible_store</a>&lt;CoinType&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_coin_address"></a>

## Function `coin_address`

A helper function that returns the address of CoinType.


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;(): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;(): <b>address</b> &#123;<br />    <b>let</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a> &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;();<br />    <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_account_address">type_info::account_address</a>(&amp;<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_balance"></a>

## Function `balance`

Returns the balance of <code>owner</code> for provided <code>CoinType</code> and its paired FA if exists.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_balance">balance</a>&lt;CoinType&gt;(owner: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_balance">balance</a>&lt;CoinType&gt;(owner: <b>address</b>): u64 <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinStore">CoinStore</a> &#123;<br />    <b>let</b> paired_metadata &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />    <a href="coin.md#0x1_coin_coin_balance">coin_balance</a>&lt;CoinType&gt;(owner) &#43; <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;paired_metadata)) &#123;<br />        <a href="primary_fungible_store.md#0x1_primary_fungible_store_balance">primary_fungible_store::balance</a>(<br />            owner,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> paired_metadata)<br />        )<br />    &#125; <b>else</b> &#123; 0 &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_is_balance_at_least"></a>

## Function `is_balance_at_least`

Returns whether the balance of <code>owner</code> for provided <code>CoinType</code> and its paired FA is &gt;&#61; <code>amount</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_balance_at_least">is_balance_at_least</a>&lt;CoinType&gt;(owner: <b>address</b>, amount: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_balance_at_least">is_balance_at_least</a>&lt;CoinType&gt;(owner: <b>address</b>, amount: u64): bool <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinStore">CoinStore</a> &#123;<br />    <b>let</b> coin_balance &#61; <a href="coin.md#0x1_coin_coin_balance">coin_balance</a>&lt;CoinType&gt;(owner);<br />    <b>if</b> (coin_balance &gt;&#61; amount) &#123;<br />        <b>return</b> <b>true</b><br />    &#125;;<br /><br />    <b>let</b> paired_metadata &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />    <b>let</b> left_amount &#61; amount &#45; coin_balance;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;paired_metadata)) &#123;<br />        <a href="primary_fungible_store.md#0x1_primary_fungible_store_is_balance_at_least">primary_fungible_store::is_balance_at_least</a>(<br />            owner,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> paired_metadata),<br />            left_amount<br />        )<br />    &#125; <b>else</b> &#123; <b>false</b> &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_coin_balance"></a>

## Function `coin_balance`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_coin_balance">coin_balance</a>&lt;CoinType&gt;(owner: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_coin_balance">coin_balance</a>&lt;CoinType&gt;(owner: <b>address</b>): u64 &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(owner)) &#123;<br />        <b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(owner).<a href="coin.md#0x1_coin">coin</a>.value<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_is_coin_initialized"></a>

## Function `is_coin_initialized`

Returns <code><b>true</b></code> if the type <code>CoinType</code> is an initialized coin.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_coin_initialized">is_coin_initialized</a>&lt;CoinType&gt;(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_coin_initialized">is_coin_initialized</a>&lt;CoinType&gt;(): bool &#123;<br />    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;())<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_is_coin_store_frozen"></a>

## Function `is_coin_store_frozen`

Returns <code><b>true</b></code> is account_addr has frozen the CoinStore or if it&apos;s not registered at all


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_coin_store_frozen">is_coin_store_frozen</a>&lt;CoinType&gt;(account_addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_coin_store_frozen">is_coin_store_frozen</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b><br />): bool <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> &#123;<br />    <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">is_account_registered</a>&lt;CoinType&gt;(account_addr)) &#123;<br />        <b>return</b> <b>true</b><br />    &#125;;<br /><br />    <b>let</b> coin_store &#61; <b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />    coin_store.frozen<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_is_account_registered"></a>

## Function `is_account_registered`

Returns <code><b>true</b></code> if <code>account_addr</code> is registered to receive <code>CoinType</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_account_registered">is_account_registered</a>&lt;CoinType&gt;(account_addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_account_registered">is_account_registered</a>&lt;CoinType&gt;(account_addr: <b>address</b>): bool <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> &#123;<br />    <b>assert</b>!(<a href="coin.md#0x1_coin_is_coin_initialized">is_coin_initialized</a>&lt;CoinType&gt;(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_INFO_NOT_PUBLISHED">ECOIN_INFO_NOT_PUBLISHED</a>));<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr)) &#123;<br />        <b>true</b><br />    &#125; <b>else</b> &#123;<br />        <b>let</b> paired_metadata_opt &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />        (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<br />            &amp;paired_metadata_opt<br />        ) &amp;&amp; <a href="coin.md#0x1_coin_migrated_primary_fungible_store_exists">migrated_primary_fungible_store_exists</a>(account_addr, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(paired_metadata_opt)))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_name"></a>

## Function `name`

Returns the name of the coin.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_name">name</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_name">name</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a> <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;()).name<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_symbol"></a>

## Function `symbol`

Returns the symbol of the coin, usually a shorter version of the name.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_symbol">symbol</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_symbol">symbol</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a> <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;()).symbol<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_decimals"></a>

## Function `decimals`

Returns the number of decimals used to get its user representation.
For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_decimals">decimals</a>&lt;CoinType&gt;(): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_decimals">decimals</a>&lt;CoinType&gt;(): u8 <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;()).decimals<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_supply"></a>

## Function `supply`

Returns the amount of coin in existence.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;(): Option&lt;u128&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> &#123;<br />    <b>let</b> coin_supply &#61; <a href="coin.md#0x1_coin_coin_supply">coin_supply</a>&lt;CoinType&gt;();<br />    <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;metadata)) &#123;<br />        <b>let</b> fungible_asset_supply &#61; <a href="fungible_asset.md#0x1_fungible_asset_supply">fungible_asset::supply</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> metadata));<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;coin_supply)) &#123;<br />            <b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> coin_supply);<br />            &#42;<a href="coin.md#0x1_coin_supply">supply</a> &#61; &#42;<a href="coin.md#0x1_coin_supply">supply</a> &#43; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(fungible_asset_supply);<br />        &#125;;<br />    &#125;;<br />    coin_supply<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_coin_supply"></a>

## Function `coin_supply`

Returns the amount of coin in existence.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_coin_supply">coin_supply</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_coin_supply">coin_supply</a>&lt;CoinType&gt;(): Option&lt;u128&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>let</b> maybe_supply &#61; &amp;<b>borrow_global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;()).<a href="coin.md#0x1_coin_supply">supply</a>;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(maybe_supply)) &#123;<br />        // We do track <a href="coin.md#0x1_coin_supply">supply</a>, in this case read from optional <a href="aggregator.md#0x1_aggregator">aggregator</a>.<br />        <b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(maybe_supply);<br />        <b>let</b> value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_read">optional_aggregator::read</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(value)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_burn"></a>

## Function `burn`

Burn <code><a href="coin.md#0x1_coin">coin</a></code> with capability.
The capability <code>_cap</code> should be passed as a reference to <code><a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_burn">burn</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;, _cap: &amp;<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_burn">burn</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;, _cap: &amp;<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;) <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <a href="coin.md#0x1_coin_burn_internal">burn_internal</a>(<a href="coin.md#0x1_coin">coin</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_burn_from"></a>

## Function `burn_from`

Burn <code><a href="coin.md#0x1_coin">coin</a></code> from the specified <code><a href="account.md#0x1_account">account</a></code> with capability.
The capability <code>burn_cap</code> should be passed as a reference to <code><a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;</code>.
This function shouldn&apos;t fail as it&apos;s called as part of transaction fee burning.

Note: This bypasses CoinStore::frozen &#45;&#45; coins within a frozen CoinStore can be burned.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_burn_from">burn_from</a>&lt;CoinType&gt;(account_addr: <b>address</b>, amount: u64, burn_cap: &amp;<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_burn_from">burn_from</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    amount: u64,<br />    burn_cap: &amp;<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;,<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_PairedFungibleAssetRefs">PairedFungibleAssetRefs</a> &#123;<br />    // Skip burning <b>if</b> amount is zero. This shouldn&apos;t <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> out <b>as</b> it&apos;s called <b>as</b> part of transaction fee burning.<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    <b>let</b> (coin_amount_to_burn, fa_amount_to_burn) &#61; <a href="coin.md#0x1_coin_calculate_amount_to_withdraw">calculate_amount_to_withdraw</a>&lt;CoinType&gt;(<br />        account_addr,<br />        amount<br />    );<br />    <b>if</b> (coin_amount_to_burn &gt; 0) &#123;<br />        <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />        <b>let</b> coin_to_burn &#61; <a href="coin.md#0x1_coin_extract">extract</a>(&amp;<b>mut</b> coin_store.<a href="coin.md#0x1_coin">coin</a>, coin_amount_to_burn);<br />        <a href="coin.md#0x1_coin_burn">burn</a>(coin_to_burn, burn_cap);<br />    &#125;;<br />    <b>if</b> (fa_amount_to_burn &gt; 0) &#123;<br />        <a href="fungible_asset.md#0x1_fungible_asset_burn_from">fungible_asset::burn_from</a>(<br />            <a href="coin.md#0x1_coin_borrow_paired_burn_ref">borrow_paired_burn_ref</a>(burn_cap),<br />            <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_fungible_store::primary_store</a>(account_addr, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(<a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;())),<br />            fa_amount_to_burn<br />        );<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_deposit"></a>

## Function `deposit`

Deposit the coin balance into the recipient&apos;s account and emit an event.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_deposit">deposit</a>&lt;CoinType&gt;(account_addr: <b>address</b>, <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_deposit">deposit</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr)) &#123;<br />        <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />        <b>assert</b>!(<br />            !coin_store.frozen,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="coin.md#0x1_coin_EFROZEN">EFROZEN</a>),<br />        );<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="coin.md#0x1_coin_Deposit">Deposit</a>&lt;CoinType&gt; &#123; <a href="account.md#0x1_account">account</a>: account_addr, amount: <a href="coin.md#0x1_coin">coin</a>.value &#125;);<br />        &#125;;<br />        <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="coin.md#0x1_coin_DepositEvent">DepositEvent</a>&gt;(<br />            &amp;<b>mut</b> coin_store.deposit_events,<br />            <a href="coin.md#0x1_coin_DepositEvent">DepositEvent</a> &#123; amount: <a href="coin.md#0x1_coin">coin</a>.value &#125;,<br />        );<br />        <a href="coin.md#0x1_coin_merge">merge</a>(&amp;<b>mut</b> coin_store.<a href="coin.md#0x1_coin">coin</a>, <a href="coin.md#0x1_coin">coin</a>);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;metadata) &amp;&amp; <a href="coin.md#0x1_coin_migrated_primary_fungible_store_exists">migrated_primary_fungible_store_exists</a>(<br />            account_addr,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(metadata)<br />        )) &#123;<br />            <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">primary_fungible_store::deposit</a>(account_addr, <a href="coin.md#0x1_coin_coin_to_fungible_asset">coin_to_fungible_asset</a>(<a href="coin.md#0x1_coin">coin</a>));<br />        &#125; <b>else</b> &#123;<br />            <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_ECOIN_STORE_NOT_PUBLISHED">ECOIN_STORE_NOT_PUBLISHED</a>)<br />        &#125;;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_migrated_primary_fungible_store_exists"></a>

## Function `migrated_primary_fungible_store_exists`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_migrated_primary_fungible_store_exists">migrated_primary_fungible_store_exists</a>(account_address: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="coin.md#0x1_coin_migrated_primary_fungible_store_exists">migrated_primary_fungible_store_exists</a>(<br />    account_address: <b>address</b>,<br />    metadata: Object&lt;Metadata&gt;<br />): bool &#123;<br />    <b>let</b> primary_store_address &#61; <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_fungible_store::primary_store_address</a>&lt;Metadata&gt;(account_address, metadata);<br />    <a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(primary_store_address) &amp;&amp; <b>exists</b>&lt;<a href="coin.md#0x1_coin_MigrationFlag">MigrationFlag</a>&gt;(primary_store_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_force_deposit"></a>

## Function `force_deposit`

Deposit the coin balance into the recipient&apos;s account without checking if the account is frozen.
This is for internal use only and doesn&apos;t emit an DepositEvent.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_force_deposit">force_deposit</a>&lt;CoinType&gt;(account_addr: <b>address</b>, <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_force_deposit">force_deposit</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr)) &#123;<br />        <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />        <a href="coin.md#0x1_coin_merge">merge</a>(&amp;<b>mut</b> coin_store.<a href="coin.md#0x1_coin">coin</a>, <a href="coin.md#0x1_coin">coin</a>);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> metadata &#61; <a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;();<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;metadata) &amp;&amp; <a href="coin.md#0x1_coin_migrated_primary_fungible_store_exists">migrated_primary_fungible_store_exists</a>(<br />            account_addr,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(metadata)<br />        )) &#123;<br />            <b>let</b> fa &#61; <a href="coin.md#0x1_coin_coin_to_fungible_asset">coin_to_fungible_asset</a>(<a href="coin.md#0x1_coin">coin</a>);<br />            <b>let</b> metadata &#61; <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">fungible_asset::asset_metadata</a>(&amp;fa);<br />            <b>let</b> store &#61; <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_fungible_store::primary_store</a>(account_addr, metadata);<br />            <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">fungible_asset::deposit_internal</a>(store, fa);<br />        &#125; <b>else</b> &#123;<br />            <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="coin.md#0x1_coin_ECOIN_STORE_NOT_PUBLISHED">ECOIN_STORE_NOT_PUBLISHED</a>)<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_destroy_zero"></a>

## Function `destroy_zero`

Destroys a zero&#45;value coin. Calls will fail if the <code>value</code> in the passed&#45;in <code>token</code> is non&#45;zero
so it is impossible to &quot;burn&quot; any non&#45;zero amount of <code><a href="coin.md#0x1_coin_Coin">Coin</a></code> without having
a <code><a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a></code> for the specific <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_zero">destroy_zero</a>&lt;CoinType&gt;(zero_coin: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_zero">destroy_zero</a>&lt;CoinType&gt;(zero_coin: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;) &#123;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#45; zero_coin.value;<br />    &#125;;<br />    <b>let</b> <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value &#125; &#61; zero_coin;<br />    <b>assert</b>!(value &#61;&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_EDESTRUCTION_OF_NONZERO_TOKEN">EDESTRUCTION_OF_NONZERO_TOKEN</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_extract"></a>

## Function `extract`

Extracts <code>amount</code> from the passed&#45;in <code><a href="coin.md#0x1_coin">coin</a></code>, where the original token is modified in place.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_extract">extract</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;, amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_extract">extract</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;, amount: u64): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />    <b>assert</b>!(<a href="coin.md#0x1_coin">coin</a>.value &gt;&#61; amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#45; amount;<br />    &#125;;<br />    <a href="coin.md#0x1_coin">coin</a>.value &#61; <a href="coin.md#0x1_coin">coin</a>.value &#45; amount;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; amount;<br />    &#125;;<br />    <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value: amount &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_extract_all"></a>

## Function `extract_all`

Extracts the entire amount from the passed&#45;in <code><a href="coin.md#0x1_coin">coin</a></code>, where the original token is modified in place.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_extract_all">extract_all</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_extract_all">extract_all</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />    <b>let</b> total_value &#61; <a href="coin.md#0x1_coin">coin</a>.value;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#45; <a href="coin.md#0x1_coin">coin</a>.value;<br />    &#125;;<br />    <a href="coin.md#0x1_coin">coin</a>.value &#61; 0;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; total_value;<br />    &#125;;<br />    <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value: total_value &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_freeze_coin_store"></a>

## Function `freeze_coin_store`

Freeze a CoinStore to prevent transfers


<pre><code>&#35;[legacy_entry_fun]<br /><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_freeze_coin_store">freeze_coin_store</a>&lt;CoinType&gt;(account_addr: <b>address</b>, _freeze_cap: &amp;<a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_freeze_coin_store">freeze_coin_store</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    _freeze_cap: &amp;<a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;,<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a> &#123;<br />    <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />    coin_store.frozen &#61; <b>true</b>;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_unfreeze_coin_store"></a>

## Function `unfreeze_coin_store`

Unfreeze a CoinStore to allow transfers


<pre><code>&#35;[legacy_entry_fun]<br /><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_unfreeze_coin_store">unfreeze_coin_store</a>&lt;CoinType&gt;(account_addr: <b>address</b>, _freeze_cap: &amp;<a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_unfreeze_coin_store">unfreeze_coin_store</a>&lt;CoinType&gt;(<br />    account_addr: <b>address</b>,<br />    _freeze_cap: &amp;<a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;,<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a> &#123;<br />    <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />    coin_store.frozen &#61; <b>false</b>;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_upgrade_supply"></a>

## Function `upgrade_supply`

Upgrade total supply to use a parallelizable implementation if it is
available.


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_upgrade_supply">upgrade_supply</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_upgrade_supply">upgrade_supply</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    // Only <a href="coin.md#0x1_coin">coin</a> creators can upgrade total <a href="coin.md#0x1_coin_supply">supply</a>.<br />    <b>assert</b>!(<br />        <a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;() &#61;&#61; account_addr,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_INFO_ADDRESS_MISMATCH">ECOIN_INFO_ADDRESS_MISMATCH</a>),<br />    );<br /><br />    // Can only succeed once on&#45;chain governance agreed on the upgrade.<br />    <b>assert</b>!(<br />        <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(@aptos_framework).allow_upgrades,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="coin.md#0x1_coin_ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED">ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED</a>)<br />    );<br /><br />    <b>let</b> maybe_supply &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(maybe_supply)) &#123;<br />        <b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(maybe_supply);<br /><br />        // If <a href="coin.md#0x1_coin_supply">supply</a> is tracked and the current implementation uses an integer &#45; upgrade.<br />        <b>if</b> (!<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">optional_aggregator::is_parallelizable</a>(<a href="coin.md#0x1_coin_supply">supply</a>)) &#123;<br />            <a href="optional_aggregator.md#0x1_optional_aggregator_switch">optional_aggregator::switch</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_initialize"></a>

## Function `initialize`

Creates a new Coin with given <code>CoinType</code> and returns minting/freezing/burning capabilities.
The given signer also becomes the account hosting the information  about the coin
(name, supply, etc.). Supply is initialized as non&#45;parallelizable integer.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_initialize">initialize</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, monitor_supply: bool): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_initialize">initialize</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>,<br />    symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>,<br />    decimals: u8,<br />    monitor_supply: bool,<br />): (<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;) &#123;<br />    <a href="coin.md#0x1_coin_initialize_internal">initialize_internal</a>(<a href="account.md#0x1_account">account</a>, name, symbol, decimals, monitor_supply, <b>false</b>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_initialize_with_parallelizable_supply"></a>

## Function `initialize_with_parallelizable_supply`

Same as <code>initialize</code> but supply can be initialized to parallelizable aggregator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_with_parallelizable_supply">initialize_with_parallelizable_supply</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, monitor_supply: bool): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_with_parallelizable_supply">initialize_with_parallelizable_supply</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>,<br />    symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>,<br />    decimals: u8,<br />    monitor_supply: bool,<br />): (<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);<br />    <a href="coin.md#0x1_coin_initialize_internal">initialize_internal</a>(<a href="account.md#0x1_account">account</a>, name, symbol, decimals, monitor_supply, <b>true</b>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_initialize_internal"></a>

## Function `initialize_internal`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_initialize_internal">initialize_internal</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, monitor_supply: bool, parallelizable: bool): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_initialize_internal">initialize_internal</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>,<br />    symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>,<br />    decimals: u8,<br />    monitor_supply: bool,<br />    parallelizable: bool,<br />): (<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;) &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    <b>assert</b>!(<br />        <a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;() &#61;&#61; account_addr,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_INFO_ADDRESS_MISMATCH">ECOIN_INFO_ADDRESS_MISMATCH</a>),<br />    );<br /><br />    <b>assert</b>!(<br />        !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="coin.md#0x1_coin_ECOIN_INFO_ALREADY_PUBLISHED">ECOIN_INFO_ALREADY_PUBLISHED</a>),<br />    );<br /><br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="coin.md#0x1_coin_MAX_COIN_NAME_LENGTH">MAX_COIN_NAME_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_NAME_TOO_LONG">ECOIN_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;symbol) &lt;&#61; <a href="coin.md#0x1_coin_MAX_COIN_SYMBOL_LENGTH">MAX_COIN_SYMBOL_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="coin.md#0x1_coin_ECOIN_SYMBOL_TOO_LONG">ECOIN_SYMBOL_TOO_LONG</a>));<br /><br />    <b>let</b> coin_info &#61; <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt; &#123;<br />        name,<br />        symbol,<br />        decimals,<br />        <a href="coin.md#0x1_coin_supply">supply</a>: <b>if</b> (monitor_supply) &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<br />                <a href="optional_aggregator.md#0x1_optional_aggregator_new">optional_aggregator::new</a>(<a href="coin.md#0x1_coin_MAX_U128">MAX_U128</a>, parallelizable)<br />            )<br />        &#125; <b>else</b> &#123; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() &#125;,<br />    &#125;;<br />    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, coin_info);<br /><br />    (<a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt; &#123;&#125;, <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt; &#123;&#125;, <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt; &#123;&#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_merge"></a>

## Function `merge`

&quot;Merges&quot; the two given coins.  The coin passed in as <code>dst_coin</code> will have a value equal
to the sum of the two tokens (<code>dst_coin</code> and <code>source_coin</code>).


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_merge">merge</a>&lt;CoinType&gt;(dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;, source_coin: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_merge">merge</a>&lt;CoinType&gt;(dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;, source_coin: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;) &#123;<br />    <b>spec</b> &#123;<br />        <b>assume</b> dst_coin.value &#43; source_coin.<a href="coin.md#0x1_coin_value">value</a> &lt;&#61; <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#45; source_coin.value;<br />    &#125;;<br />    <b>let</b> <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value &#125; &#61; source_coin;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; value;<br />    &#125;;<br />    dst_coin.value &#61; dst_coin.value &#43; value;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_mint"></a>

## Function `mint`

Mint new <code><a href="coin.md#0x1_coin_Coin">Coin</a></code> with capability.
The capability <code>_cap</code> should be passed as reference to <code><a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;</code>.
Returns minted <code><a href="coin.md#0x1_coin_Coin">Coin</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_mint">mint</a>&lt;CoinType&gt;(amount: u64, _cap: &amp;<a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_mint">mint</a>&lt;CoinType&gt;(<br />    amount: u64,<br />    _cap: &amp;<a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;,<br />): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <a href="coin.md#0x1_coin_mint_internal">mint_internal</a>&lt;CoinType&gt;(amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_register"></a>

## Function `register`



<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_register">register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_register">register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    // Short&#45;circuit and do nothing <b>if</b> <a href="account.md#0x1_account">account</a> is already registered for CoinType.<br />    <b>if</b> (<a href="coin.md#0x1_coin_is_account_registered">is_account_registered</a>&lt;CoinType&gt;(account_addr)) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    <a href="account.md#0x1_account_register_coin">account::register_coin</a>&lt;CoinType&gt;(account_addr);<br />    <b>let</b> coin_store &#61; <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt; &#123;<br />        <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value: 0 &#125;,<br />        frozen: <b>false</b>,<br />        deposit_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="coin.md#0x1_coin_DepositEvent">DepositEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />        withdraw_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="coin.md#0x1_coin_WithdrawEvent">WithdrawEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />    &#125;;<br />    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, coin_store);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of coins <code>CoinType</code> from <code>from</code> to <code><b>to</b></code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_transfer">transfer</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_transfer">transfer</a>&lt;CoinType&gt;(<br />    from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <b>to</b>: <b>address</b>,<br />    amount: u64,<br />) <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin">coin</a> &#61; <a href="coin.md#0x1_coin_withdraw">withdraw</a>&lt;CoinType&gt;(from, amount);<br />    <a href="coin.md#0x1_coin_deposit">deposit</a>(<b>to</b>, <a href="coin.md#0x1_coin">coin</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_value"></a>

## Function `value`

Returns the <code>value</code> passed in <code><a href="coin.md#0x1_coin">coin</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_value">value</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_value">value</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;): u64 &#123;<br />    <a href="coin.md#0x1_coin">coin</a>.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_withdraw"></a>

## Function `withdraw`

Withdraw specified <code>amount</code> of coin <code>CoinType</code> from the signing account.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_withdraw">withdraw</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_withdraw">withdraw</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    amount: u64,<br />): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinStore">CoinStore</a>, <a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>, <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>, <a href="coin.md#0x1_coin_PairedCoinType">PairedCoinType</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    <b>let</b> (coin_amount_to_withdraw, fa_amount_to_withdraw) &#61; <a href="coin.md#0x1_coin_calculate_amount_to_withdraw">calculate_amount_to_withdraw</a>&lt;CoinType&gt;(<br />        account_addr,<br />        amount<br />    );<br />    <b>let</b> withdrawn_coin &#61; <b>if</b> (coin_amount_to_withdraw &gt; 0) &#123;<br />        <b>let</b> coin_store &#61; <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />        <b>assert</b>!(<br />            !coin_store.frozen,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="coin.md#0x1_coin_EFROZEN">EFROZEN</a>),<br />        );<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="coin.md#0x1_coin_Withdraw">Withdraw</a>&lt;CoinType&gt; &#123; <a href="account.md#0x1_account">account</a>: account_addr, amount: coin_amount_to_withdraw &#125;);<br />        &#125;;<br />        <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="coin.md#0x1_coin_WithdrawEvent">WithdrawEvent</a>&gt;(<br />            &amp;<b>mut</b> coin_store.withdraw_events,<br />            <a href="coin.md#0x1_coin_WithdrawEvent">WithdrawEvent</a> &#123; amount: coin_amount_to_withdraw &#125;,<br />        );<br />        <a href="coin.md#0x1_coin_extract">extract</a>(&amp;<b>mut</b> coin_store.<a href="coin.md#0x1_coin">coin</a>, coin_amount_to_withdraw)<br />    &#125; <b>else</b> &#123;<br />        <a href="coin.md#0x1_coin_zero">zero</a>()<br />    &#125;;<br />    <b>if</b> (fa_amount_to_withdraw &gt; 0) &#123;<br />        <b>let</b> fa &#61; <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw">primary_fungible_store::withdraw</a>(<br />            <a href="account.md#0x1_account">account</a>,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(<a href="coin.md#0x1_coin_paired_metadata">paired_metadata</a>&lt;CoinType&gt;()),<br />            fa_amount_to_withdraw<br />        );<br />        <a href="coin.md#0x1_coin_merge">merge</a>(&amp;<b>mut</b> withdrawn_coin, <a href="coin.md#0x1_coin_fungible_asset_to_coin">fungible_asset_to_coin</a>(fa));<br />    &#125;;<br />    withdrawn_coin<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_zero"></a>

## Function `zero`

Create a new <code><a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;</code> with a value of <code>0</code>.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_zero">zero</a>&lt;CoinType&gt;(): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_zero">zero</a>&lt;CoinType&gt;(): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; 0;<br />    &#125;;<br />    <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />        value: 0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_destroy_freeze_cap"></a>

## Function `destroy_freeze_cap`

Destroy a freeze capability. Freeze capability is dangerous and therefore should be destroyed if not used.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_freeze_cap">destroy_freeze_cap</a>&lt;CoinType&gt;(freeze_cap: <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_freeze_cap">destroy_freeze_cap</a>&lt;CoinType&gt;(freeze_cap: <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt;) &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt; &#123;&#125; &#61; freeze_cap;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Destroy a mint capability.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_mint_cap">destroy_mint_cap</a>&lt;CoinType&gt;(mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_mint_cap">destroy_mint_cap</a>&lt;CoinType&gt;(mint_cap: <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt;) &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt; &#123;&#125; &#61; mint_cap;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_destroy_burn_cap"></a>

## Function `destroy_burn_cap`

Destroy a burn capability.


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_burn_cap">destroy_burn_cap</a>&lt;CoinType&gt;(burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_burn_cap">destroy_burn_cap</a>&lt;CoinType&gt;(burn_cap: <a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt;) &#123;<br />    <b>let</b> <a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt; &#123;&#125; &#61; burn_cap;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_mint_internal"></a>

## Function `mint_internal`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_mint_internal">mint_internal</a>&lt;CoinType&gt;(amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_mint_internal">mint_internal</a>&lt;CoinType&gt;(amount: u64): <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <b>return</b> <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123;<br />            value: 0<br />        &#125;<br />    &#125;;<br /><br />    <b>let</b> maybe_supply &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;()).<a href="coin.md#0x1_coin_supply">supply</a>;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(maybe_supply)) &#123;<br />        <b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(maybe_supply);<br />        <b>spec</b> &#123;<br />            <b>use</b> aptos_framework::optional_aggregator;<br />            <b>use</b> aptos_framework::aggregator;<br />            <b>assume</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">optional_aggregator::is_parallelizable</a>(<a href="coin.md#0x1_coin_supply">supply</a>) &#61;&#61;&gt; (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<br />                <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="coin.md#0x1_coin_supply">supply</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)<br />            )<br />                &#43; amount &lt;&#61; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="coin.md#0x1_coin_supply">supply</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)));<br />            <b>assume</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">optional_aggregator::is_parallelizable</a>(<a href="coin.md#0x1_coin_supply">supply</a>) &#61;&#61;&gt;<br />                (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="coin.md#0x1_coin_supply">supply</a>.integer).value &#43; amount &lt;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="coin.md#0x1_coin_supply">supply</a>.integer).limit);<br />        &#125;;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_add">optional_aggregator::add</a>(<a href="coin.md#0x1_coin_supply">supply</a>, (amount <b>as</b> u128));<br />    &#125;;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; amount;<br />    &#125;;<br />    <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123; value: amount &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_coin_burn_internal"></a>

## Function `burn_internal`



<pre><code><b>fun</b> <a href="coin.md#0x1_coin_burn_internal">burn_internal</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_burn_internal">burn_internal</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt;): u64 <b>acquires</b> <a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a> &#123;<br />    <b>spec</b> &#123;<br />        <b>update</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61; <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#45; <a href="coin.md#0x1_coin">coin</a>.value;<br />    &#125;;<br />    <b>let</b> <a href="coin.md#0x1_coin_Coin">Coin</a> &#123; value: amount &#125; &#61; <a href="coin.md#0x1_coin">coin</a>;<br />    <b>if</b> (amount !&#61; 0) &#123;<br />        <b>let</b> maybe_supply &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;()).<a href="coin.md#0x1_coin_supply">supply</a>;<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(maybe_supply)) &#123;<br />            <b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(maybe_supply);<br />            <a href="optional_aggregator.md#0x1_optional_aggregator_sub">optional_aggregator::sub</a>(<a href="coin.md#0x1_coin_supply">supply</a>, (amount <b>as</b> u128));<br />        &#125;;<br />    &#125;;<br />    amount<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Only the owner of a coin may mint, burn or freeze coins.</td>
<td>Critical</td>
<td>Acquiring capabilities for a particular CoinType may only occur if the caller has a signer for the module declaring that type. The initialize function returns these capabilities to the caller.</td>
<td>Formally Verified via <a href="#high-level-req-1.1">upgrade_supply</a> and <a href="#high-level-req-1.2">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Each coin may only be created exactly once.</td>
<td>Medium</td>
<td>The initialization function may only be called once.</td>
<td>Formally Verified via <a href="#high-level-req-2">initialize</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The merging of coins may only be done on coins of the same type.</td>
<td>Critical</td>
<td>The merge function is limited to merging coins of the same type only.</td>
<td>Formally Verified via <a href="#high-level-req-3">merge</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The supply of a coin is only affected by burn and mint operations.</td>
<td>High</td>
<td>Only mint and burn operations on a coin alter the total supply of coins.</td>
<td>Formally Verified via <a href="#high-level-req-4">TotalSupplyNoChange</a>.</td>
</tr>

<tr>
<td>5</td>
<td>Users may register an account for a coin multiple times idempotently.</td>
<td>Medium</td>
<td>The register function should work idempotently. Importantly, it should not abort if the coin is already registered.</td>
<td>Formally verified via aborts_if on <a href="#high-level-req-5">register</a>.</td>
</tr>

<tr>
<td>6</td>
<td>Coin operations should fail if the user has not registered for the coin.</td>
<td>Medium</td>
<td>Coin operations may succeed only on valid user coin registration.</td>
<td>Formally Verified via <a href="#high-level-req-6.1">balance</a>, <a href="#high-level-req-6.2">burn_from</a>, <a href="#high-level-req-6.3">freeze</a>, <a href="#high-level-req-6.4">unfreeze</a>, <a href="#high-level-req-6.5">transfer</a> and <a href="#high-level-req-6.6">withdraw</a>.</td>
</tr>

<tr>
<td>7</td>
<td>It should always be possible to (1) determine if a coin exists, and (2) determine if a user registered an account with a particular coin. If a coin exists, it should always be possible to request the following information of the coin: (1) Name, (2) Symbol, and (3) Supply.</td>
<td>Low</td>
<td>The following functions should never abort: (1) is_coin_initialized, and (2) is_account_registered. The following functions should not abort if the coin exists: (1) name, (2) symbol, and (3) supply.</td>
<td>Formally Verified in corresponding functions: <a href="#high-level-req-7.1">is_coin_initialized</a>, <a href="#high-level-req-7.2">is_account_registered</a>, <a href="#high-level-req-7.3">name</a>, <a href="#high-level-req-7.4">symbol</a> and <a href="#high-level-req-7.5">supply</a>.</td>
</tr>

<tr>
<td>8</td>
<td>Coin operations should fail if the user&apos;s CoinStore is frozen.</td>
<td>Medium</td>
<td>If the CoinStore of an address is frozen, coin operations are disallowed.</td>
<td>Formally Verified via <a href="#high-level-req-8.1">withdraw</a>, <a href="#high-level-req-8.2">transfer</a> and <a href="#high-level-req-8.3">deposit</a>.</td>
</tr>

<tr>
<td>9</td>
<td>Utilizing AggregatableCoins does not violate other critical invariants, such as (4).</td>
<td>High</td>
<td>Utilizing AggregatableCoin does not change the real&#45;supply of any token.</td>
<td>Formally Verified via <a href="#high-level-req-9">TotalSupplyNoChange</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><a id="0x1_coin_supply"></a>
<b>global</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;: num;<br /><a id="0x1_coin_aggregate_supply"></a>
<b>global</b> <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt;: num;<br /><b>apply</b> <a href="coin.md#0x1_coin_TotalSupplyTracked">TotalSupplyTracked</a>&lt;CoinType&gt; <b>to</b> &#42;&lt;CoinType&gt; <b>except</b><br />initialize, initialize_internal, initialize_with_parallelizable_supply;<br /></code></pre>




<a id="0x1_coin_spec_fun_supply_tracked"></a>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_spec_fun_supply_tracked">spec_fun_supply_tracked</a>&lt;CoinType&gt;(val: u64, <a href="coin.md#0x1_coin_supply">supply</a>: Option&lt;OptionalAggregator&gt;): bool &#123;<br />   <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<a href="coin.md#0x1_coin_supply">supply</a>) &#61;&#61;&gt; val &#61;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>
       (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="coin.md#0x1_coin_supply">supply</a>))<br />&#125;<br /></code></pre>




<a id="0x1_coin_TotalSupplyTracked"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_TotalSupplyTracked">TotalSupplyTracked</a>&lt;CoinType&gt; &#123;<br /><b>ensures</b> <b>old</b>(<a href="coin.md#0x1_coin_spec_fun_supply_tracked">spec_fun_supply_tracked</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt;,<br />    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address).<a href="coin.md#0x1_coin_supply">supply</a>)) &#61;&#61;&gt;<br />    <a href="coin.md#0x1_coin_spec_fun_supply_tracked">spec_fun_supply_tracked</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#43; <a href="coin.md#0x1_coin_aggregate_supply">aggregate_supply</a>&lt;CoinType&gt;,<br />        <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address).<a href="coin.md#0x1_coin_supply">supply</a>);<br />&#125;<br /></code></pre>




<a id="0x1_coin_spec_fun_supply_no_change"></a>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_spec_fun_supply_no_change">spec_fun_supply_no_change</a>&lt;CoinType&gt;(old_supply: Option&lt;OptionalAggregator&gt;,<br />                                            <a href="coin.md#0x1_coin_supply">supply</a>: Option&lt;OptionalAggregator&gt;): bool &#123;<br />   <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(old_supply) &#61;&#61;&gt; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>
       (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(old_supply)) &#61;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>
       (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="coin.md#0x1_coin_supply">supply</a>))<br />&#125;<br /></code></pre>




<a id="0x1_coin_TotalSupplyNoChange"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_TotalSupplyNoChange">TotalSupplyNoChange</a>&lt;CoinType&gt; &#123;<br /><b>let</b> old_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>let</b> <b>post</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>ensures</b> <a href="coin.md#0x1_coin_spec_fun_supply_no_change">spec_fun_supply_no_change</a>&lt;CoinType&gt;(old_supply, <a href="coin.md#0x1_coin_supply">supply</a>);<br />&#125;<br /></code></pre>



<a id="@Specification_1_AggregatableCoin"></a>

### Struct `AggregatableCoin`


<pre><code><b>struct</b> <a href="coin.md#0x1_coin_AggregatableCoin">AggregatableCoin</a>&lt;CoinType&gt; <b>has</b> store<br /></code></pre>



<dl>
<dt>
<code>value: <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a></code>
</dt>
<dd>
 Amount of aggregatable coin this address has.
</dd>
</dl>



<pre><code><b>invariant</b> <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(value) &#61;&#61; <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>;<br /></code></pre>



<a id="@Specification_1_coin_to_fungible_asset"></a>

### Function `coin_to_fungible_asset`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_coin_to_fungible_asset">coin_to_fungible_asset</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /></code></pre>



<a id="@Specification_1_fungible_asset_to_coin"></a>

### Function `fungible_asset_to_coin`


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_fungible_asset_to_coin">fungible_asset_to_coin</a>&lt;CoinType&gt;(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_initialize_supply_config"></a>

### Function `initialize_supply_config`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_supply_config">initialize_supply_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Can only be initialized once.
Can only be published by reserved addresses.


<pre><code><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(aptos_addr);<br /><b>ensures</b> !<b>global</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(aptos_addr).allow_upgrades;<br /><b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(aptos_addr);<br /></code></pre>



<a id="@Specification_1_allow_supply_upgrades"></a>

### Function `allow_supply_upgrades`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_allow_supply_upgrades">allow_supply_upgrades</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allowed: bool)<br /></code></pre>


Can only be updated by <code>@aptos_framework</code>.


<pre><code><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(@aptos_framework);<br /><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(aptos_addr);<br /><b>let</b> <b>post</b> allow_upgrades_post &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> allow_upgrades_post.allow_upgrades &#61;&#61; allowed;<br /></code></pre>



<a id="@Specification_1_initialize_aggregatable_coin"></a>

### Function `initialize_aggregatable_coin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_aggregatable_coin">initialize_aggregatable_coin</a>&lt;CoinType&gt;(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a> &#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>include</b> <a href="aggregator_factory.md#0x1_aggregator_factory_CreateAggregatorInternalAbortsIf">aggregator_factory::CreateAggregatorInternalAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_is_aggregatable_coin_zero"></a>

### Function `is_aggregatable_coin_zero`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_is_aggregatable_coin_zero">is_aggregatable_coin_zero</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (<a href="aggregator.md#0x1_aggregator_spec_read">aggregator::spec_read</a>(<a href="coin.md#0x1_coin">coin</a>.value) &#61;&#61; 0);<br /></code></pre>



<a id="@Specification_1_drain_aggregatable_coin"></a>

### Function `drain_aggregatable_coin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_drain_aggregatable_coin">drain_aggregatable_coin</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="aggregator.md#0x1_aggregator_spec_read">aggregator::spec_read</a>(<a href="coin.md#0x1_coin">coin</a>.value) &gt; <a href="coin.md#0x1_coin_MAX_U64">MAX_U64</a>;<br /><b>ensures</b> result.value &#61;&#61; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<b>old</b>(<a href="coin.md#0x1_coin">coin</a>).value);<br /></code></pre>



<a id="@Specification_1_merge_aggregatable_coin"></a>

### Function `merge_aggregatable_coin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_merge_aggregatable_coin">merge_aggregatable_coin</a>&lt;CoinType&gt;(dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>let</b> aggr &#61; dst_coin.value;<br /><b>let</b> <b>post</b> p_aggr &#61; dst_coin.value;<br /><b>aborts_if</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr)<br />    &#43; <a href="coin.md#0x1_coin">coin</a>.value &gt; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(aggr);<br /><b>aborts_if</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr)<br />    &#43; <a href="coin.md#0x1_coin">coin</a>.value &gt; <a href="coin.md#0x1_coin_MAX_U128">MAX_U128</a>;<br /><b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr) &#43; <a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(p_aggr);<br /></code></pre>



<a id="@Specification_1_collect_into_aggregatable_coin"></a>

### Function `collect_into_aggregatable_coin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">collect_into_aggregatable_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>, amount: u64, dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> aggr &#61; dst_coin.value;<br /><b>let</b> <b>post</b> p_aggr &#61; dst_coin.value;<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> <b>post</b> p_coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> amount &gt; 0 &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> amount &gt; 0 &amp;&amp; coin_store.<a href="coin.md#0x1_coin">coin</a>.<a href="coin.md#0x1_coin_value">value</a> &lt; amount;<br /><b>aborts_if</b> amount &gt; 0 &amp;&amp; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr)<br />    &#43; amount &gt; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(aggr);<br /><b>aborts_if</b> amount &gt; 0 &amp;&amp; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr)<br />    &#43; amount &gt; <a href="coin.md#0x1_coin_MAX_U128">MAX_U128</a>;<br /><b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr) &#43; amount &#61;&#61; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(p_aggr);<br /><b>ensures</b> coin_store.<a href="coin.md#0x1_coin">coin</a>.value &#45; amount &#61;&#61; p_coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_maybe_convert_to_fungible_store"></a>

### Function `maybe_convert_to_fungible_store`


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_maybe_convert_to_fungible_store">maybe_convert_to_fungible_store</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="account.md#0x1_account">account</a>);<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(<a href="account.md#0x1_account">account</a>);<br /></code></pre>




<a id="0x1_coin_DepositAbortsIf"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_DepositAbortsIf">DepositAbortsIf</a>&lt;CoinType&gt; &#123;<br />account_addr: <b>address</b>;<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> coin_store.frozen;<br />&#125;<br /></code></pre>



<a id="@Specification_1_coin_address"></a>

### Function `coin_address`


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_coin_address">coin_address</a>&lt;CoinType&gt;(): <b>address</b><br /></code></pre>


Get address by reflection.


<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /></code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_balance">balance</a>&lt;CoinType&gt;(owner: <b>address</b>): u64<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(owner);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(owner).<a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_is_coin_initialized"></a>

### Function `is_coin_initialized`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_coin_initialized">is_coin_initialized</a>&lt;CoinType&gt;(): bool<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-7.1" href="#high-level-req">high&#45;level requirement 7</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_is_account_registered"></a>

### Function `is_account_registered`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_is_account_registered">is_account_registered</a>&lt;CoinType&gt;(account_addr: <b>address</b>): bool<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_coin_get_coin_supply_opt"></a>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_get_coin_supply_opt">get_coin_supply_opt</a>&lt;CoinType&gt;(): Option&lt;OptionalAggregator&gt; &#123;<br />   <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address).<a href="coin.md#0x1_coin_supply">supply</a><br />&#125;<br /></code></pre>




<a id="0x1_coin_spec_paired_metadata"></a>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_spec_paired_metadata">spec_paired_metadata</a>&lt;CoinType&gt;(): Option&lt;Object&lt;Metadata&gt;&gt; &#123;<br />   <b>if</b> (<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework)) &#123;<br />       <b>let</b> map &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinConversionMap">CoinConversionMap</a>&gt;(@aptos_framework).coin_to_fungible_asset_map;<br />       <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(map, <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;())) &#123;<br />           <b>let</b> metadata &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(map, <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;());<br />           <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(metadata)<br />       &#125; <b>else</b> &#123;<br />           <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_none">option::spec_none</a>()<br />       &#125;<br />   &#125; <b>else</b> &#123;<br />       <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_none">option::spec_none</a>()<br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_coin_spec_is_account_registered"></a>


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_spec_is_account_registered">spec_is_account_registered</a>&lt;CoinType&gt;(account_addr: <b>address</b>): bool &#123;<br />   <b>let</b> paired_metadata_opt &#61; <a href="coin.md#0x1_coin_spec_paired_metadata">spec_paired_metadata</a>&lt;CoinType&gt;();<br />   <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr) &#124;&#124; (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<br />       paired_metadata_opt<br />   ) &amp;&amp; <a href="primary_fungible_store.md#0x1_primary_fungible_store_spec_primary_store_exists">primary_fungible_store::spec_primary_store_exists</a>(account_addr, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(paired_metadata_opt)))<br />&#125;<br /></code></pre>




<a id="0x1_coin_CoinSubAbortsIf"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_CoinSubAbortsIf">CoinSubAbortsIf</a>&lt;CoinType&gt; &#123;<br />amount: u64;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>let</b> maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>include</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<br />    maybe_supply<br />)) &#61;&#61;&gt; <a href="optional_aggregator.md#0x1_optional_aggregator_SubAbortsIf">optional_aggregator::SubAbortsIf</a> &#123; <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(maybe_supply), value: amount &#125;;<br />&#125;<br /></code></pre>




<a id="0x1_coin_CoinAddAbortsIf"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_CoinAddAbortsIf">CoinAddAbortsIf</a>&lt;CoinType&gt; &#123;<br />amount: u64;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>let</b> maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>include</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<br />    maybe_supply<br />)) &#61;&#61;&gt; <a href="optional_aggregator.md#0x1_optional_aggregator_AddAbortsIf">optional_aggregator::AddAbortsIf</a> &#123; <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(maybe_supply), value: amount &#125;;<br />&#125;<br /></code></pre>




<a id="0x1_coin_AbortsIfNotExistCoinInfo"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_AbortsIfNotExistCoinInfo">AbortsIfNotExistCoinInfo</a>&lt;CoinType&gt; &#123;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br />&#125;<br /></code></pre>



<a id="@Specification_1_name"></a>

### Function `name`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_name">name</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-7.3" href="#high-level-req">high&#45;level requirement 7</a>:
<b>include</b> <a href="coin.md#0x1_coin_AbortsIfNotExistCoinInfo">AbortsIfNotExistCoinInfo</a>&lt;CoinType&gt;;<br /></code></pre>



<a id="@Specification_1_symbol"></a>

### Function `symbol`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_symbol">symbol</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-7.4" href="#high-level-req">high&#45;level requirement 7</a>:
<b>include</b> <a href="coin.md#0x1_coin_AbortsIfNotExistCoinInfo">AbortsIfNotExistCoinInfo</a>&lt;CoinType&gt;;<br /></code></pre>



<a id="@Specification_1_decimals"></a>

### Function `decimals`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_decimals">decimals</a>&lt;CoinType&gt;(): u8<br /></code></pre>




<pre><code><b>include</b> <a href="coin.md#0x1_coin_AbortsIfNotExistCoinInfo">AbortsIfNotExistCoinInfo</a>&lt;CoinType&gt;;<br /></code></pre>



<a id="@Specification_1_supply"></a>

### Function `supply`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_coin_supply"></a>

### Function `coin_supply`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_coin_supply">coin_supply</a>&lt;CoinType&gt;(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>




<pre><code><b>let</b> coin_addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br />// This enforces <a id="high-level-req-7.5" href="#high-level-req">high&#45;level requirement 7</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(coin_addr);<br /><b>let</b> maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(coin_addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_supply);<br /><b>let</b> value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br /><b>ensures</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_supply)) &#123;<br />    result &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_some">option::spec_some</a>(value)<br />&#125; <b>else</b> &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(result)<br />&#125;;<br /></code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_burn">burn</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;, _cap: &amp;<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>include</b> <a href="coin.md#0x1_coin_AbortsIfNotExistCoinInfo">AbortsIfNotExistCoinInfo</a>&lt;CoinType&gt;;<br /><b>aborts_if</b> <a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; 0;<br /><b>include</b> <a href="coin.md#0x1_coin_CoinSubAbortsIf">CoinSubAbortsIf</a>&lt;CoinType&gt; &#123; amount: <a href="coin.md#0x1_coin">coin</a>.value &#125;;<br /><b>ensures</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;) &#45; <a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_burn_from"></a>

### Function `burn_from`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_burn_from">burn_from</a>&lt;CoinType&gt;(account_addr: <b>address</b>, amount: u64, burn_cap: &amp;<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> <b>post</b> post_coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />// This enforces <a id="high-level-req-6.2" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> amount !&#61; 0 &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>aborts_if</b> amount !&#61; 0 &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> coin_store.<a href="coin.md#0x1_coin">coin</a>.<a href="coin.md#0x1_coin_value">value</a> &lt; amount;<br /><b>let</b> maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>let</b> supply_aggr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_supply);<br /><b>let</b> value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(supply_aggr);<br /><b>let</b> <b>post</b> post_maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>let</b> <b>post</b> post_supply &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(post_maybe_supply);<br /><b>let</b> <b>post</b> post_value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(post_supply);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_supply) &amp;&amp; <a href="coin.md#0x1_coin_value">value</a> &lt; amount;<br /><b>ensures</b> post_coin_store.<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; coin_store.<a href="coin.md#0x1_coin">coin</a>.value &#45; amount;<br />// This enforces <a id="high-level-req-5" href="managed_coin.md#high-level-req">high&#45;level requirement 5</a> of the <a href="managed_coin.md">managed_coin</a> module:
<b>ensures</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_supply)) &#123;<br />    post_value &#61;&#61; value &#45; amount<br />&#125; <b>else</b> &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(post_maybe_supply)<br />&#125;;<br /><b>ensures</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;) &#45; amount;<br /></code></pre>



<a id="@Specification_1_deposit"></a>

### Function `deposit`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_deposit">deposit</a>&lt;CoinType&gt;(account_addr: <b>address</b>, <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>


<code>account_addr</code> is not frozen.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br />// This enforces <a id="high-level-req-8.3" href="#high-level-req">high&#45;level requirement 8</a>:
<b>include</b> <a href="coin.md#0x1_coin_DepositAbortsIf">DepositAbortsIf</a>&lt;CoinType&gt;;<br /><b>ensures</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr).<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; <b>old</b>(<br />    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr)<br />).<a href="coin.md#0x1_coin">coin</a>.value &#43; <a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_force_deposit"></a>

### Function `force_deposit`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_force_deposit">force_deposit</a>&lt;CoinType&gt;(account_addr: <b>address</b>, <a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>ensures</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr).<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; <b>old</b>(<br />    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr)<br />).<a href="coin.md#0x1_coin">coin</a>.value &#43; <a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_destroy_zero"></a>

### Function `destroy_zero`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_destroy_zero">destroy_zero</a>&lt;CoinType&gt;(zero_coin: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>


The value of <code>zero_coin</code> must be 0.


<pre><code><b>aborts_if</b> zero_coin.value &gt; 0;<br /></code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_extract">extract</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;, amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="coin.md#0x1_coin">coin</a>.<a href="coin.md#0x1_coin_value">value</a> &lt; amount;<br /><b>ensures</b> result.value &#61;&#61; amount;<br /><b>ensures</b> <a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin">coin</a>.value) &#45; amount;<br /></code></pre>



<a id="@Specification_1_extract_all"></a>

### Function `extract_all`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_extract_all">extract_all</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>ensures</b> result.value &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin">coin</a>).value;<br /><b>ensures</b> <a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_freeze_coin_store"></a>

### Function `freeze_coin_store`


<pre><code>&#35;[legacy_entry_fun]<br /><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_freeze_coin_store">freeze_coin_store</a>&lt;CoinType&gt;(account_addr: <b>address</b>, _freeze_cap: &amp;<a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />// This enforces <a id="high-level-req-6.3" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> <b>post</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>ensures</b> coin_store.frozen;<br /></code></pre>



<a id="@Specification_1_unfreeze_coin_store"></a>

### Function `unfreeze_coin_store`


<pre><code>&#35;[legacy_entry_fun]<br /><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_unfreeze_coin_store">unfreeze_coin_store</a>&lt;CoinType&gt;(account_addr: <b>address</b>, _freeze_cap: &amp;<a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />// This enforces <a id="high-level-req-6.4" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> <b>post</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>ensures</b> !coin_store.frozen;<br /></code></pre>



<a id="@Specification_1_upgrade_supply"></a>

### Function `upgrade_supply`


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_upgrade_supply">upgrade_supply</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


The creator of <code>CoinType</code> must be <code>@aptos_framework</code>.
<code><a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a></code> allow upgrade.


<pre><code><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> coin_address &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>aborts_if</b> coin_address !&#61; account_addr;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-1.1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> supply_config &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">SupplyConfig</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !supply_config.allow_upgrades;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>let</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_supply);<br /><b>let</b> value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br /><b>let</b> <b>post</b> post_maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr).<a href="coin.md#0x1_coin_supply">supply</a>;<br /><b>let</b> <b>post</b> post_supply &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(post_maybe_supply);<br /><b>let</b> <b>post</b> post_value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(post_supply);<br /><b>let</b> supply_no_parallel &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_supply) &amp;&amp;<br />    !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">optional_aggregator::is_parallelizable</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br /><b>aborts_if</b> supply_no_parallel &amp;&amp; !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br /><b>ensures</b> supply_no_parallel &#61;&#61;&gt;<br />    <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">optional_aggregator::is_parallelizable</a>(post_supply) &amp;&amp; post_value &#61;&#61; value;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_initialize">initialize</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, monitor_supply: bool): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />// This enforces <a id="high-level-req-1.2" href="#high-level-req">high&#45;level requirement 1</a>:
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address !&#61; account_addr;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(name) &gt; <a href="coin.md#0x1_coin_MAX_COIN_NAME_LENGTH">MAX_COIN_NAME_LENGTH</a>;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(symbol) &gt; <a href="coin.md#0x1_coin_MAX_COIN_SYMBOL_LENGTH">MAX_COIN_SYMBOL_LENGTH</a>;<br /></code></pre>



<a id="@Specification_1_initialize_with_parallelizable_supply"></a>

### Function `initialize_with_parallelizable_supply`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="coin.md#0x1_coin_initialize_with_parallelizable_supply">initialize_with_parallelizable_supply</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, monitor_supply: bool): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> monitor_supply &amp;&amp; !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="coin.md#0x1_coin_InitializeInternalSchema">InitializeInternalSchema</a>&lt;CoinType&gt; &#123;<br />    name: name.bytes,<br />    symbol: symbol.bytes<br />&#125;;<br /><b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /></code></pre>


Make sure <code>name</code> and <code>symbol</code> are legal length.
Only the creator of <code>CoinType</code> can initialize.


<a id="0x1_coin_InitializeInternalSchema"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_InitializeInternalSchema">InitializeInternalSchema</a>&lt;CoinType&gt; &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> coin_address &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>aborts_if</b> coin_address !&#61; account_addr;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> len(name) &gt; <a href="coin.md#0x1_coin_MAX_COIN_NAME_LENGTH">MAX_COIN_NAME_LENGTH</a>;<br /><b>aborts_if</b> len(symbol) &gt; <a href="coin.md#0x1_coin_MAX_COIN_SYMBOL_LENGTH">MAX_COIN_SYMBOL_LENGTH</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_initialize_internal"></a>

### Function `initialize_internal`


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_initialize_internal">initialize_internal</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, monitor_supply: bool, parallelizable: bool): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>include</b> <a href="coin.md#0x1_coin_InitializeInternalSchema">InitializeInternalSchema</a>&lt;CoinType&gt; &#123;<br />    name: name.bytes,<br />    symbol: symbol.bytes<br />&#125;;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> <b>post</b> coin_info &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> <b>post</b> <a href="coin.md#0x1_coin_supply">supply</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(coin_info.<a href="coin.md#0x1_coin_supply">supply</a>);<br /><b>let</b> <b>post</b> value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br /><b>let</b> <b>post</b> limit &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_limit">optional_aggregator::optional_aggregator_limit</a>(<a href="coin.md#0x1_coin_supply">supply</a>);<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> monitor_supply &amp;&amp; parallelizable<br />    &amp;&amp; !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-2" href="managed_coin.md#high-level-req">high&#45;level requirement 2</a> of the <a href="managed_coin.md">managed_coin</a> module:
<b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(account_addr)<br />    &amp;&amp; coin_info.name &#61;&#61; name<br />    &amp;&amp; coin_info.symbol &#61;&#61; symbol<br />    &amp;&amp; coin_info.decimals &#61;&#61; decimals;<br /><b>ensures</b> <b>if</b> (monitor_supply) &#123;<br />    value &#61;&#61; 0 &amp;&amp; limit &#61;&#61; <a href="coin.md#0x1_coin_MAX_U128">MAX_U128</a><br />        &amp;&amp; (parallelizable &#61;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">optional_aggregator::is_parallelizable</a>(<a href="coin.md#0x1_coin_supply">supply</a>))<br />&#125; <b>else</b> &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(coin_info.<a href="coin.md#0x1_coin_supply">supply</a>)<br />&#125;;<br /><b>ensures</b> result_1 &#61;&#61; <a href="coin.md#0x1_coin_BurnCapability">BurnCapability</a>&lt;CoinType&gt; &#123;&#125;;<br /><b>ensures</b> result_2 &#61;&#61; <a href="coin.md#0x1_coin_FreezeCapability">FreezeCapability</a>&lt;CoinType&gt; &#123;&#125;;<br /><b>ensures</b> result_3 &#61;&#61; <a href="coin.md#0x1_coin_MintCapability">MintCapability</a>&lt;CoinType&gt; &#123;&#125;;<br /></code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_merge">merge</a>&lt;CoinType&gt;(dst_coin: &amp;<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;, source_coin: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> dst_coin.value &#61;&#61; <b>old</b>(dst_coin.value) &#43; source_coin.value;<br /></code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_mint">mint</a>&lt;CoinType&gt;(amount: u64, _cap: &amp;<a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /></code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_register">register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


An account can only be registered once.
Updating <code>Account.guid_creation_num</code> will not overflow.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="coin.md#0x1_coin_transfer">transfer</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>


<code>from</code> and <code><b>to</b></code> account not frozen.
<code>from</code> and <code><b>to</b></code> not the same address.
<code>from</code> account sufficient balance.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr_from &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);<br /><b>let</b> coin_store_from &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_from);<br /><b>let</b> <b>post</b> coin_store_post_from &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_from);<br /><b>let</b> coin_store_to &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br /><b>let</b> <b>post</b> coin_store_post_to &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br />// This enforces <a id="high-level-req-6.5" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_from);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br />// This enforces <a id="high-level-req-8.2" href="#high-level-req">high&#45;level requirement 8</a>:
<b>aborts_if</b> coin_store_from.frozen;<br /><b>aborts_if</b> coin_store_to.frozen;<br /><b>aborts_if</b> coin_store_from.<a href="coin.md#0x1_coin">coin</a>.<a href="coin.md#0x1_coin_value">value</a> &lt; amount;<br /><b>ensures</b> account_addr_from !&#61; <b>to</b> &#61;&#61;&gt; coin_store_post_from.<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61;<br />    coin_store_from.<a href="coin.md#0x1_coin">coin</a>.value &#45; amount;<br /><b>ensures</b> account_addr_from !&#61; <b>to</b> &#61;&#61;&gt; coin_store_post_to.<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; coin_store_to.<a href="coin.md#0x1_coin">coin</a>.value &#43; amount;<br /><b>ensures</b> account_addr_from &#61;&#61; <b>to</b> &#61;&#61;&gt; coin_store_post_from.<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; coin_store_from.<a href="coin.md#0x1_coin">coin</a>.value;<br /></code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code><b>public</b> <b>fun</b> <a href="coin.md#0x1_coin_withdraw">withdraw</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>


Account is not frozen and sufficient balance.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="coin.md#0x1_coin_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt;;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> balance &#61; coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>let</b> <b>post</b> coin_post &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr).<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>ensures</b> coin_post &#61;&#61; balance &#45; amount;<br /><b>ensures</b> result &#61;&#61; <a href="coin.md#0x1_coin_Coin">Coin</a>&lt;CoinType&gt; &#123; value: amount &#125;;<br /></code></pre>




<a id="0x1_coin_WithdrawAbortsIf"></a>


<pre><code><b>schema</b> <a href="coin.md#0x1_coin_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt; &#123;<br /><a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />amount: u64;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> balance &#61; coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br />// This enforces <a id="high-level-req-6.6" href="#high-level-req">high&#45;level requirement 6</a>:
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br />// This enforces <a id="high-level-req-8.1" href="#high-level-req">high&#45;level requirement 8</a>:
    <b>aborts_if</b> coin_store.frozen;<br /><b>aborts_if</b> <a href="coin.md#0x1_coin_balance">balance</a> &lt; amount;<br />&#125;<br /></code></pre>



<a id="@Specification_1_mint_internal"></a>

### Function `mint_internal`


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_mint_internal">mint_internal</a>&lt;CoinType&gt;(amount: u64): <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>aborts_if</b> (amount !&#61; 0) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>ensures</b> <a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt; &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin_supply">supply</a>&lt;CoinType&gt;) &#43; amount;<br /><b>ensures</b> result.value &#61;&#61; amount;<br /></code></pre>



<a id="@Specification_1_burn_internal"></a>

### Function `burn_internal`


<pre><code><b>fun</b> <a href="coin.md#0x1_coin_burn_internal">burn_internal</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): u64<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>modifies</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
