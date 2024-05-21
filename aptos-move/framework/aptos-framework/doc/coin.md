
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


<pre><code>use 0x1::account;<br/>use 0x1::aggregator;<br/>use 0x1::aggregator_factory;<br/>use 0x1::create_signer;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::fungible_asset;<br/>use 0x1::guid;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::optional_aggregator;<br/>use 0x1::primary_fungible_store;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::table;<br/>use 0x1::type_info;<br/></code></pre>



<a id="0x1_coin_Coin"></a>

## Struct `Coin`

Core data structures
Main structure representing a coin/token in an account's custody.


<pre><code>struct Coin&lt;CoinType&gt; has store<br/></code></pre>



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
the coin in every transaction avoiding read-modify-write conflicts. Only
used for gas fees distribution by Aptos Framework (0x1).


<pre><code>struct AggregatableCoin&lt;CoinType&gt; has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: aggregator::Aggregator</code>
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


<pre><code>struct CoinStore&lt;CoinType&gt; has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin: coin::Coin&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>frozen: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_events: event::EventHandle&lt;coin::DepositEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: event::EventHandle&lt;coin::WithdrawEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_SupplyConfig"></a>

## Resource `SupplyConfig`

Configuration that controls the behavior of total coin supply. If the field
is set, coin creators are allowed to upgrade to parallelizable implementations.


<pre><code>struct SupplyConfig has key<br/></code></pre>



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

Information about a specific coin type. Stored on the creator of the coin's account.


<pre><code>struct CoinInfo&lt;CoinType&gt; has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>symbol: string::String</code>
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
<code>supply: option::Option&lt;optional_aggregator::OptionalAggregator&gt;</code>
</dt>
<dd>
 Amount of this coin type in existence.
</dd>
</dl>


</details>

<a id="0x1_coin_DepositEvent"></a>

## Struct `DepositEvent`

Event emitted when some amount of a coin is deposited into an account.


<pre><code>struct DepositEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct Deposit&lt;CoinType&gt; has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
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


<pre><code>struct WithdrawEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct Withdraw&lt;CoinType&gt; has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
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


<pre><code>&#35;[event]<br/>struct CoinEventHandleDeletion has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>event_handle_creation_address: address</code>
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


<pre><code>&#35;[event]<br/>struct PairCreation has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin_type: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
<dt>
<code>fungible_asset_metadata_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_MigrationFlag"></a>

## Resource `MigrationFlag`

The flag the existence of which indicates the primary fungible store is created by the migration from CoinStore.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct MigrationFlag has key<br/></code></pre>



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


<pre><code>struct MintCapability&lt;CoinType&gt; has copy, store<br/></code></pre>



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


<pre><code>struct FreezeCapability&lt;CoinType&gt; has copy, store<br/></code></pre>



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


<pre><code>struct BurnCapability&lt;CoinType&gt; has copy, store<br/></code></pre>



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


<pre><code>struct CoinConversionMap has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin_to_fungible_asset_map: table::Table&lt;type_info::TypeInfo, object::Object&lt;fungible_asset::Metadata&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_PairedCoinType"></a>

## Resource `PairedCoinType`

The paired coin type info stored in fungible asset metadata object.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct PairedCoinType has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_PairedFungibleAssetRefs"></a>

## Resource `PairedFungibleAssetRefs`

The refs of the paired fungible asset.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct PairedFungibleAssetRefs has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_ref_opt: option::Option&lt;fungible_asset::MintRef&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transfer_ref_opt: option::Option&lt;fungible_asset::TransferRef&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_ref_opt: option::Option&lt;fungible_asset::BurnRef&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_MintRefReceipt"></a>

## Struct `MintRefReceipt`

The hot potato receipt for flash borrowing MintRef.


<pre><code>struct MintRefReceipt<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: object::Object&lt;fungible_asset::Metadata&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_TransferRefReceipt"></a>

## Struct `TransferRefReceipt`

The hot potato receipt for flash borrowing TransferRef.


<pre><code>struct TransferRefReceipt<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: object::Object&lt;fungible_asset::Metadata&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_BurnRefReceipt"></a>

## Struct `BurnRefReceipt`

The hot potato receipt for flash borrowing BurnRef.


<pre><code>struct BurnRefReceipt<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: object::Object&lt;fungible_asset::Metadata&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_coin_Ghost$supply"></a>

## Resource `Ghost$supply`



<pre><code>struct Ghost$supply&lt;CoinType&gt; has copy, drop, store, key<br/></code></pre>



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



<pre><code>struct Ghost$aggregate_supply&lt;CoinType&gt; has copy, drop, store, key<br/></code></pre>



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


<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_coin_MAX_U128"></a>

Maximum possible coin supply.


<pre><code>const MAX_U128: u128 &#61; 340282366920938463463374607431768211455;<br/></code></pre>



<a id="0x1_coin_EINSUFFICIENT_BALANCE"></a>

Not enough coins to complete transaction


<pre><code>const EINSUFFICIENT_BALANCE: u64 &#61; 6;<br/></code></pre>



<a id="0x1_coin_EAGGREGATABLE_COIN_VALUE_TOO_LARGE"></a>

The value of aggregatable coin used for transaction fees redistribution does not fit in u64.


<pre><code>const EAGGREGATABLE_COIN_VALUE_TOO_LARGE: u64 &#61; 14;<br/></code></pre>



<a id="0x1_coin_EAPT_PAIRING_IS_NOT_ENABLED"></a>

APT pairing is not eanbled yet.


<pre><code>const EAPT_PAIRING_IS_NOT_ENABLED: u64 &#61; 28;<br/></code></pre>



<a id="0x1_coin_EBURN_REF_NOT_FOUND"></a>

The BurnRef does not exist.


<pre><code>const EBURN_REF_NOT_FOUND: u64 &#61; 25;<br/></code></pre>



<a id="0x1_coin_EBURN_REF_RECEIPT_MISMATCH"></a>

The BurnRefReceipt does not match the BurnRef to be returned.


<pre><code>const EBURN_REF_RECEIPT_MISMATCH: u64 &#61; 24;<br/></code></pre>



<a id="0x1_coin_ECOIN_CONVERSION_MAP_NOT_FOUND"></a>

The coin converison map is not created yet.


<pre><code>const ECOIN_CONVERSION_MAP_NOT_FOUND: u64 &#61; 27;<br/></code></pre>



<a id="0x1_coin_ECOIN_INFO_ADDRESS_MISMATCH"></a>

Address of account which is used to initialize a coin <code>CoinType</code> doesn't match the deployer of module


<pre><code>const ECOIN_INFO_ADDRESS_MISMATCH: u64 &#61; 1;<br/></code></pre>



<a id="0x1_coin_ECOIN_INFO_ALREADY_PUBLISHED"></a>

<code>CoinType</code> is already initialized as a coin


<pre><code>const ECOIN_INFO_ALREADY_PUBLISHED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_coin_ECOIN_INFO_NOT_PUBLISHED"></a>

<code>CoinType</code> hasn't been initialized as a coin


<pre><code>const ECOIN_INFO_NOT_PUBLISHED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_coin_ECOIN_NAME_TOO_LONG"></a>

Name of the coin is too long


<pre><code>const ECOIN_NAME_TOO_LONG: u64 &#61; 12;<br/></code></pre>



<a id="0x1_coin_ECOIN_STORE_ALREADY_PUBLISHED"></a>

Deprecated. Account already has <code>CoinStore</code> registered for <code>CoinType</code>


<pre><code>const ECOIN_STORE_ALREADY_PUBLISHED: u64 &#61; 4;<br/></code></pre>



<a id="0x1_coin_ECOIN_STORE_NOT_PUBLISHED"></a>

Account hasn't registered <code>CoinStore</code> for <code>CoinType</code>


<pre><code>const ECOIN_STORE_NOT_PUBLISHED: u64 &#61; 5;<br/></code></pre>



<a id="0x1_coin_ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED"></a>

Cannot upgrade the total supply of coins to different implementation.


<pre><code>const ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED: u64 &#61; 11;<br/></code></pre>



<a id="0x1_coin_ECOIN_SYMBOL_TOO_LONG"></a>

Symbol of the coin is too long


<pre><code>const ECOIN_SYMBOL_TOO_LONG: u64 &#61; 13;<br/></code></pre>



<a id="0x1_coin_ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED"></a>

The feature of migration from coin to fungible asset is not enabled.


<pre><code>const ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED: u64 &#61; 18;<br/></code></pre>



<a id="0x1_coin_ECOIN_TYPE_MISMATCH"></a>

The coin type from the map does not match the calling function type argument.


<pre><code>const ECOIN_TYPE_MISMATCH: u64 &#61; 17;<br/></code></pre>



<a id="0x1_coin_EDESTRUCTION_OF_NONZERO_TOKEN"></a>

Cannot destroy non-zero coins


<pre><code>const EDESTRUCTION_OF_NONZERO_TOKEN: u64 &#61; 7;<br/></code></pre>



<a id="0x1_coin_EFROZEN"></a>

CoinStore is frozen. Coins cannot be deposited or withdrawn


<pre><code>const EFROZEN: u64 &#61; 10;<br/></code></pre>



<a id="0x1_coin_EMIGRATION_FRAMEWORK_NOT_ENABLED"></a>

The migration process from coin to fungible asset is not enabled yet.


<pre><code>const EMIGRATION_FRAMEWORK_NOT_ENABLED: u64 &#61; 26;<br/></code></pre>



<a id="0x1_coin_EMINT_REF_NOT_FOUND"></a>

The MintRef does not exist.


<pre><code>const EMINT_REF_NOT_FOUND: u64 &#61; 21;<br/></code></pre>



<a id="0x1_coin_EMINT_REF_RECEIPT_MISMATCH"></a>

The MintRefReceipt does not match the MintRef to be returned.


<pre><code>const EMINT_REF_RECEIPT_MISMATCH: u64 &#61; 20;<br/></code></pre>



<a id="0x1_coin_EPAIRED_COIN"></a>

Error regarding paired coin type of the fungible asset metadata.


<pre><code>const EPAIRED_COIN: u64 &#61; 15;<br/></code></pre>



<a id="0x1_coin_EPAIRED_FUNGIBLE_ASSET"></a>

Error regarding paired fungible asset metadata of a coin type.


<pre><code>const EPAIRED_FUNGIBLE_ASSET: u64 &#61; 16;<br/></code></pre>



<a id="0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND"></a>

PairedFungibleAssetRefs resource does not exist.


<pre><code>const EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND: u64 &#61; 19;<br/></code></pre>



<a id="0x1_coin_ETRANSFER_REF_NOT_FOUND"></a>

The TransferRef does not exist.


<pre><code>const ETRANSFER_REF_NOT_FOUND: u64 &#61; 23;<br/></code></pre>



<a id="0x1_coin_ETRANSFER_REF_RECEIPT_MISMATCH"></a>

The TransferRefReceipt does not match the TransferRef to be returned.


<pre><code>const ETRANSFER_REF_RECEIPT_MISMATCH: u64 &#61; 22;<br/></code></pre>



<a id="0x1_coin_MAX_COIN_NAME_LENGTH"></a>



<pre><code>const MAX_COIN_NAME_LENGTH: u64 &#61; 32;<br/></code></pre>



<a id="0x1_coin_MAX_COIN_SYMBOL_LENGTH"></a>



<pre><code>const MAX_COIN_SYMBOL_LENGTH: u64 &#61; 10;<br/></code></pre>



<a id="0x1_coin_paired_metadata"></a>

## Function `paired_metadata`

Get the paired fungible asset metadata object of a coin type. If not exist, return option::none().


<pre><code>&#35;[view]<br/>public fun paired_metadata&lt;CoinType&gt;(): option::Option&lt;object::Object&lt;fungible_asset::Metadata&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_metadata&lt;CoinType&gt;(): Option&lt;Object&lt;Metadata&gt;&gt; acquires CoinConversionMap &#123;<br/>    if (exists&lt;CoinConversionMap&gt;(@aptos_framework) &amp;&amp; features::coin_to_fungible_asset_migration_feature_enabled(<br/>    )) &#123;<br/>        let map &#61; &amp;borrow_global&lt;CoinConversionMap&gt;(@aptos_framework).coin_to_fungible_asset_map;<br/>        let type &#61; type_info::type_of&lt;CoinType&gt;();<br/>        if (table::contains(map, type)) &#123;<br/>            return option::some(&#42;table::borrow(map, type))<br/>        &#125;<br/>    &#125;;<br/>    option::none()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_create_coin_conversion_map"></a>

## Function `create_coin_conversion_map`



<pre><code>public entry fun create_coin_conversion_map(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_coin_conversion_map(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    if (!exists&lt;CoinConversionMap&gt;(@aptos_framework)) &#123;<br/>        move_to(aptos_framework, CoinConversionMap &#123;<br/>            coin_to_fungible_asset_map: table::new(),<br/>        &#125;)<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_create_pairing"></a>

## Function `create_pairing`

Create APT pairing by passing <code>AptosCoin</code>.


<pre><code>public entry fun create_pairing&lt;CoinType&gt;(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_pairing&lt;CoinType&gt;(<br/>    aptos_framework: &amp;signer<br/>) acquires CoinConversionMap, CoinInfo &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(true);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_is_apt"></a>

## Function `is_apt`



<pre><code>fun is_apt&lt;CoinType&gt;(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun is_apt&lt;CoinType&gt;(): bool &#123;<br/>    type_info::type_name&lt;CoinType&gt;() &#61;&#61; string::utf8(b&quot;0x1::aptos_coin::AptosCoin&quot;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_create_and_return_paired_metadata_if_not_exist"></a>

## Function `create_and_return_paired_metadata_if_not_exist`



<pre><code>fun create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(allow_apt_creation: bool): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(allow_apt_creation: bool): Object&lt;Metadata&gt; &#123;<br/>    assert!(<br/>        features::coin_to_fungible_asset_migration_feature_enabled(),<br/>        error::invalid_state(EMIGRATION_FRAMEWORK_NOT_ENABLED)<br/>    );<br/>    assert!(exists&lt;CoinConversionMap&gt;(@aptos_framework), error::not_found(ECOIN_CONVERSION_MAP_NOT_FOUND));<br/>    let map &#61; borrow_global_mut&lt;CoinConversionMap&gt;(@aptos_framework);<br/>    let type &#61; type_info::type_of&lt;CoinType&gt;();<br/>    if (!table::contains(&amp;map.coin_to_fungible_asset_map, type)) &#123;<br/>        let is_apt &#61; is_apt&lt;CoinType&gt;();<br/>        assert!(!is_apt &#124;&#124; allow_apt_creation, error::invalid_state(EAPT_PAIRING_IS_NOT_ENABLED));<br/>        let metadata_object_cref &#61;<br/>            if (is_apt) &#123;<br/>                object::create_sticky_object_at_address(@aptos_framework, @aptos_fungible_asset)<br/>            &#125; else &#123;<br/>                object::create_named_object(<br/>                    &amp;create_signer::create_signer(@aptos_fungible_asset),<br/>                    &#42;string::bytes(&amp;type_info::type_name&lt;CoinType&gt;())<br/>                )<br/>            &#125;;<br/>        primary_fungible_store::create_primary_store_enabled_fungible_asset(<br/>            &amp;metadata_object_cref,<br/>            option::map(coin_supply&lt;CoinType&gt;(), &#124;_&#124; MAX_U128),<br/>            name&lt;CoinType&gt;(),<br/>            symbol&lt;CoinType&gt;(),<br/>            decimals&lt;CoinType&gt;(),<br/>            string::utf8(b&quot;&quot;),<br/>            string::utf8(b&quot;&quot;),<br/>        );<br/><br/>        let metadata_object_signer &#61; &amp;object::generate_signer(&amp;metadata_object_cref);<br/>        let type &#61; type_info::type_of&lt;CoinType&gt;();<br/>        move_to(metadata_object_signer, PairedCoinType &#123; type &#125;);<br/>        let metadata_obj &#61; object::object_from_constructor_ref(&amp;metadata_object_cref);<br/><br/>        table::add(&amp;mut map.coin_to_fungible_asset_map, type, metadata_obj);<br/>        event::emit(PairCreation &#123;<br/>            coin_type: type,<br/>            fungible_asset_metadata_address: object_address(&amp;metadata_obj)<br/>        &#125;);<br/><br/>        // Generates all three refs<br/>        let mint_ref &#61; fungible_asset::generate_mint_ref(&amp;metadata_object_cref);<br/>        let transfer_ref &#61; fungible_asset::generate_transfer_ref(&amp;metadata_object_cref);<br/>        let burn_ref &#61; fungible_asset::generate_burn_ref(&amp;metadata_object_cref);<br/>        move_to(metadata_object_signer,<br/>            PairedFungibleAssetRefs &#123;<br/>                mint_ref_opt: option::some(mint_ref),<br/>                transfer_ref_opt: option::some(transfer_ref),<br/>                burn_ref_opt: option::some(burn_ref),<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    &#42;table::borrow(&amp;map.coin_to_fungible_asset_map, type)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_ensure_paired_metadata"></a>

## Function `ensure_paired_metadata`

Get the paired fungible asset metadata object of a coin type, create if not exist.


<pre><code>public(friend) fun ensure_paired_metadata&lt;CoinType&gt;(): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun ensure_paired_metadata&lt;CoinType&gt;(): Object&lt;Metadata&gt; acquires CoinConversionMap, CoinInfo &#123;<br/>    create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_paired_coin"></a>

## Function `paired_coin`

Get the paired coin type of a fungible asset metadata object.


<pre><code>&#35;[view]<br/>public fun paired_coin(metadata: object::Object&lt;fungible_asset::Metadata&gt;): option::Option&lt;type_info::TypeInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_coin(metadata: Object&lt;Metadata&gt;): Option&lt;TypeInfo&gt; acquires PairedCoinType &#123;<br/>    let metadata_addr &#61; object::object_address(&amp;metadata);<br/>    if (exists&lt;PairedCoinType&gt;(metadata_addr)) &#123;<br/>        option::some(borrow_global&lt;PairedCoinType&gt;(metadata_addr).type)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_coin_to_fungible_asset"></a>

## Function `coin_to_fungible_asset`

Conversion from coin to fungible asset


<pre><code>public fun coin_to_fungible_asset&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun coin_to_fungible_asset&lt;CoinType&gt;(<br/>    coin: Coin&lt;CoinType&gt;<br/>): FungibleAsset acquires CoinConversionMap, CoinInfo &#123;<br/>    let metadata &#61; ensure_paired_metadata&lt;CoinType&gt;();<br/>    let amount &#61; burn_internal(coin);<br/>    fungible_asset::mint_internal(metadata, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_fungible_asset_to_coin"></a>

## Function `fungible_asset_to_coin`

Conversion from fungible asset to coin. Not public to push the migration to FA.


<pre><code>fun fungible_asset_to_coin&lt;CoinType&gt;(fungible_asset: fungible_asset::FungibleAsset): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun fungible_asset_to_coin&lt;CoinType&gt;(<br/>    fungible_asset: FungibleAsset<br/>): Coin&lt;CoinType&gt; acquires CoinInfo, PairedCoinType &#123;<br/>    let metadata_addr &#61; object::object_address(&amp;fungible_asset::metadata_from_asset(&amp;fungible_asset));<br/>    assert!(<br/>        object::object_exists&lt;PairedCoinType&gt;(metadata_addr),<br/>        error::not_found(EPAIRED_COIN)<br/>    );<br/>    let coin_type_info &#61; borrow_global&lt;PairedCoinType&gt;(metadata_addr).type;<br/>    assert!(coin_type_info &#61;&#61; type_info::type_of&lt;CoinType&gt;(), error::invalid_argument(ECOIN_TYPE_MISMATCH));<br/>    let amount &#61; fungible_asset::burn_internal(fungible_asset);<br/>    mint_internal&lt;CoinType&gt;(amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_assert_paired_metadata_exists"></a>

## Function `assert_paired_metadata_exists`



<pre><code>fun assert_paired_metadata_exists&lt;CoinType&gt;(): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_paired_metadata_exists&lt;CoinType&gt;(): Object&lt;Metadata&gt; &#123;<br/>    let metadata_opt &#61; paired_metadata&lt;CoinType&gt;();<br/>    assert!(option::is_some(&amp;metadata_opt), error::not_found(EPAIRED_FUNGIBLE_ASSET));<br/>    option::destroy_some(metadata_opt)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_paired_mint_ref_exists"></a>

## Function `paired_mint_ref_exists`

Check whether <code>MintRef</code> has not been taken.


<pre><code>&#35;[view]<br/>public fun paired_mint_ref_exists&lt;CoinType&gt;(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_mint_ref_exists&lt;CoinType&gt;(): bool acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    option::is_some(&amp;borrow_global&lt;PairedFungibleAssetRefs&gt;(metadata_addr).mint_ref_opt)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_get_paired_mint_ref"></a>

## Function `get_paired_mint_ref`

Get the <code>MintRef</code> of paired fungible asset of a coin type from <code>MintCapability</code>.


<pre><code>public fun get_paired_mint_ref&lt;CoinType&gt;(_: &amp;coin::MintCapability&lt;CoinType&gt;): (fungible_asset::MintRef, coin::MintRefReceipt)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_paired_mint_ref&lt;CoinType&gt;(<br/>    _: &amp;MintCapability&lt;CoinType&gt;<br/>): (MintRef, MintRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    let mint_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).mint_ref_opt;<br/>    assert!(option::is_some(mint_ref_opt), error::not_found(EMINT_REF_NOT_FOUND));<br/>    (option::extract(mint_ref_opt), MintRefReceipt &#123; metadata &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_return_paired_mint_ref"></a>

## Function `return_paired_mint_ref`

Return the <code>MintRef</code> with the hot potato receipt.


<pre><code>public fun return_paired_mint_ref(mint_ref: fungible_asset::MintRef, receipt: coin::MintRefReceipt)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun return_paired_mint_ref(mint_ref: MintRef, receipt: MintRefReceipt) acquires PairedFungibleAssetRefs &#123;<br/>    let MintRefReceipt &#123; metadata &#125; &#61; receipt;<br/>    assert!(<br/>        fungible_asset::mint_ref_metadata(&amp;mint_ref) &#61;&#61; metadata,<br/>        error::invalid_argument(EMINT_REF_RECEIPT_MISMATCH)<br/>    );<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    let mint_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).mint_ref_opt;<br/>    option::fill(mint_ref_opt, mint_ref);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_paired_transfer_ref_exists"></a>

## Function `paired_transfer_ref_exists`

Check whether <code>TransferRef</code> still exists.


<pre><code>&#35;[view]<br/>public fun paired_transfer_ref_exists&lt;CoinType&gt;(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_transfer_ref_exists&lt;CoinType&gt;(): bool acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    option::is_some(&amp;borrow_global&lt;PairedFungibleAssetRefs&gt;(metadata_addr).transfer_ref_opt)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_get_paired_transfer_ref"></a>

## Function `get_paired_transfer_ref`

Get the TransferRef of paired fungible asset of a coin type from <code>FreezeCapability</code>.


<pre><code>public fun get_paired_transfer_ref&lt;CoinType&gt;(_: &amp;coin::FreezeCapability&lt;CoinType&gt;): (fungible_asset::TransferRef, coin::TransferRefReceipt)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_paired_transfer_ref&lt;CoinType&gt;(<br/>    _: &amp;FreezeCapability&lt;CoinType&gt;<br/>): (TransferRef, TransferRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    let transfer_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).transfer_ref_opt;<br/>    assert!(option::is_some(transfer_ref_opt), error::not_found(ETRANSFER_REF_NOT_FOUND));<br/>    (option::extract(transfer_ref_opt), TransferRefReceipt &#123; metadata &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_return_paired_transfer_ref"></a>

## Function `return_paired_transfer_ref`

Return the <code>TransferRef</code> with the hot potato receipt.


<pre><code>public fun return_paired_transfer_ref(transfer_ref: fungible_asset::TransferRef, receipt: coin::TransferRefReceipt)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun return_paired_transfer_ref(<br/>    transfer_ref: TransferRef,<br/>    receipt: TransferRefReceipt<br/>) acquires PairedFungibleAssetRefs &#123;<br/>    let TransferRefReceipt &#123; metadata &#125; &#61; receipt;<br/>    assert!(<br/>        fungible_asset::transfer_ref_metadata(&amp;transfer_ref) &#61;&#61; metadata,<br/>        error::invalid_argument(ETRANSFER_REF_RECEIPT_MISMATCH)<br/>    );<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    let transfer_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).transfer_ref_opt;<br/>    option::fill(transfer_ref_opt, transfer_ref);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_paired_burn_ref_exists"></a>

## Function `paired_burn_ref_exists`

Check whether <code>BurnRef</code> has not been taken.


<pre><code>&#35;[view]<br/>public fun paired_burn_ref_exists&lt;CoinType&gt;(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_burn_ref_exists&lt;CoinType&gt;(): bool acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    option::is_some(&amp;borrow_global&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_get_paired_burn_ref"></a>

## Function `get_paired_burn_ref`

Get the <code>BurnRef</code> of paired fungible asset of a coin type from <code>BurnCapability</code>.


<pre><code>public fun get_paired_burn_ref&lt;CoinType&gt;(_: &amp;coin::BurnCapability&lt;CoinType&gt;): (fungible_asset::BurnRef, coin::BurnRefReceipt)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_paired_burn_ref&lt;CoinType&gt;(<br/>    _: &amp;BurnCapability&lt;CoinType&gt;<br/>): (BurnRef, BurnRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    let burn_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt;<br/>    assert!(option::is_some(burn_ref_opt), error::not_found(EBURN_REF_NOT_FOUND));<br/>    (option::extract(burn_ref_opt), BurnRefReceipt &#123; metadata &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_return_paired_burn_ref"></a>

## Function `return_paired_burn_ref`

Return the <code>BurnRef</code> with the hot potato receipt.


<pre><code>public fun return_paired_burn_ref(burn_ref: fungible_asset::BurnRef, receipt: coin::BurnRefReceipt)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun return_paired_burn_ref(<br/>    burn_ref: BurnRef,<br/>    receipt: BurnRefReceipt<br/>) acquires PairedFungibleAssetRefs &#123;<br/>    let BurnRefReceipt &#123; metadata &#125; &#61; receipt;<br/>    assert!(<br/>        fungible_asset::burn_ref_metadata(&amp;burn_ref) &#61;&#61; metadata,<br/>        error::invalid_argument(EBURN_REF_RECEIPT_MISMATCH)<br/>    );<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    let burn_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt;<br/>    option::fill(burn_ref_opt, burn_ref);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_borrow_paired_burn_ref"></a>

## Function `borrow_paired_burn_ref`



<pre><code>fun borrow_paired_burn_ref&lt;CoinType&gt;(_: &amp;coin::BurnCapability&lt;CoinType&gt;): &amp;fungible_asset::BurnRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_paired_burn_ref&lt;CoinType&gt;(<br/>    _: &amp;BurnCapability&lt;CoinType&gt;<br/>): &amp;BurnRef acquires CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();<br/>    let metadata_addr &#61; object_address(&amp;metadata);<br/>    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));<br/>    let burn_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt;<br/>    assert!(option::is_some(burn_ref_opt), error::not_found(EBURN_REF_NOT_FOUND));<br/>    option::borrow(burn_ref_opt)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_initialize_supply_config"></a>

## Function `initialize_supply_config`

Publishes supply configuration. Initially, upgrading is not allowed.


<pre><code>public(friend) fun initialize_supply_config(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_supply_config(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, SupplyConfig &#123; allow_upgrades: false &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_allow_supply_upgrades"></a>

## Function `allow_supply_upgrades`

This should be called by on-chain governance to update the config and allow
or disallow upgradability of total supply.


<pre><code>public fun allow_supply_upgrades(aptos_framework: &amp;signer, allowed: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun allow_supply_upgrades(aptos_framework: &amp;signer, allowed: bool) acquires SupplyConfig &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    let allow_upgrades &#61; &amp;mut borrow_global_mut&lt;SupplyConfig&gt;(@aptos_framework).allow_upgrades;<br/>    &#42;allow_upgrades &#61; allowed;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_initialize_aggregatable_coin"></a>

## Function `initialize_aggregatable_coin`

Creates a new aggregatable coin with value overflowing on <code>limit</code>. Note that this function can
only be called by Aptos Framework (0x1) account for now because of <code>create_aggregator</code>.


<pre><code>public(friend) fun initialize_aggregatable_coin&lt;CoinType&gt;(aptos_framework: &amp;signer): coin::AggregatableCoin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_aggregatable_coin&lt;CoinType&gt;(aptos_framework: &amp;signer): AggregatableCoin&lt;CoinType&gt; &#123;<br/>    let aggregator &#61; aggregator_factory::create_aggregator(aptos_framework, MAX_U64);<br/>    AggregatableCoin&lt;CoinType&gt; &#123;<br/>        value: aggregator,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_is_aggregatable_coin_zero"></a>

## Function `is_aggregatable_coin_zero`

Returns true if the value of aggregatable coin is zero.


<pre><code>public(friend) fun is_aggregatable_coin_zero&lt;CoinType&gt;(coin: &amp;coin::AggregatableCoin&lt;CoinType&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun is_aggregatable_coin_zero&lt;CoinType&gt;(coin: &amp;AggregatableCoin&lt;CoinType&gt;): bool &#123;<br/>    let amount &#61; aggregator::read(&amp;coin.value);<br/>    amount &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_drain_aggregatable_coin"></a>

## Function `drain_aggregatable_coin`

Drains the aggregatable coin, setting it to zero and returning a standard coin.


<pre><code>public(friend) fun drain_aggregatable_coin&lt;CoinType&gt;(coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun drain_aggregatable_coin&lt;CoinType&gt;(coin: &amp;mut AggregatableCoin&lt;CoinType&gt;): Coin&lt;CoinType&gt; &#123;<br/>    spec &#123;<br/>        // TODO: The data invariant is not properly assumed from CollectedFeesPerBlock.<br/>        assume aggregator::spec_get_limit(coin.value) &#61;&#61; MAX_U64;<br/>    &#125;;<br/>    let amount &#61; aggregator::read(&amp;coin.value);<br/>    assert!(amount &lt;&#61; MAX_U64, error::out_of_range(EAGGREGATABLE_COIN_VALUE_TOO_LARGE));<br/>    spec &#123;<br/>        update aggregate_supply&lt;CoinType&gt; &#61; aggregate_supply&lt;CoinType&gt; &#45; amount;<br/>    &#125;;<br/>    aggregator::sub(&amp;mut coin.value, amount);<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; amount;<br/>    &#125;;<br/>    Coin&lt;CoinType&gt; &#123;<br/>        value: (amount as u64),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_merge_aggregatable_coin"></a>

## Function `merge_aggregatable_coin`

Merges <code>coin</code> into aggregatable coin (<code>dst_coin</code>).


<pre><code>public(friend) fun merge_aggregatable_coin&lt;CoinType&gt;(dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;, coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun merge_aggregatable_coin&lt;CoinType&gt;(<br/>    dst_coin: &amp;mut AggregatableCoin&lt;CoinType&gt;,<br/>    coin: Coin&lt;CoinType&gt;<br/>) &#123;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; coin.value;<br/>    &#125;;<br/>    let Coin &#123; value &#125; &#61; coin;<br/>    let amount &#61; (value as u128);<br/>    spec &#123;<br/>        update aggregate_supply&lt;CoinType&gt; &#61; aggregate_supply&lt;CoinType&gt; &#43; amount;<br/>    &#125;;<br/>    aggregator::add(&amp;mut dst_coin.value, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_collect_into_aggregatable_coin"></a>

## Function `collect_into_aggregatable_coin`

Collects a specified amount of coin form an account into aggregatable coin.


<pre><code>public(friend) fun collect_into_aggregatable_coin&lt;CoinType&gt;(account_addr: address, amount: u64, dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun collect_into_aggregatable_coin&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    amount: u64,<br/>    dst_coin: &amp;mut AggregatableCoin&lt;CoinType&gt;,<br/>) acquires CoinStore, CoinConversionMap, CoinInfo, PairedCoinType &#123;<br/>    // Skip collecting if amount is zero.<br/>    if (amount &#61;&#61; 0) &#123;<br/>        return<br/>    &#125;;<br/><br/>    let (coin_amount_to_collect, fa_amount_to_collect) &#61; calculate_amount_to_withdraw&lt;CoinType&gt;(<br/>        account_addr,<br/>        amount<br/>    );<br/>    let coin &#61; if (coin_amount_to_collect &gt; 0) &#123;<br/>        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>        extract(&amp;mut coin_store.coin, coin_amount_to_collect)<br/>    &#125; else &#123;<br/>        zero()<br/>    &#125;;<br/>    if (fa_amount_to_collect &gt; 0) &#123;<br/>        let store_addr &#61; primary_fungible_store::primary_store_address(<br/>            account_addr,<br/>            option::destroy_some(paired_metadata&lt;CoinType&gt;())<br/>        );<br/>        let fa &#61; fungible_asset::withdraw_internal(store_addr, fa_amount_to_collect);<br/>        merge(&amp;mut coin, fungible_asset_to_coin&lt;CoinType&gt;(fa));<br/>    &#125;;<br/>    merge_aggregatable_coin(dst_coin, coin);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_calculate_amount_to_withdraw"></a>

## Function `calculate_amount_to_withdraw`



<pre><code>fun calculate_amount_to_withdraw&lt;CoinType&gt;(account_addr: address, amount: u64): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun calculate_amount_to_withdraw&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    amount: u64<br/>): (u64, u64) &#123;<br/>    let coin_balance &#61; coin_balance&lt;CoinType&gt;(account_addr);<br/>    if (coin_balance &gt;&#61; amount) &#123;<br/>        (amount, 0)<br/>    &#125; else &#123;<br/>        let metadata &#61; paired_metadata&lt;CoinType&gt;();<br/>        if (option::is_some(&amp;metadata) &amp;&amp; primary_fungible_store::primary_store_exists(<br/>            account_addr,<br/>            option::destroy_some(metadata)<br/>        ))<br/>            (coin_balance, amount &#45; coin_balance)<br/>        else<br/>            abort error::invalid_argument(EINSUFFICIENT_BALANCE)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_maybe_convert_to_fungible_store"></a>

## Function `maybe_convert_to_fungible_store`



<pre><code>fun maybe_convert_to_fungible_store&lt;CoinType&gt;(account: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun maybe_convert_to_fungible_store&lt;CoinType&gt;(account: address) acquires CoinStore, CoinConversionMap, CoinInfo &#123;<br/>    if (!features::coin_to_fungible_asset_migration_feature_enabled()) &#123;<br/>        abort error::unavailable(ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED)<br/>    &#125;;<br/>    assert!(is_coin_initialized&lt;CoinType&gt;(), error::invalid_argument(ECOIN_INFO_NOT_PUBLISHED));<br/><br/>    let metadata &#61; ensure_paired_metadata&lt;CoinType&gt;();<br/>    let store &#61; primary_fungible_store::ensure_primary_store_exists(account, metadata);<br/>    let store_address &#61; object::object_address(&amp;store);<br/>    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account)) &#123;<br/>        let CoinStore&lt;CoinType&gt; &#123; coin, frozen, deposit_events, withdraw_events &#125; &#61; move_from&lt;CoinStore&lt;CoinType&gt;&gt;(<br/>            account<br/>        );<br/>        event::emit(<br/>            CoinEventHandleDeletion &#123;<br/>                event_handle_creation_address: guid::creator_address(<br/>                    event::guid(&amp;deposit_events)<br/>                ),<br/>                deleted_deposit_event_handle_creation_number: guid::creation_num(event::guid(&amp;deposit_events)),<br/>                deleted_withdraw_event_handle_creation_number: guid::creation_num(event::guid(&amp;withdraw_events))<br/>            &#125;<br/>        );<br/>        event::destroy_handle(deposit_events);<br/>        event::destroy_handle(withdraw_events);<br/>        if (coin.value &#61;&#61; 0) &#123;<br/>            destroy_zero(coin);<br/>        &#125; else &#123;<br/>            fungible_asset::deposit(store, coin_to_fungible_asset(coin));<br/>        &#125;;<br/>        // Note:<br/>        // It is possible the primary fungible store may already exist before this function call.<br/>        // In this case, if the account owns a frozen CoinStore and an unfrozen primary fungible store, this<br/>        // function would convert and deposit the rest coin into the primary store and freeze it to make the<br/>        // `frozen` semantic as consistent as possible.<br/>        if (frozen !&#61; fungible_asset::is_frozen(store)) &#123;<br/>            fungible_asset::set_frozen_flag_internal(store, frozen);<br/>        &#125;<br/>    &#125;;<br/>    if (!exists&lt;MigrationFlag&gt;(store_address)) &#123;<br/>        move_to(&amp;create_signer::create_signer(store_address), MigrationFlag &#123;&#125;);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_migrate_to_fungible_store"></a>

## Function `migrate_to_fungible_store`

Voluntarily migrate to fungible store for <code>CoinType</code> if not yet.


<pre><code>public entry fun migrate_to_fungible_store&lt;CoinType&gt;(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun migrate_to_fungible_store&lt;CoinType&gt;(<br/>    account: &amp;signer<br/>) acquires CoinStore, CoinConversionMap, CoinInfo &#123;<br/>    maybe_convert_to_fungible_store&lt;CoinType&gt;(signer::address_of(account));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_coin_address"></a>

## Function `coin_address`

A helper function that returns the address of CoinType.


<pre><code>fun coin_address&lt;CoinType&gt;(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun coin_address&lt;CoinType&gt;(): address &#123;<br/>    let type_info &#61; type_info::type_of&lt;CoinType&gt;();<br/>    type_info::account_address(&amp;type_info)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_balance"></a>

## Function `balance`

Returns the balance of <code>owner</code> for provided <code>CoinType</code> and its paired FA if exists.


<pre><code>&#35;[view]<br/>public fun balance&lt;CoinType&gt;(owner: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance&lt;CoinType&gt;(owner: address): u64 acquires CoinConversionMap, CoinStore &#123;<br/>    let paired_metadata &#61; paired_metadata&lt;CoinType&gt;();<br/>    coin_balance&lt;CoinType&gt;(owner) &#43; if (option::is_some(&amp;paired_metadata)) &#123;<br/>        primary_fungible_store::balance(<br/>            owner,<br/>            option::extract(&amp;mut paired_metadata)<br/>        )<br/>    &#125; else &#123; 0 &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_is_balance_at_least"></a>

## Function `is_balance_at_least`

Returns whether the balance of <code>owner</code> for provided <code>CoinType</code> and its paired FA is >= <code>amount</code>.


<pre><code>&#35;[view]<br/>public fun is_balance_at_least&lt;CoinType&gt;(owner: address, amount: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_balance_at_least&lt;CoinType&gt;(owner: address, amount: u64): bool acquires CoinConversionMap, CoinStore &#123;<br/>    let coin_balance &#61; coin_balance&lt;CoinType&gt;(owner);<br/>    if (coin_balance &gt;&#61; amount) &#123;<br/>        return true<br/>    &#125;;<br/><br/>    let paired_metadata &#61; paired_metadata&lt;CoinType&gt;();<br/>    let left_amount &#61; amount &#45; coin_balance;<br/>    if (option::is_some(&amp;paired_metadata)) &#123;<br/>        primary_fungible_store::is_balance_at_least(<br/>            owner,<br/>            option::extract(&amp;mut paired_metadata),<br/>            left_amount<br/>        )<br/>    &#125; else &#123; false &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_coin_balance"></a>

## Function `coin_balance`



<pre><code>fun coin_balance&lt;CoinType&gt;(owner: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun coin_balance&lt;CoinType&gt;(owner: address): u64 &#123;<br/>    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(owner)) &#123;<br/>        borrow_global&lt;CoinStore&lt;CoinType&gt;&gt;(owner).coin.value<br/>    &#125; else &#123;<br/>        0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_is_coin_initialized"></a>

## Function `is_coin_initialized`

Returns <code>true</code> if the type <code>CoinType</code> is an initialized coin.


<pre><code>&#35;[view]<br/>public fun is_coin_initialized&lt;CoinType&gt;(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_coin_initialized&lt;CoinType&gt;(): bool &#123;<br/>    exists&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;())<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_is_coin_store_frozen"></a>

## Function `is_coin_store_frozen`

Returns <code>true</code> is account_addr has frozen the CoinStore or if it's not registered at all


<pre><code>&#35;[view]<br/>public fun is_coin_store_frozen&lt;CoinType&gt;(account_addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_coin_store_frozen&lt;CoinType&gt;(<br/>    account_addr: address<br/>): bool acquires CoinStore, CoinConversionMap &#123;<br/>    if (!is_account_registered&lt;CoinType&gt;(account_addr)) &#123;<br/>        return true<br/>    &#125;;<br/><br/>    let coin_store &#61; borrow_global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>    coin_store.frozen<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_is_account_registered"></a>

## Function `is_account_registered`

Returns <code>true</code> if <code>account_addr</code> is registered to receive <code>CoinType</code>.


<pre><code>&#35;[view]<br/>public fun is_account_registered&lt;CoinType&gt;(account_addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_account_registered&lt;CoinType&gt;(account_addr: address): bool acquires CoinConversionMap &#123;<br/>    assert!(is_coin_initialized&lt;CoinType&gt;(), error::invalid_argument(ECOIN_INFO_NOT_PUBLISHED));<br/>    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)) &#123;<br/>        true<br/>    &#125; else &#123;<br/>        let paired_metadata_opt &#61; paired_metadata&lt;CoinType&gt;();<br/>        (option::is_some(<br/>            &amp;paired_metadata_opt<br/>        ) &amp;&amp; migrated_primary_fungible_store_exists(account_addr, option::destroy_some(paired_metadata_opt)))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_name"></a>

## Function `name`

Returns the name of the coin.


<pre><code>&#35;[view]<br/>public fun name&lt;CoinType&gt;(): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;CoinType&gt;(): string::String acquires CoinInfo &#123;<br/>    borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).name<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_symbol"></a>

## Function `symbol`

Returns the symbol of the coin, usually a shorter version of the name.


<pre><code>&#35;[view]<br/>public fun symbol&lt;CoinType&gt;(): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun symbol&lt;CoinType&gt;(): string::String acquires CoinInfo &#123;<br/>    borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).symbol<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_decimals"></a>

## Function `decimals`

Returns the number of decimals used to get its user representation.
For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).


<pre><code>&#35;[view]<br/>public fun decimals&lt;CoinType&gt;(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun decimals&lt;CoinType&gt;(): u8 acquires CoinInfo &#123;<br/>    borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).decimals<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_supply"></a>

## Function `supply`

Returns the amount of coin in existence.


<pre><code>&#35;[view]<br/>public fun supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun supply&lt;CoinType&gt;(): Option&lt;u128&gt; acquires CoinInfo, CoinConversionMap &#123;<br/>    let coin_supply &#61; coin_supply&lt;CoinType&gt;();<br/>    let metadata &#61; paired_metadata&lt;CoinType&gt;();<br/>    if (option::is_some(&amp;metadata)) &#123;<br/>        let fungible_asset_supply &#61; fungible_asset::supply(option::extract(&amp;mut metadata));<br/>        if (option::is_some(&amp;coin_supply)) &#123;<br/>            let supply &#61; option::borrow_mut(&amp;mut coin_supply);<br/>            &#42;supply &#61; &#42;supply &#43; option::destroy_some(fungible_asset_supply);<br/>        &#125;;<br/>    &#125;;<br/>    coin_supply<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_coin_supply"></a>

## Function `coin_supply`

Returns the amount of coin in existence.


<pre><code>&#35;[view]<br/>public fun coin_supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun coin_supply&lt;CoinType&gt;(): Option&lt;u128&gt; acquires CoinInfo &#123;<br/>    let maybe_supply &#61; &amp;borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).supply;<br/>    if (option::is_some(maybe_supply)) &#123;<br/>        // We do track supply, in this case read from optional aggregator.<br/>        let supply &#61; option::borrow(maybe_supply);<br/>        let value &#61; optional_aggregator::read(supply);<br/>        option::some(value)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_burn"></a>

## Function `burn`

Burn <code>coin</code> with capability.
The capability <code>_cap</code> should be passed as a reference to <code>BurnCapability&lt;CoinType&gt;</code>.


<pre><code>public fun burn&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;, _cap: &amp;coin::BurnCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn&lt;CoinType&gt;(coin: Coin&lt;CoinType&gt;, _cap: &amp;BurnCapability&lt;CoinType&gt;) acquires CoinInfo &#123;<br/>    burn_internal(coin);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_burn_from"></a>

## Function `burn_from`

Burn <code>coin</code> from the specified <code>account</code> with capability.
The capability <code>burn_cap</code> should be passed as a reference to <code>BurnCapability&lt;CoinType&gt;</code>.
This function shouldn't fail as it's called as part of transaction fee burning.

Note: This bypasses CoinStore::frozen -- coins within a frozen CoinStore can be burned.


<pre><code>public fun burn_from&lt;CoinType&gt;(account_addr: address, amount: u64, burn_cap: &amp;coin::BurnCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn_from&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    amount: u64,<br/>    burn_cap: &amp;BurnCapability&lt;CoinType&gt;,<br/>) acquires CoinInfo, CoinStore, CoinConversionMap, PairedFungibleAssetRefs &#123;<br/>    // Skip burning if amount is zero. This shouldn&apos;t error out as it&apos;s called as part of transaction fee burning.<br/>    if (amount &#61;&#61; 0) &#123;<br/>        return<br/>    &#125;;<br/><br/>    let (coin_amount_to_burn, fa_amount_to_burn) &#61; calculate_amount_to_withdraw&lt;CoinType&gt;(<br/>        account_addr,<br/>        amount<br/>    );<br/>    if (coin_amount_to_burn &gt; 0) &#123;<br/>        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>        let coin_to_burn &#61; extract(&amp;mut coin_store.coin, coin_amount_to_burn);<br/>        burn(coin_to_burn, burn_cap);<br/>    &#125;;<br/>    if (fa_amount_to_burn &gt; 0) &#123;<br/>        fungible_asset::burn_from(<br/>            borrow_paired_burn_ref(burn_cap),<br/>            primary_fungible_store::primary_store(account_addr, option::destroy_some(paired_metadata&lt;CoinType&gt;())),<br/>            fa_amount_to_burn<br/>        );<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_deposit"></a>

## Function `deposit`

Deposit the coin balance into the recipient's account and emit an event.


<pre><code>public fun deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    coin: Coin&lt;CoinType&gt;<br/>) acquires CoinStore, CoinConversionMap, CoinInfo &#123;<br/>    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)) &#123;<br/>        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>        assert!(<br/>            !coin_store.frozen,<br/>            error::permission_denied(EFROZEN),<br/>        );<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(Deposit&lt;CoinType&gt; &#123; account: account_addr, amount: coin.value &#125;);<br/>        &#125;;<br/>        event::emit_event&lt;DepositEvent&gt;(<br/>            &amp;mut coin_store.deposit_events,<br/>            DepositEvent &#123; amount: coin.value &#125;,<br/>        );<br/>        merge(&amp;mut coin_store.coin, coin);<br/>    &#125; else &#123;<br/>        let metadata &#61; paired_metadata&lt;CoinType&gt;();<br/>        if (option::is_some(&amp;metadata) &amp;&amp; migrated_primary_fungible_store_exists(<br/>            account_addr,<br/>            option::destroy_some(metadata)<br/>        )) &#123;<br/>            primary_fungible_store::deposit(account_addr, coin_to_fungible_asset(coin));<br/>        &#125; else &#123;<br/>            abort error::not_found(ECOIN_STORE_NOT_PUBLISHED)<br/>        &#125;;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_migrated_primary_fungible_store_exists"></a>

## Function `migrated_primary_fungible_store_exists`



<pre><code>fun migrated_primary_fungible_store_exists(account_address: address, metadata: object::Object&lt;fungible_asset::Metadata&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun migrated_primary_fungible_store_exists(<br/>    account_address: address,<br/>    metadata: Object&lt;Metadata&gt;<br/>): bool &#123;<br/>    let primary_store_address &#61; primary_fungible_store::primary_store_address&lt;Metadata&gt;(account_address, metadata);<br/>    fungible_asset::store_exists(primary_store_address) &amp;&amp; exists&lt;MigrationFlag&gt;(primary_store_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_force_deposit"></a>

## Function `force_deposit`

Deposit the coin balance into the recipient's account without checking if the account is frozen.
This is for internal use only and doesn't emit an DepositEvent.


<pre><code>public(friend) fun force_deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun force_deposit&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    coin: Coin&lt;CoinType&gt;<br/>) acquires CoinStore, CoinConversionMap, CoinInfo &#123;<br/>    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)) &#123;<br/>        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>        merge(&amp;mut coin_store.coin, coin);<br/>    &#125; else &#123;<br/>        let metadata &#61; paired_metadata&lt;CoinType&gt;();<br/>        if (option::is_some(&amp;metadata) &amp;&amp; migrated_primary_fungible_store_exists(<br/>            account_addr,<br/>            option::destroy_some(metadata)<br/>        )) &#123;<br/>            let fa &#61; coin_to_fungible_asset(coin);<br/>            let metadata &#61; fungible_asset::asset_metadata(&amp;fa);<br/>            let store &#61; primary_fungible_store::primary_store(account_addr, metadata);<br/>            fungible_asset::deposit_internal(store, fa);<br/>        &#125; else &#123;<br/>            abort error::not_found(ECOIN_STORE_NOT_PUBLISHED)<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_destroy_zero"></a>

## Function `destroy_zero`

Destroys a zero-value coin. Calls will fail if the <code>value</code> in the passed-in <code>token</code> is non-zero
so it is impossible to "burn" any non-zero amount of <code>Coin</code> without having
a <code>BurnCapability</code> for the specific <code>CoinType</code>.


<pre><code>public fun destroy_zero&lt;CoinType&gt;(zero_coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_zero&lt;CoinType&gt;(zero_coin: Coin&lt;CoinType&gt;) &#123;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; zero_coin.value;<br/>    &#125;;<br/>    let Coin &#123; value &#125; &#61; zero_coin;<br/>    assert!(value &#61;&#61; 0, error::invalid_argument(EDESTRUCTION_OF_NONZERO_TOKEN))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_extract"></a>

## Function `extract`

Extracts <code>amount</code> from the passed-in <code>coin</code>, where the original token is modified in place.


<pre><code>public fun extract&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;, amount: u64): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract&lt;CoinType&gt;(coin: &amp;mut Coin&lt;CoinType&gt;, amount: u64): Coin&lt;CoinType&gt; &#123;<br/>    assert!(coin.value &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; amount;<br/>    &#125;;<br/>    coin.value &#61; coin.value &#45; amount;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; amount;<br/>    &#125;;<br/>    Coin &#123; value: amount &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_extract_all"></a>

## Function `extract_all`

Extracts the entire amount from the passed-in <code>coin</code>, where the original token is modified in place.


<pre><code>public fun extract_all&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract_all&lt;CoinType&gt;(coin: &amp;mut Coin&lt;CoinType&gt;): Coin&lt;CoinType&gt; &#123;<br/>    let total_value &#61; coin.value;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; coin.value;<br/>    &#125;;<br/>    coin.value &#61; 0;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; total_value;<br/>    &#125;;<br/>    Coin &#123; value: total_value &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_freeze_coin_store"></a>

## Function `freeze_coin_store`

Freeze a CoinStore to prevent transfers


<pre><code>&#35;[legacy_entry_fun]<br/>public entry fun freeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun freeze_coin_store&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    _freeze_cap: &amp;FreezeCapability&lt;CoinType&gt;,<br/>) acquires CoinStore &#123;<br/>    let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>    coin_store.frozen &#61; true;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_unfreeze_coin_store"></a>

## Function `unfreeze_coin_store`

Unfreeze a CoinStore to allow transfers


<pre><code>&#35;[legacy_entry_fun]<br/>public entry fun unfreeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unfreeze_coin_store&lt;CoinType&gt;(<br/>    account_addr: address,<br/>    _freeze_cap: &amp;FreezeCapability&lt;CoinType&gt;,<br/>) acquires CoinStore &#123;<br/>    let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>    coin_store.frozen &#61; false;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_upgrade_supply"></a>

## Function `upgrade_supply`

Upgrade total supply to use a parallelizable implementation if it is
available.


<pre><code>public entry fun upgrade_supply&lt;CoinType&gt;(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun upgrade_supply&lt;CoinType&gt;(account: &amp;signer) acquires CoinInfo, SupplyConfig &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/><br/>    // Only coin creators can upgrade total supply.<br/>    assert!(<br/>        coin_address&lt;CoinType&gt;() &#61;&#61; account_addr,<br/>        error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),<br/>    );<br/><br/>    // Can only succeed once on&#45;chain governance agreed on the upgrade.<br/>    assert!(<br/>        borrow_global_mut&lt;SupplyConfig&gt;(@aptos_framework).allow_upgrades,<br/>        error::permission_denied(ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED)<br/>    );<br/><br/>    let maybe_supply &#61; &amp;mut borrow_global_mut&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr).supply;<br/>    if (option::is_some(maybe_supply)) &#123;<br/>        let supply &#61; option::borrow_mut(maybe_supply);<br/><br/>        // If supply is tracked and the current implementation uses an integer &#45; upgrade.<br/>        if (!optional_aggregator::is_parallelizable(supply)) &#123;<br/>            optional_aggregator::switch(supply);<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_initialize"></a>

## Function `initialize`

Creates a new Coin with given <code>CoinType</code> and returns minting/freezing/burning capabilities.
The given signer also becomes the account hosting the information  about the coin
(name, supply, etc.). Supply is initialized as non-parallelizable integer.


<pre><code>public fun initialize&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize&lt;CoinType&gt;(<br/>    account: &amp;signer,<br/>    name: string::String,<br/>    symbol: string::String,<br/>    decimals: u8,<br/>    monitor_supply: bool,<br/>): (BurnCapability&lt;CoinType&gt;, FreezeCapability&lt;CoinType&gt;, MintCapability&lt;CoinType&gt;) &#123;<br/>    initialize_internal(account, name, symbol, decimals, monitor_supply, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_initialize_with_parallelizable_supply"></a>

## Function `initialize_with_parallelizable_supply`

Same as <code>initialize</code> but supply can be initialized to parallelizable aggregator.


<pre><code>public(friend) fun initialize_with_parallelizable_supply&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_with_parallelizable_supply&lt;CoinType&gt;(<br/>    account: &amp;signer,<br/>    name: string::String,<br/>    symbol: string::String,<br/>    decimals: u8,<br/>    monitor_supply: bool,<br/>): (BurnCapability&lt;CoinType&gt;, FreezeCapability&lt;CoinType&gt;, MintCapability&lt;CoinType&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(account);<br/>    initialize_internal(account, name, symbol, decimals, monitor_supply, true)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_initialize_internal"></a>

## Function `initialize_internal`



<pre><code>fun initialize_internal&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool, parallelizable: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_internal&lt;CoinType&gt;(<br/>    account: &amp;signer,<br/>    name: string::String,<br/>    symbol: string::String,<br/>    decimals: u8,<br/>    monitor_supply: bool,<br/>    parallelizable: bool,<br/>): (BurnCapability&lt;CoinType&gt;, FreezeCapability&lt;CoinType&gt;, MintCapability&lt;CoinType&gt;) &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/><br/>    assert!(<br/>        coin_address&lt;CoinType&gt;() &#61;&#61; account_addr,<br/>        error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),<br/>    );<br/><br/>    assert!(<br/>        !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr),<br/>        error::already_exists(ECOIN_INFO_ALREADY_PUBLISHED),<br/>    );<br/><br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_COIN_NAME_LENGTH, error::invalid_argument(ECOIN_NAME_TOO_LONG));<br/>    assert!(string::length(&amp;symbol) &lt;&#61; MAX_COIN_SYMBOL_LENGTH, error::invalid_argument(ECOIN_SYMBOL_TOO_LONG));<br/><br/>    let coin_info &#61; CoinInfo&lt;CoinType&gt; &#123;<br/>        name,<br/>        symbol,<br/>        decimals,<br/>        supply: if (monitor_supply) &#123;<br/>            option::some(<br/>                optional_aggregator::new(MAX_U128, parallelizable)<br/>            )<br/>        &#125; else &#123; option::none() &#125;,<br/>    &#125;;<br/>    move_to(account, coin_info);<br/><br/>    (BurnCapability&lt;CoinType&gt; &#123;&#125;, FreezeCapability&lt;CoinType&gt; &#123;&#125;, MintCapability&lt;CoinType&gt; &#123;&#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_merge"></a>

## Function `merge`

"Merges" the two given coins.  The coin passed in as <code>dst_coin</code> will have a value equal
to the sum of the two tokens (<code>dst_coin</code> and <code>source_coin</code>).


<pre><code>public fun merge&lt;CoinType&gt;(dst_coin: &amp;mut coin::Coin&lt;CoinType&gt;, source_coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun merge&lt;CoinType&gt;(dst_coin: &amp;mut Coin&lt;CoinType&gt;, source_coin: Coin&lt;CoinType&gt;) &#123;<br/>    spec &#123;<br/>        assume dst_coin.value &#43; source_coin.value &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; source_coin.value;<br/>    &#125;;<br/>    let Coin &#123; value &#125; &#61; source_coin;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; value;<br/>    &#125;;<br/>    dst_coin.value &#61; dst_coin.value &#43; value;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_mint"></a>

## Function `mint`

Mint new <code>Coin</code> with capability.
The capability <code>_cap</code> should be passed as reference to <code>MintCapability&lt;CoinType&gt;</code>.
Returns minted <code>Coin</code>.


<pre><code>public fun mint&lt;CoinType&gt;(amount: u64, _cap: &amp;coin::MintCapability&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint&lt;CoinType&gt;(<br/>    amount: u64,<br/>    _cap: &amp;MintCapability&lt;CoinType&gt;,<br/>): Coin&lt;CoinType&gt; acquires CoinInfo &#123;<br/>    mint_internal&lt;CoinType&gt;(amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_register"></a>

## Function `register`



<pre><code>public fun register&lt;CoinType&gt;(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun register&lt;CoinType&gt;(account: &amp;signer) acquires CoinConversionMap &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/>    // Short&#45;circuit and do nothing if account is already registered for CoinType.<br/>    if (is_account_registered&lt;CoinType&gt;(account_addr)) &#123;<br/>        return<br/>    &#125;;<br/><br/>    account::register_coin&lt;CoinType&gt;(account_addr);<br/>    let coin_store &#61; CoinStore&lt;CoinType&gt; &#123;<br/>        coin: Coin &#123; value: 0 &#125;,<br/>        frozen: false,<br/>        deposit_events: account::new_event_handle&lt;DepositEvent&gt;(account),<br/>        withdraw_events: account::new_event_handle&lt;WithdrawEvent&gt;(account),<br/>    &#125;;<br/>    move_to(account, coin_store);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of coins <code>CoinType</code> from <code>from</code> to <code>to</code>.


<pre><code>public entry fun transfer&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;CoinType&gt;(<br/>    from: &amp;signer,<br/>    to: address,<br/>    amount: u64,<br/>) acquires CoinStore, CoinConversionMap, CoinInfo, PairedCoinType &#123;<br/>    let coin &#61; withdraw&lt;CoinType&gt;(from, amount);<br/>    deposit(to, coin);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_value"></a>

## Function `value`

Returns the <code>value</code> passed in <code>coin</code>.


<pre><code>public fun value&lt;CoinType&gt;(coin: &amp;coin::Coin&lt;CoinType&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun value&lt;CoinType&gt;(coin: &amp;Coin&lt;CoinType&gt;): u64 &#123;<br/>    coin.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_withdraw"></a>

## Function `withdraw`

Withdraw specified <code>amount</code> of coin <code>CoinType</code> from the signing account.


<pre><code>public fun withdraw&lt;CoinType&gt;(account: &amp;signer, amount: u64): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;CoinType&gt;(<br/>    account: &amp;signer,<br/>    amount: u64,<br/>): Coin&lt;CoinType&gt; acquires CoinStore, CoinConversionMap, CoinInfo, PairedCoinType &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/><br/>    let (coin_amount_to_withdraw, fa_amount_to_withdraw) &#61; calculate_amount_to_withdraw&lt;CoinType&gt;(<br/>        account_addr,<br/>        amount<br/>    );<br/>    let withdrawn_coin &#61; if (coin_amount_to_withdraw &gt; 0) &#123;<br/>        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>        assert!(<br/>            !coin_store.frozen,<br/>            error::permission_denied(EFROZEN),<br/>        );<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(Withdraw&lt;CoinType&gt; &#123; account: account_addr, amount: coin_amount_to_withdraw &#125;);<br/>        &#125;;<br/>        event::emit_event&lt;WithdrawEvent&gt;(<br/>            &amp;mut coin_store.withdraw_events,<br/>            WithdrawEvent &#123; amount: coin_amount_to_withdraw &#125;,<br/>        );<br/>        extract(&amp;mut coin_store.coin, coin_amount_to_withdraw)<br/>    &#125; else &#123;<br/>        zero()<br/>    &#125;;<br/>    if (fa_amount_to_withdraw &gt; 0) &#123;<br/>        let fa &#61; primary_fungible_store::withdraw(<br/>            account,<br/>            option::destroy_some(paired_metadata&lt;CoinType&gt;()),<br/>            fa_amount_to_withdraw<br/>        );<br/>        merge(&amp;mut withdrawn_coin, fungible_asset_to_coin(fa));<br/>    &#125;;<br/>    withdrawn_coin<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_zero"></a>

## Function `zero`

Create a new <code>Coin&lt;CoinType&gt;</code> with a value of <code>0</code>.


<pre><code>public fun zero&lt;CoinType&gt;(): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun zero&lt;CoinType&gt;(): Coin&lt;CoinType&gt; &#123;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; 0;<br/>    &#125;;<br/>    Coin&lt;CoinType&gt; &#123;<br/>        value: 0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_destroy_freeze_cap"></a>

## Function `destroy_freeze_cap`

Destroy a freeze capability. Freeze capability is dangerous and therefore should be destroyed if not used.


<pre><code>public fun destroy_freeze_cap&lt;CoinType&gt;(freeze_cap: coin::FreezeCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_freeze_cap&lt;CoinType&gt;(freeze_cap: FreezeCapability&lt;CoinType&gt;) &#123;<br/>    let FreezeCapability&lt;CoinType&gt; &#123;&#125; &#61; freeze_cap;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Destroy a mint capability.


<pre><code>public fun destroy_mint_cap&lt;CoinType&gt;(mint_cap: coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_mint_cap&lt;CoinType&gt;(mint_cap: MintCapability&lt;CoinType&gt;) &#123;<br/>    let MintCapability&lt;CoinType&gt; &#123;&#125; &#61; mint_cap;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_destroy_burn_cap"></a>

## Function `destroy_burn_cap`

Destroy a burn capability.


<pre><code>public fun destroy_burn_cap&lt;CoinType&gt;(burn_cap: coin::BurnCapability&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_burn_cap&lt;CoinType&gt;(burn_cap: BurnCapability&lt;CoinType&gt;) &#123;<br/>    let BurnCapability&lt;CoinType&gt; &#123;&#125; &#61; burn_cap;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_mint_internal"></a>

## Function `mint_internal`



<pre><code>fun mint_internal&lt;CoinType&gt;(amount: u64): coin::Coin&lt;CoinType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun mint_internal&lt;CoinType&gt;(amount: u64): Coin&lt;CoinType&gt; acquires CoinInfo &#123;<br/>    if (amount &#61;&#61; 0) &#123;<br/>        return Coin&lt;CoinType&gt; &#123;<br/>            value: 0<br/>        &#125;<br/>    &#125;;<br/><br/>    let maybe_supply &#61; &amp;mut borrow_global_mut&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).supply;<br/>    if (option::is_some(maybe_supply)) &#123;<br/>        let supply &#61; option::borrow_mut(maybe_supply);<br/>        spec &#123;<br/>            use aptos_framework::optional_aggregator;<br/>            use aptos_framework::aggregator;<br/>            assume optional_aggregator::is_parallelizable(supply) &#61;&#61;&gt; (aggregator::spec_aggregator_get_val(<br/>                option::borrow(supply.aggregator)<br/>            )<br/>                &#43; amount &lt;&#61; aggregator::spec_get_limit(option::borrow(supply.aggregator)));<br/>            assume !optional_aggregator::is_parallelizable(supply) &#61;&#61;&gt;<br/>                (option::borrow(supply.integer).value &#43; amount &lt;&#61; option::borrow(supply.integer).limit);<br/>        &#125;;<br/>        optional_aggregator::add(supply, (amount as u128));<br/>    &#125;;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; amount;<br/>    &#125;;<br/>    Coin&lt;CoinType&gt; &#123; value: amount &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_coin_burn_internal"></a>

## Function `burn_internal`



<pre><code>fun burn_internal&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun burn_internal&lt;CoinType&gt;(coin: Coin&lt;CoinType&gt;): u64 acquires CoinInfo &#123;<br/>    spec &#123;<br/>        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; coin.value;<br/>    &#125;;<br/>    let Coin &#123; value: amount &#125; &#61; coin;<br/>    if (amount !&#61; 0) &#123;<br/>        let maybe_supply &#61; &amp;mut borrow_global_mut&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).supply;<br/>        if (option::is_some(maybe_supply)) &#123;<br/>            let supply &#61; option::borrow_mut(maybe_supply);<br/>            optional_aggregator::sub(supply, (amount as u128));<br/>        &#125;;<br/>    &#125;;<br/>    amount<br/>&#125;<br/></code></pre>



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
<td>Coin operations should fail if the user's CoinStore is frozen.</td>
<td>Medium</td>
<td>If the CoinStore of an address is frozen, coin operations are disallowed.</td>
<td>Formally Verified via <a href="#high-level-req-8.1">withdraw</a>, <a href="#high-level-req-8.2">transfer</a> and <a href="#high-level-req-8.3">deposit</a>.</td>
</tr>

<tr>
<td>9</td>
<td>Utilizing AggregatableCoins does not violate other critical invariants, such as (4).</td>
<td>High</td>
<td>Utilizing AggregatableCoin does not change the real-supply of any token.</td>
<td>Formally Verified via <a href="#high-level-req-9">TotalSupplyNoChange</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/><a id="0x1_coin_supply"></a>
global supply&lt;CoinType&gt;: num;<br/><a id="0x1_coin_aggregate_supply"></a>
global aggregate_supply&lt;CoinType&gt;: num;<br/>apply TotalSupplyTracked&lt;CoinType&gt; to &#42;&lt;CoinType&gt; except<br/>initialize, initialize_internal, initialize_with_parallelizable_supply;<br/></code></pre>




<a id="0x1_coin_spec_fun_supply_tracked"></a>


<pre><code>fun spec_fun_supply_tracked&lt;CoinType&gt;(val: u64, supply: Option&lt;OptionalAggregator&gt;): bool &#123;<br/>   option::spec_is_some(supply) &#61;&#61;&gt; val &#61;&#61; optional_aggregator::optional_aggregator_value
       (option::spec_borrow(supply))<br/>&#125;<br/></code></pre>




<a id="0x1_coin_TotalSupplyTracked"></a>


<pre><code>schema TotalSupplyTracked&lt;CoinType&gt; &#123;<br/>ensures old(spec_fun_supply_tracked&lt;CoinType&gt;(supply&lt;CoinType&gt; &#43; aggregate_supply&lt;CoinType&gt;,<br/>    global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply)) &#61;&#61;&gt;<br/>    spec_fun_supply_tracked&lt;CoinType&gt;(supply&lt;CoinType&gt; &#43; aggregate_supply&lt;CoinType&gt;,<br/>        global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply);<br/>&#125;<br/></code></pre>




<a id="0x1_coin_spec_fun_supply_no_change"></a>


<pre><code>fun spec_fun_supply_no_change&lt;CoinType&gt;(old_supply: Option&lt;OptionalAggregator&gt;,<br/>                                            supply: Option&lt;OptionalAggregator&gt;): bool &#123;<br/>   option::spec_is_some(old_supply) &#61;&#61;&gt; optional_aggregator::optional_aggregator_value
       (option::spec_borrow(old_supply)) &#61;&#61; optional_aggregator::optional_aggregator_value
       (option::spec_borrow(supply))<br/>&#125;<br/></code></pre>




<a id="0x1_coin_TotalSupplyNoChange"></a>


<pre><code>schema TotalSupplyNoChange&lt;CoinType&gt; &#123;<br/>let old_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply;<br/>let post supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply;<br/>ensures spec_fun_supply_no_change&lt;CoinType&gt;(old_supply, supply);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_AggregatableCoin"></a>

### Struct `AggregatableCoin`


<pre><code>struct AggregatableCoin&lt;CoinType&gt; has store<br/></code></pre>



<dl>
<dt>
<code>value: aggregator::Aggregator</code>
</dt>
<dd>
 Amount of aggregatable coin this address has.
</dd>
</dl>



<pre><code>invariant aggregator::spec_get_limit(value) &#61;&#61; MAX_U64;<br/></code></pre>



<a id="@Specification_1_coin_to_fungible_asset"></a>

### Function `coin_to_fungible_asset`


<pre><code>public fun coin_to_fungible_asset&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): fungible_asset::FungibleAsset<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/></code></pre>



<a id="@Specification_1_fungible_asset_to_coin"></a>

### Function `fungible_asset_to_coin`


<pre><code>fun fungible_asset_to_coin&lt;CoinType&gt;(fungible_asset: fungible_asset::FungibleAsset): coin::Coin&lt;CoinType&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_initialize_supply_config"></a>

### Function `initialize_supply_config`


<pre><code>public(friend) fun initialize_supply_config(aptos_framework: &amp;signer)<br/></code></pre>


Can only be initialized once.
Can only be published by reserved addresses.


<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>aborts_if exists&lt;SupplyConfig&gt;(aptos_addr);<br/>ensures !global&lt;SupplyConfig&gt;(aptos_addr).allow_upgrades;<br/>ensures exists&lt;SupplyConfig&gt;(aptos_addr);<br/></code></pre>



<a id="@Specification_1_allow_supply_upgrades"></a>

### Function `allow_supply_upgrades`


<pre><code>public fun allow_supply_upgrades(aptos_framework: &amp;signer, allowed: bool)<br/></code></pre>


Can only be updated by <code>@aptos_framework</code>.


<pre><code>modifies global&lt;SupplyConfig&gt;(@aptos_framework);<br/>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>aborts_if !exists&lt;SupplyConfig&gt;(aptos_addr);<br/>let post allow_upgrades_post &#61; global&lt;SupplyConfig&gt;(@aptos_framework);<br/>ensures allow_upgrades_post.allow_upgrades &#61;&#61; allowed;<br/></code></pre>



<a id="@Specification_1_initialize_aggregatable_coin"></a>

### Function `initialize_aggregatable_coin`


<pre><code>public(friend) fun initialize_aggregatable_coin&lt;CoinType&gt;(aptos_framework: &amp;signer): coin::AggregatableCoin&lt;CoinType&gt;<br/></code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework &#123; account: aptos_framework &#125;;<br/>include aggregator_factory::CreateAggregatorInternalAbortsIf;<br/></code></pre>



<a id="@Specification_1_is_aggregatable_coin_zero"></a>

### Function `is_aggregatable_coin_zero`


<pre><code>public(friend) fun is_aggregatable_coin_zero&lt;CoinType&gt;(coin: &amp;coin::AggregatableCoin&lt;CoinType&gt;): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; (aggregator::spec_read(coin.value) &#61;&#61; 0);<br/></code></pre>



<a id="@Specification_1_drain_aggregatable_coin"></a>

### Function `drain_aggregatable_coin`


<pre><code>public(friend) fun drain_aggregatable_coin&lt;CoinType&gt;(coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;<br/></code></pre>




<pre><code>aborts_if aggregator::spec_read(coin.value) &gt; MAX_U64;<br/>ensures result.value &#61;&#61; aggregator::spec_aggregator_get_val(old(coin).value);<br/></code></pre>



<a id="@Specification_1_merge_aggregatable_coin"></a>

### Function `merge_aggregatable_coin`


<pre><code>public(friend) fun merge_aggregatable_coin&lt;CoinType&gt;(dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;, coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>




<pre><code>let aggr &#61; dst_coin.value;<br/>let post p_aggr &#61; dst_coin.value;<br/>aborts_if aggregator::spec_aggregator_get_val(aggr)<br/>    &#43; coin.value &gt; aggregator::spec_get_limit(aggr);<br/>aborts_if aggregator::spec_aggregator_get_val(aggr)<br/>    &#43; coin.value &gt; MAX_U128;<br/>ensures aggregator::spec_aggregator_get_val(aggr) &#43; coin.value &#61;&#61; aggregator::spec_aggregator_get_val(p_aggr);<br/></code></pre>



<a id="@Specification_1_collect_into_aggregatable_coin"></a>

### Function `collect_into_aggregatable_coin`


<pre><code>public(friend) fun collect_into_aggregatable_coin&lt;CoinType&gt;(account_addr: address, amount: u64, dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let aggr &#61; dst_coin.value;<br/>let post p_aggr &#61; dst_coin.value;<br/>let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let post p_coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if amount &gt; 0 &amp;&amp; !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if amount &gt; 0 &amp;&amp; coin_store.coin.value &lt; amount;<br/>aborts_if amount &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)<br/>    &#43; amount &gt; aggregator::spec_get_limit(aggr);<br/>aborts_if amount &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)<br/>    &#43; amount &gt; MAX_U128;<br/>ensures aggregator::spec_aggregator_get_val(aggr) &#43; amount &#61;&#61; aggregator::spec_aggregator_get_val(p_aggr);<br/>ensures coin_store.coin.value &#45; amount &#61;&#61; p_coin_store.coin.value;<br/></code></pre>



<a id="@Specification_1_maybe_convert_to_fungible_store"></a>

### Function `maybe_convert_to_fungible_store`


<pre><code>fun maybe_convert_to_fungible_store&lt;CoinType&gt;(account: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account);<br/>modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account);<br/></code></pre>




<a id="0x1_coin_DepositAbortsIf"></a>


<pre><code>schema DepositAbortsIf&lt;CoinType&gt; &#123;<br/>account_addr: address;<br/>let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if coin_store.frozen;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_coin_address"></a>

### Function `coin_address`


<pre><code>fun coin_address&lt;CoinType&gt;(): address<br/></code></pre>


Get address by reflection.


<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/></code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code>&#35;[view]<br/>public fun balance&lt;CoinType&gt;(owner: address): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(owner);<br/>ensures result &#61;&#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(owner).coin.value;<br/></code></pre>



<a id="@Specification_1_is_coin_initialized"></a>

### Function `is_coin_initialized`


<pre><code>&#35;[view]<br/>public fun is_coin_initialized&lt;CoinType&gt;(): bool<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-7.1" href="#high-level-req">high-level requirement 7</a>:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_is_account_registered"></a>

### Function `is_account_registered`


<pre><code>&#35;[view]<br/>public fun is_account_registered&lt;CoinType&gt;(account_addr: address): bool<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>aborts_if false;<br/></code></pre>




<a id="0x1_coin_get_coin_supply_opt"></a>


<pre><code>fun get_coin_supply_opt&lt;CoinType&gt;(): Option&lt;OptionalAggregator&gt; &#123;<br/>   global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply<br/>&#125;<br/></code></pre>




<a id="0x1_coin_spec_paired_metadata"></a>


<pre><code>fun spec_paired_metadata&lt;CoinType&gt;(): Option&lt;Object&lt;Metadata&gt;&gt; &#123;<br/>   if (exists&lt;CoinConversionMap&gt;(@aptos_framework)) &#123;<br/>       let map &#61; global&lt;CoinConversionMap&gt;(@aptos_framework).coin_to_fungible_asset_map;<br/>       if (table::spec_contains(map, type_info::type_of&lt;CoinType&gt;())) &#123;<br/>           let metadata &#61; table::spec_get(map, type_info::type_of&lt;CoinType&gt;());<br/>           option::spec_some(metadata)<br/>       &#125; else &#123;<br/>           option::spec_none()<br/>       &#125;<br/>   &#125; else &#123;<br/>       option::spec_none()<br/>   &#125;<br/>&#125;<br/></code></pre>




<a id="0x1_coin_spec_is_account_registered"></a>


<pre><code>fun spec_is_account_registered&lt;CoinType&gt;(account_addr: address): bool &#123;<br/>   let paired_metadata_opt &#61; spec_paired_metadata&lt;CoinType&gt;();<br/>   exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr) &#124;&#124; (option::spec_is_some(<br/>       paired_metadata_opt<br/>   ) &amp;&amp; primary_fungible_store::spec_primary_store_exists(account_addr, option::spec_borrow(paired_metadata_opt)))<br/>&#125;<br/></code></pre>




<a id="0x1_coin_CoinSubAbortsIf"></a>


<pre><code>schema CoinSubAbortsIf&lt;CoinType&gt; &#123;<br/>amount: u64;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;<br/>include (option::is_some(<br/>    maybe_supply<br/>)) &#61;&#61;&gt; optional_aggregator::SubAbortsIf &#123; optional_aggregator: option::borrow(maybe_supply), value: amount &#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_coin_CoinAddAbortsIf"></a>


<pre><code>schema CoinAddAbortsIf&lt;CoinType&gt; &#123;<br/>amount: u64;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;<br/>include (option::is_some(<br/>    maybe_supply<br/>)) &#61;&#61;&gt; optional_aggregator::AddAbortsIf &#123; optional_aggregator: option::borrow(maybe_supply), value: amount &#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_coin_AbortsIfNotExistCoinInfo"></a>


<pre><code>schema AbortsIfNotExistCoinInfo&lt;CoinType&gt; &#123;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>aborts_if !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_name"></a>

### Function `name`


<pre><code>&#35;[view]<br/>public fun name&lt;CoinType&gt;(): string::String<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-7.3" href="#high-level-req">high-level requirement 7</a>:
include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;<br/></code></pre>



<a id="@Specification_1_symbol"></a>

### Function `symbol`


<pre><code>&#35;[view]<br/>public fun symbol&lt;CoinType&gt;(): string::String<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-7.4" href="#high-level-req">high-level requirement 7</a>:
include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;<br/></code></pre>



<a id="@Specification_1_decimals"></a>

### Function `decimals`


<pre><code>&#35;[view]<br/>public fun decimals&lt;CoinType&gt;(): u8<br/></code></pre>




<pre><code>include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;<br/></code></pre>



<a id="@Specification_1_supply"></a>

### Function `supply`


<pre><code>&#35;[view]<br/>public fun supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_coin_supply"></a>

### Function `coin_supply`


<pre><code>&#35;[view]<br/>public fun coin_supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;<br/></code></pre>




<pre><code>let coin_addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>// This enforces <a id="high-level-req-7.5" href="#high-level-req">high-level requirement 7</a>:
aborts_if !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_addr);<br/>let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_addr).supply;<br/>let supply &#61; option::spec_borrow(maybe_supply);<br/>let value &#61; optional_aggregator::optional_aggregator_value(supply);<br/>ensures if (option::spec_is_some(maybe_supply)) &#123;<br/>    result &#61;&#61; option::spec_some(value)<br/>&#125; else &#123;<br/>    option::spec_is_none(result)<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public fun burn&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;, _cap: &amp;coin::BurnCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/>include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;<br/>aborts_if coin.value &#61;&#61; 0;<br/>include CoinSubAbortsIf&lt;CoinType&gt; &#123; amount: coin.value &#125;;<br/>ensures supply&lt;CoinType&gt; &#61;&#61; old(supply&lt;CoinType&gt;) &#45; coin.value;<br/></code></pre>



<a id="@Specification_1_burn_from"></a>

### Function `burn_from`


<pre><code>public fun burn_from&lt;CoinType&gt;(account_addr: address, amount: u64, burn_cap: &amp;coin::BurnCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let post post_coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/>modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
aborts_if amount !&#61; 0 &amp;&amp; !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/>aborts_if amount !&#61; 0 &amp;&amp; !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if coin_store.coin.value &lt; amount;<br/>let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;<br/>let supply_aggr &#61; option::spec_borrow(maybe_supply);<br/>let value &#61; optional_aggregator::optional_aggregator_value(supply_aggr);<br/>let post post_maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;<br/>let post post_supply &#61; option::spec_borrow(post_maybe_supply);<br/>let post post_value &#61; optional_aggregator::optional_aggregator_value(post_supply);<br/>aborts_if option::spec_is_some(maybe_supply) &amp;&amp; value &lt; amount;<br/>ensures post_coin_store.coin.value &#61;&#61; coin_store.coin.value &#45; amount;<br/>// This enforces <a id="high-level-req-5" href="managed_coin.md#high-level-req">high-level requirement 5</a> of the <a href="managed_coin.md">managed_coin</a> module:
ensures if (option::spec_is_some(maybe_supply)) &#123;<br/>    post_value &#61;&#61; value &#45; amount<br/>&#125; else &#123;<br/>    option::spec_is_none(post_maybe_supply)<br/>&#125;;<br/>ensures supply&lt;CoinType&gt; &#61;&#61; old(supply&lt;CoinType&gt;) &#45; amount;<br/></code></pre>



<a id="@Specification_1_deposit"></a>

### Function `deposit`


<pre><code>public fun deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>


<code>account_addr</code> is not frozen.


<pre><code>pragma verify &#61; false;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>// This enforces <a id="high-level-req-8.3" href="#high-level-req">high-level requirement 8</a>:
include DepositAbortsIf&lt;CoinType&gt;;<br/>ensures global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr).coin.value &#61;&#61; old(<br/>    global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)<br/>).coin.value &#43; coin.value;<br/></code></pre>



<a id="@Specification_1_force_deposit"></a>

### Function `force_deposit`


<pre><code>public(friend) fun force_deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>ensures global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr).coin.value &#61;&#61; old(<br/>    global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)<br/>).coin.value &#43; coin.value;<br/></code></pre>



<a id="@Specification_1_destroy_zero"></a>

### Function `destroy_zero`


<pre><code>public fun destroy_zero&lt;CoinType&gt;(zero_coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>


The value of <code>zero_coin</code> must be 0.


<pre><code>aborts_if zero_coin.value &gt; 0;<br/></code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code>public fun extract&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;, amount: u64): coin::Coin&lt;CoinType&gt;<br/></code></pre>




<pre><code>aborts_if coin.value &lt; amount;<br/>ensures result.value &#61;&#61; amount;<br/>ensures coin.value &#61;&#61; old(coin.value) &#45; amount;<br/></code></pre>



<a id="@Specification_1_extract_all"></a>

### Function `extract_all`


<pre><code>public fun extract_all&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;<br/></code></pre>




<pre><code>ensures result.value &#61;&#61; old(coin).value;<br/>ensures coin.value &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_freeze_coin_store"></a>

### Function `freeze_coin_store`


<pre><code>&#35;[legacy_entry_fun]<br/>public entry fun freeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>// This enforces <a id="high-level-req-6.3" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let post coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>ensures coin_store.frozen;<br/></code></pre>



<a id="@Specification_1_unfreeze_coin_store"></a>

### Function `unfreeze_coin_store`


<pre><code>&#35;[legacy_entry_fun]<br/>public entry fun unfreeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>// This enforces <a id="high-level-req-6.4" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let post coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>ensures !coin_store.frozen;<br/></code></pre>



<a id="@Specification_1_upgrade_supply"></a>

### Function `upgrade_supply`


<pre><code>public entry fun upgrade_supply&lt;CoinType&gt;(account: &amp;signer)<br/></code></pre>


The creator of <code>CoinType</code> must be <code>@aptos_framework</code>.
<code>SupplyConfig</code> allow upgrade.


<pre><code>let account_addr &#61; signer::address_of(account);<br/>let coin_address &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>aborts_if coin_address !&#61; account_addr;<br/>aborts_if !exists&lt;SupplyConfig&gt;(@aptos_framework);<br/>// This enforces <a id="high-level-req-1.1" href="#high-level-req">high-level requirement 1</a>:
aborts_if !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>let supply_config &#61; global&lt;SupplyConfig&gt;(@aptos_framework);<br/>aborts_if !supply_config.allow_upgrades;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr).supply;<br/>let supply &#61; option::spec_borrow(maybe_supply);<br/>let value &#61; optional_aggregator::optional_aggregator_value(supply);<br/>let post post_maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr).supply;<br/>let post post_supply &#61; option::spec_borrow(post_maybe_supply);<br/>let post post_value &#61; optional_aggregator::optional_aggregator_value(post_supply);<br/>let supply_no_parallel &#61; option::spec_is_some(maybe_supply) &amp;&amp;<br/>    !optional_aggregator::is_parallelizable(supply);<br/>aborts_if supply_no_parallel &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>ensures supply_no_parallel &#61;&#61;&gt;<br/>    optional_aggregator::is_parallelizable(post_supply) &amp;&amp; post_value &#61;&#61; value;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public fun initialize&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>let account_addr &#61; signer::address_of(account);<br/>// This enforces <a id="high-level-req-1.2" href="#high-level-req">high-level requirement 1</a>:
aborts_if type_info::type_of&lt;CoinType&gt;().account_address !&#61; account_addr;<br/>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if string::length(name) &gt; MAX_COIN_NAME_LENGTH;<br/>aborts_if string::length(symbol) &gt; MAX_COIN_SYMBOL_LENGTH;<br/></code></pre>



<a id="@Specification_1_initialize_with_parallelizable_supply"></a>

### Function `initialize_with_parallelizable_supply`


<pre><code>public(friend) fun initialize_with_parallelizable_supply&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(account);<br/>aborts_if addr !&#61; @aptos_framework;<br/>aborts_if monitor_supply &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>include InitializeInternalSchema&lt;CoinType&gt; &#123;<br/>    name: name.bytes,<br/>    symbol: symbol.bytes<br/>&#125;;<br/>ensures exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/></code></pre>


Make sure <code>name</code> and <code>symbol</code> are legal length.
Only the creator of <code>CoinType</code> can initialize.


<a id="0x1_coin_InitializeInternalSchema"></a>


<pre><code>schema InitializeInternalSchema&lt;CoinType&gt; &#123;<br/>account: signer;<br/>name: vector&lt;u8&gt;;<br/>symbol: vector&lt;u8&gt;;<br/>let account_addr &#61; signer::address_of(account);<br/>let coin_address &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>aborts_if coin_address !&#61; account_addr;<br/>aborts_if exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if len(name) &gt; MAX_COIN_NAME_LENGTH;<br/>aborts_if len(symbol) &gt; MAX_COIN_SYMBOL_LENGTH;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_initialize_internal"></a>

### Function `initialize_internal`


<pre><code>fun initialize_internal&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool, parallelizable: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)<br/></code></pre>




<pre><code>include InitializeInternalSchema&lt;CoinType&gt; &#123;<br/>    name: name.bytes,<br/>    symbol: symbol.bytes<br/>&#125;;<br/>let account_addr &#61; signer::address_of(account);<br/>let post coin_info &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>let post supply &#61; option::spec_borrow(coin_info.supply);<br/>let post value &#61; optional_aggregator::optional_aggregator_value(supply);<br/>let post limit &#61; optional_aggregator::optional_aggregator_limit(supply);<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);<br/>aborts_if monitor_supply &amp;&amp; parallelizable<br/>    &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>// This enforces <a id="high-level-req-2" href="managed_coin.md#high-level-req">high-level requirement 2</a> of the <a href="managed_coin.md">managed_coin</a> module:
ensures exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr)<br/>    &amp;&amp; coin_info.name &#61;&#61; name<br/>    &amp;&amp; coin_info.symbol &#61;&#61; symbol<br/>    &amp;&amp; coin_info.decimals &#61;&#61; decimals;<br/>ensures if (monitor_supply) &#123;<br/>    value &#61;&#61; 0 &amp;&amp; limit &#61;&#61; MAX_U128<br/>        &amp;&amp; (parallelizable &#61;&#61; optional_aggregator::is_parallelizable(supply))<br/>&#125; else &#123;<br/>    option::spec_is_none(coin_info.supply)<br/>&#125;;<br/>ensures result_1 &#61;&#61; BurnCapability&lt;CoinType&gt; &#123;&#125;;<br/>ensures result_2 &#61;&#61; FreezeCapability&lt;CoinType&gt; &#123;&#125;;<br/>ensures result_3 &#61;&#61; MintCapability&lt;CoinType&gt; &#123;&#125;;<br/></code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code>public fun merge&lt;CoinType&gt;(dst_coin: &amp;mut coin::Coin&lt;CoinType&gt;, source_coin: coin::Coin&lt;CoinType&gt;)<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures dst_coin.value &#61;&#61; old(dst_coin.value) &#43; source_coin.value;<br/></code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code>public fun mint&lt;CoinType&gt;(amount: u64, _cap: &amp;coin::MintCapability&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;<br/></code></pre>




<pre><code>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/></code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code>public fun register&lt;CoinType&gt;(account: &amp;signer)<br/></code></pre>


An account can only be registered once.
Updating <code>Account.guid_creation_num</code> will not overflow.


<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public entry fun transfer&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)<br/></code></pre>


<code>from</code> and <code>to</code> account not frozen.
<code>from</code> and <code>to</code> not the same address.
<code>from</code> account sufficient balance.


<pre><code>pragma verify &#61; false;<br/>let account_addr_from &#61; signer::address_of(from);<br/>let coin_store_from &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr_from);<br/>let post coin_store_post_from &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr_from);<br/>let coin_store_to &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(to);<br/>let post coin_store_post_to &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(to);<br/>// This enforces <a id="high-level-req-6.5" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr_from);<br/>aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(to);<br/>// This enforces <a id="high-level-req-8.2" href="#high-level-req">high-level requirement 8</a>:
aborts_if coin_store_from.frozen;<br/>aborts_if coin_store_to.frozen;<br/>aborts_if coin_store_from.coin.value &lt; amount;<br/>ensures account_addr_from !&#61; to &#61;&#61;&gt; coin_store_post_from.coin.value &#61;&#61;<br/>    coin_store_from.coin.value &#45; amount;<br/>ensures account_addr_from !&#61; to &#61;&#61;&gt; coin_store_post_to.coin.value &#61;&#61; coin_store_to.coin.value &#43; amount;<br/>ensures account_addr_from &#61;&#61; to &#61;&#61;&gt; coin_store_post_from.coin.value &#61;&#61; coin_store_from.coin.value;<br/></code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code>public fun withdraw&lt;CoinType&gt;(account: &amp;signer, amount: u64): coin::Coin&lt;CoinType&gt;<br/></code></pre>


Account is not frozen and sufficient balance.


<pre><code>pragma verify &#61; false;<br/>include WithdrawAbortsIf&lt;CoinType&gt;;<br/>modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let account_addr &#61; signer::address_of(account);<br/>let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let balance &#61; coin_store.coin.value;<br/>let post coin_post &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr).coin.value;<br/>ensures coin_post &#61;&#61; balance &#45; amount;<br/>ensures result &#61;&#61; Coin&lt;CoinType&gt; &#123; value: amount &#125;;<br/></code></pre>




<a id="0x1_coin_WithdrawAbortsIf"></a>


<pre><code>schema WithdrawAbortsIf&lt;CoinType&gt; &#123;<br/>account: &amp;signer;<br/>amount: u64;<br/>let account_addr &#61; signer::address_of(account);<br/>let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>let balance &#61; coin_store.coin.value;<br/>// This enforces <a id="high-level-req-6.6" href="#high-level-req">high-level requirement 6</a>:
    aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);<br/>// This enforces <a id="high-level-req-8.1" href="#high-level-req">high-level requirement 8</a>:
    aborts_if coin_store.frozen;<br/>aborts_if balance &lt; amount;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_mint_internal"></a>

### Function `mint_internal`


<pre><code>fun mint_internal&lt;CoinType&gt;(amount: u64): coin::Coin&lt;CoinType&gt;<br/></code></pre>




<pre><code>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/>aborts_if (amount !&#61; 0) &amp;&amp; !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/>ensures supply&lt;CoinType&gt; &#61;&#61; old(supply&lt;CoinType&gt;) &#43; amount;<br/>ensures result.value &#61;&#61; amount;<br/></code></pre>



<a id="@Specification_1_burn_internal"></a>

### Function `burn_internal`


<pre><code>fun burn_internal&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;<br/>modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
