
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


<pre><code>use 0x1::account;
use 0x1::aggregator;
use 0x1::aggregator_factory;
use 0x1::create_signer;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::fungible_asset;
use 0x1::guid;
use 0x1::object;
use 0x1::option;
use 0x1::optional_aggregator;
use 0x1::primary_fungible_store;
use 0x1::signer;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::table;
use 0x1::type_info;
</code></pre>



<a id="0x1_coin_Coin"></a>

## Struct `Coin`

Core data structures
Main structure representing a coin/token in an account's custody.


<pre><code>struct Coin&lt;CoinType&gt; has store
</code></pre>



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


<pre><code>struct AggregatableCoin&lt;CoinType&gt; has store
</code></pre>



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


<pre><code>struct CoinStore&lt;CoinType&gt; has key
</code></pre>



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


<pre><code>struct SupplyConfig has key
</code></pre>



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


<pre><code>struct CoinInfo&lt;CoinType&gt; has key
</code></pre>



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


<pre><code>struct DepositEvent has drop, store
</code></pre>



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


<pre><code>&#35;[event]
struct Deposit&lt;CoinType&gt; has drop, store
</code></pre>



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


<pre><code>struct WithdrawEvent has drop, store
</code></pre>



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


<pre><code>&#35;[event]
struct Withdraw&lt;CoinType&gt; has drop, store
</code></pre>



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


<pre><code>&#35;[event]
struct CoinEventHandleDeletion has drop, store
</code></pre>



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


<pre><code>&#35;[event]
struct PairCreation has drop, store
</code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct MigrationFlag has key
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

<a id="0x1_coin_MintCapability"></a>

## Struct `MintCapability`

Capability required to mint coins.


<pre><code>struct MintCapability&lt;CoinType&gt; has copy, store
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

<a id="0x1_coin_FreezeCapability"></a>

## Struct `FreezeCapability`

Capability required to freeze a coin store.


<pre><code>struct FreezeCapability&lt;CoinType&gt; has copy, store
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

<a id="0x1_coin_BurnCapability"></a>

## Struct `BurnCapability`

Capability required to burn coins.


<pre><code>struct BurnCapability&lt;CoinType&gt; has copy, store
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

<a id="0x1_coin_CoinConversionMap"></a>

## Resource `CoinConversionMap`

The mapping between coin and fungible asset.


<pre><code>struct CoinConversionMap has key
</code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct PairedCoinType has key
</code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct PairedFungibleAssetRefs has key
</code></pre>



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


<pre><code>struct MintRefReceipt
</code></pre>



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


<pre><code>struct TransferRefReceipt
</code></pre>



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


<pre><code>struct BurnRefReceipt
</code></pre>



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



<pre><code>struct Ghost$supply&lt;CoinType&gt; has copy, drop, store, key
</code></pre>



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



<pre><code>struct Ghost$aggregate_supply&lt;CoinType&gt; has copy, drop, store, key
</code></pre>



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


<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_coin_MAX_U128"></a>

Maximum possible coin supply.


<pre><code>const MAX_U128: u128 &#61; 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_coin_EINSUFFICIENT_BALANCE"></a>

Not enough coins to complete transaction


<pre><code>const EINSUFFICIENT_BALANCE: u64 &#61; 6;
</code></pre>



<a id="0x1_coin_EAGGREGATABLE_COIN_VALUE_TOO_LARGE"></a>

The value of aggregatable coin used for transaction fees redistribution does not fit in u64.


<pre><code>const EAGGREGATABLE_COIN_VALUE_TOO_LARGE: u64 &#61; 14;
</code></pre>



<a id="0x1_coin_EAPT_PAIRING_IS_NOT_ENABLED"></a>

APT pairing is not eanbled yet.


<pre><code>const EAPT_PAIRING_IS_NOT_ENABLED: u64 &#61; 28;
</code></pre>



<a id="0x1_coin_EBURN_REF_NOT_FOUND"></a>

The BurnRef does not exist.


<pre><code>const EBURN_REF_NOT_FOUND: u64 &#61; 25;
</code></pre>



<a id="0x1_coin_EBURN_REF_RECEIPT_MISMATCH"></a>

The BurnRefReceipt does not match the BurnRef to be returned.


<pre><code>const EBURN_REF_RECEIPT_MISMATCH: u64 &#61; 24;
</code></pre>



<a id="0x1_coin_ECOIN_CONVERSION_MAP_NOT_FOUND"></a>

The coin converison map is not created yet.


<pre><code>const ECOIN_CONVERSION_MAP_NOT_FOUND: u64 &#61; 27;
</code></pre>



<a id="0x1_coin_ECOIN_INFO_ADDRESS_MISMATCH"></a>

Address of account which is used to initialize a coin <code>CoinType</code> doesn't match the deployer of module


<pre><code>const ECOIN_INFO_ADDRESS_MISMATCH: u64 &#61; 1;
</code></pre>



<a id="0x1_coin_ECOIN_INFO_ALREADY_PUBLISHED"></a>

<code>CoinType</code> is already initialized as a coin


<pre><code>const ECOIN_INFO_ALREADY_PUBLISHED: u64 &#61; 2;
</code></pre>



<a id="0x1_coin_ECOIN_INFO_NOT_PUBLISHED"></a>

<code>CoinType</code> hasn't been initialized as a coin


<pre><code>const ECOIN_INFO_NOT_PUBLISHED: u64 &#61; 3;
</code></pre>



<a id="0x1_coin_ECOIN_NAME_TOO_LONG"></a>

Name of the coin is too long


<pre><code>const ECOIN_NAME_TOO_LONG: u64 &#61; 12;
</code></pre>



<a id="0x1_coin_ECOIN_STORE_ALREADY_PUBLISHED"></a>

Deprecated. Account already has <code>CoinStore</code> registered for <code>CoinType</code>


<pre><code>const ECOIN_STORE_ALREADY_PUBLISHED: u64 &#61; 4;
</code></pre>



<a id="0x1_coin_ECOIN_STORE_NOT_PUBLISHED"></a>

Account hasn't registered <code>CoinStore</code> for <code>CoinType</code>


<pre><code>const ECOIN_STORE_NOT_PUBLISHED: u64 &#61; 5;
</code></pre>



<a id="0x1_coin_ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED"></a>

Cannot upgrade the total supply of coins to different implementation.


<pre><code>const ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED: u64 &#61; 11;
</code></pre>



<a id="0x1_coin_ECOIN_SYMBOL_TOO_LONG"></a>

Symbol of the coin is too long


<pre><code>const ECOIN_SYMBOL_TOO_LONG: u64 &#61; 13;
</code></pre>



<a id="0x1_coin_ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED"></a>

The feature of migration from coin to fungible asset is not enabled.


<pre><code>const ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED: u64 &#61; 18;
</code></pre>



<a id="0x1_coin_ECOIN_TYPE_MISMATCH"></a>

The coin type from the map does not match the calling function type argument.


<pre><code>const ECOIN_TYPE_MISMATCH: u64 &#61; 17;
</code></pre>



<a id="0x1_coin_EDESTRUCTION_OF_NONZERO_TOKEN"></a>

Cannot destroy non-zero coins


<pre><code>const EDESTRUCTION_OF_NONZERO_TOKEN: u64 &#61; 7;
</code></pre>



<a id="0x1_coin_EFROZEN"></a>

CoinStore is frozen. Coins cannot be deposited or withdrawn


<pre><code>const EFROZEN: u64 &#61; 10;
</code></pre>



<a id="0x1_coin_EMIGRATION_FRAMEWORK_NOT_ENABLED"></a>

The migration process from coin to fungible asset is not enabled yet.


<pre><code>const EMIGRATION_FRAMEWORK_NOT_ENABLED: u64 &#61; 26;
</code></pre>



<a id="0x1_coin_EMINT_REF_NOT_FOUND"></a>

The MintRef does not exist.


<pre><code>const EMINT_REF_NOT_FOUND: u64 &#61; 21;
</code></pre>



<a id="0x1_coin_EMINT_REF_RECEIPT_MISMATCH"></a>

The MintRefReceipt does not match the MintRef to be returned.


<pre><code>const EMINT_REF_RECEIPT_MISMATCH: u64 &#61; 20;
</code></pre>



<a id="0x1_coin_EPAIRED_COIN"></a>

Error regarding paired coin type of the fungible asset metadata.


<pre><code>const EPAIRED_COIN: u64 &#61; 15;
</code></pre>



<a id="0x1_coin_EPAIRED_FUNGIBLE_ASSET"></a>

Error regarding paired fungible asset metadata of a coin type.


<pre><code>const EPAIRED_FUNGIBLE_ASSET: u64 &#61; 16;
</code></pre>



<a id="0x1_coin_EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND"></a>

PairedFungibleAssetRefs resource does not exist.


<pre><code>const EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND: u64 &#61; 19;
</code></pre>



<a id="0x1_coin_ETRANSFER_REF_NOT_FOUND"></a>

The TransferRef does not exist.


<pre><code>const ETRANSFER_REF_NOT_FOUND: u64 &#61; 23;
</code></pre>



<a id="0x1_coin_ETRANSFER_REF_RECEIPT_MISMATCH"></a>

The TransferRefReceipt does not match the TransferRef to be returned.


<pre><code>const ETRANSFER_REF_RECEIPT_MISMATCH: u64 &#61; 22;
</code></pre>



<a id="0x1_coin_MAX_COIN_NAME_LENGTH"></a>



<pre><code>const MAX_COIN_NAME_LENGTH: u64 &#61; 32;
</code></pre>



<a id="0x1_coin_MAX_COIN_SYMBOL_LENGTH"></a>



<pre><code>const MAX_COIN_SYMBOL_LENGTH: u64 &#61; 10;
</code></pre>



<a id="0x1_coin_paired_metadata"></a>

## Function `paired_metadata`

Get the paired fungible asset metadata object of a coin type. If not exist, return option::none().


<pre><code>&#35;[view]
public fun paired_metadata&lt;CoinType&gt;(): option::Option&lt;object::Object&lt;fungible_asset::Metadata&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_metadata&lt;CoinType&gt;(): Option&lt;Object&lt;Metadata&gt;&gt; acquires CoinConversionMap &#123;
    if (exists&lt;CoinConversionMap&gt;(@aptos_framework) &amp;&amp; features::coin_to_fungible_asset_migration_feature_enabled(
    )) &#123;
        let map &#61; &amp;borrow_global&lt;CoinConversionMap&gt;(@aptos_framework).coin_to_fungible_asset_map;
        let type &#61; type_info::type_of&lt;CoinType&gt;();
        if (table::contains(map, type)) &#123;
            return option::some(&#42;table::borrow(map, type))
        &#125;
    &#125;;
    option::none()
&#125;
</code></pre>



</details>

<a id="0x1_coin_create_coin_conversion_map"></a>

## Function `create_coin_conversion_map`



<pre><code>public entry fun create_coin_conversion_map(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_coin_conversion_map(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    if (!exists&lt;CoinConversionMap&gt;(@aptos_framework)) &#123;
        move_to(aptos_framework, CoinConversionMap &#123;
            coin_to_fungible_asset_map: table::new(),
        &#125;)
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_coin_create_pairing"></a>

## Function `create_pairing`

Create APT pairing by passing <code>AptosCoin</code>.


<pre><code>public entry fun create_pairing&lt;CoinType&gt;(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_pairing&lt;CoinType&gt;(
    aptos_framework: &amp;signer
) acquires CoinConversionMap, CoinInfo &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(true);
&#125;
</code></pre>



</details>

<a id="0x1_coin_is_apt"></a>

## Function `is_apt`



<pre><code>fun is_apt&lt;CoinType&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun is_apt&lt;CoinType&gt;(): bool &#123;
    type_info::type_name&lt;CoinType&gt;() &#61;&#61; string::utf8(b&quot;0x1::aptos_coin::AptosCoin&quot;)
&#125;
</code></pre>



</details>

<a id="0x1_coin_create_and_return_paired_metadata_if_not_exist"></a>

## Function `create_and_return_paired_metadata_if_not_exist`



<pre><code>fun create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(allow_apt_creation: bool): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(allow_apt_creation: bool): Object&lt;Metadata&gt; &#123;
    assert!(
        features::coin_to_fungible_asset_migration_feature_enabled(),
        error::invalid_state(EMIGRATION_FRAMEWORK_NOT_ENABLED)
    );
    assert!(exists&lt;CoinConversionMap&gt;(@aptos_framework), error::not_found(ECOIN_CONVERSION_MAP_NOT_FOUND));
    let map &#61; borrow_global_mut&lt;CoinConversionMap&gt;(@aptos_framework);
    let type &#61; type_info::type_of&lt;CoinType&gt;();
    if (!table::contains(&amp;map.coin_to_fungible_asset_map, type)) &#123;
        let is_apt &#61; is_apt&lt;CoinType&gt;();
        assert!(!is_apt &#124;&#124; allow_apt_creation, error::invalid_state(EAPT_PAIRING_IS_NOT_ENABLED));
        let metadata_object_cref &#61;
            if (is_apt) &#123;
                object::create_sticky_object_at_address(@aptos_framework, @aptos_fungible_asset)
            &#125; else &#123;
                object::create_named_object(
                    &amp;create_signer::create_signer(@aptos_fungible_asset),
                    &#42;string::bytes(&amp;type_info::type_name&lt;CoinType&gt;())
                )
            &#125;;
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &amp;metadata_object_cref,
            option::map(coin_supply&lt;CoinType&gt;(), &#124;_&#124; MAX_U128),
            name&lt;CoinType&gt;(),
            symbol&lt;CoinType&gt;(),
            decimals&lt;CoinType&gt;(),
            string::utf8(b&quot;&quot;),
            string::utf8(b&quot;&quot;),
        );

        let metadata_object_signer &#61; &amp;object::generate_signer(&amp;metadata_object_cref);
        let type &#61; type_info::type_of&lt;CoinType&gt;();
        move_to(metadata_object_signer, PairedCoinType &#123; type &#125;);
        let metadata_obj &#61; object::object_from_constructor_ref(&amp;metadata_object_cref);

        table::add(&amp;mut map.coin_to_fungible_asset_map, type, metadata_obj);
        event::emit(PairCreation &#123;
            coin_type: type,
            fungible_asset_metadata_address: object_address(&amp;metadata_obj)
        &#125;);

        // Generates all three refs
        let mint_ref &#61; fungible_asset::generate_mint_ref(&amp;metadata_object_cref);
        let transfer_ref &#61; fungible_asset::generate_transfer_ref(&amp;metadata_object_cref);
        let burn_ref &#61; fungible_asset::generate_burn_ref(&amp;metadata_object_cref);
        move_to(metadata_object_signer,
            PairedFungibleAssetRefs &#123;
                mint_ref_opt: option::some(mint_ref),
                transfer_ref_opt: option::some(transfer_ref),
                burn_ref_opt: option::some(burn_ref),
            &#125;
        );
    &#125;;
    &#42;table::borrow(&amp;map.coin_to_fungible_asset_map, type)
&#125;
</code></pre>



</details>

<a id="0x1_coin_ensure_paired_metadata"></a>

## Function `ensure_paired_metadata`

Get the paired fungible asset metadata object of a coin type, create if not exist.


<pre><code>public(friend) fun ensure_paired_metadata&lt;CoinType&gt;(): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun ensure_paired_metadata&lt;CoinType&gt;(): Object&lt;Metadata&gt; acquires CoinConversionMap, CoinInfo &#123;
    create_and_return_paired_metadata_if_not_exist&lt;CoinType&gt;(false)
&#125;
</code></pre>



</details>

<a id="0x1_coin_paired_coin"></a>

## Function `paired_coin`

Get the paired coin type of a fungible asset metadata object.


<pre><code>&#35;[view]
public fun paired_coin(metadata: object::Object&lt;fungible_asset::Metadata&gt;): option::Option&lt;type_info::TypeInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_coin(metadata: Object&lt;Metadata&gt;): Option&lt;TypeInfo&gt; acquires PairedCoinType &#123;
    let metadata_addr &#61; object::object_address(&amp;metadata);
    if (exists&lt;PairedCoinType&gt;(metadata_addr)) &#123;
        option::some(borrow_global&lt;PairedCoinType&gt;(metadata_addr).type)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_coin_to_fungible_asset"></a>

## Function `coin_to_fungible_asset`

Conversion from coin to fungible asset


<pre><code>public fun coin_to_fungible_asset&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun coin_to_fungible_asset&lt;CoinType&gt;(
    coin: Coin&lt;CoinType&gt;
): FungibleAsset acquires CoinConversionMap, CoinInfo &#123;
    let metadata &#61; ensure_paired_metadata&lt;CoinType&gt;();
    let amount &#61; burn_internal(coin);
    fungible_asset::mint_internal(metadata, amount)
&#125;
</code></pre>



</details>

<a id="0x1_coin_fungible_asset_to_coin"></a>

## Function `fungible_asset_to_coin`

Conversion from fungible asset to coin. Not public to push the migration to FA.


<pre><code>fun fungible_asset_to_coin&lt;CoinType&gt;(fungible_asset: fungible_asset::FungibleAsset): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun fungible_asset_to_coin&lt;CoinType&gt;(
    fungible_asset: FungibleAsset
): Coin&lt;CoinType&gt; acquires CoinInfo, PairedCoinType &#123;
    let metadata_addr &#61; object::object_address(&amp;fungible_asset::metadata_from_asset(&amp;fungible_asset));
    assert!(
        object::object_exists&lt;PairedCoinType&gt;(metadata_addr),
        error::not_found(EPAIRED_COIN)
    );
    let coin_type_info &#61; borrow_global&lt;PairedCoinType&gt;(metadata_addr).type;
    assert!(coin_type_info &#61;&#61; type_info::type_of&lt;CoinType&gt;(), error::invalid_argument(ECOIN_TYPE_MISMATCH));
    let amount &#61; fungible_asset::burn_internal(fungible_asset);
    mint_internal&lt;CoinType&gt;(amount)
&#125;
</code></pre>



</details>

<a id="0x1_coin_assert_paired_metadata_exists"></a>

## Function `assert_paired_metadata_exists`



<pre><code>fun assert_paired_metadata_exists&lt;CoinType&gt;(): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_paired_metadata_exists&lt;CoinType&gt;(): Object&lt;Metadata&gt; &#123;
    let metadata_opt &#61; paired_metadata&lt;CoinType&gt;();
    assert!(option::is_some(&amp;metadata_opt), error::not_found(EPAIRED_FUNGIBLE_ASSET));
    option::destroy_some(metadata_opt)
&#125;
</code></pre>



</details>

<a id="0x1_coin_paired_mint_ref_exists"></a>

## Function `paired_mint_ref_exists`

Check whether <code>MintRef</code> has not been taken.


<pre><code>&#35;[view]
public fun paired_mint_ref_exists&lt;CoinType&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_mint_ref_exists&lt;CoinType&gt;(): bool acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    option::is_some(&amp;borrow_global&lt;PairedFungibleAssetRefs&gt;(metadata_addr).mint_ref_opt)
&#125;
</code></pre>



</details>

<a id="0x1_coin_get_paired_mint_ref"></a>

## Function `get_paired_mint_ref`

Get the <code>MintRef</code> of paired fungible asset of a coin type from <code>MintCapability</code>.


<pre><code>public fun get_paired_mint_ref&lt;CoinType&gt;(_: &amp;coin::MintCapability&lt;CoinType&gt;): (fungible_asset::MintRef, coin::MintRefReceipt)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_paired_mint_ref&lt;CoinType&gt;(
    _: &amp;MintCapability&lt;CoinType&gt;
): (MintRef, MintRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    let mint_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).mint_ref_opt;
    assert!(option::is_some(mint_ref_opt), error::not_found(EMINT_REF_NOT_FOUND));
    (option::extract(mint_ref_opt), MintRefReceipt &#123; metadata &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_coin_return_paired_mint_ref"></a>

## Function `return_paired_mint_ref`

Return the <code>MintRef</code> with the hot potato receipt.


<pre><code>public fun return_paired_mint_ref(mint_ref: fungible_asset::MintRef, receipt: coin::MintRefReceipt)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun return_paired_mint_ref(mint_ref: MintRef, receipt: MintRefReceipt) acquires PairedFungibleAssetRefs &#123;
    let MintRefReceipt &#123; metadata &#125; &#61; receipt;
    assert!(
        fungible_asset::mint_ref_metadata(&amp;mint_ref) &#61;&#61; metadata,
        error::invalid_argument(EMINT_REF_RECEIPT_MISMATCH)
    );
    let metadata_addr &#61; object_address(&amp;metadata);
    let mint_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).mint_ref_opt;
    option::fill(mint_ref_opt, mint_ref);
&#125;
</code></pre>



</details>

<a id="0x1_coin_paired_transfer_ref_exists"></a>

## Function `paired_transfer_ref_exists`

Check whether <code>TransferRef</code> still exists.


<pre><code>&#35;[view]
public fun paired_transfer_ref_exists&lt;CoinType&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_transfer_ref_exists&lt;CoinType&gt;(): bool acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    option::is_some(&amp;borrow_global&lt;PairedFungibleAssetRefs&gt;(metadata_addr).transfer_ref_opt)
&#125;
</code></pre>



</details>

<a id="0x1_coin_get_paired_transfer_ref"></a>

## Function `get_paired_transfer_ref`

Get the TransferRef of paired fungible asset of a coin type from <code>FreezeCapability</code>.


<pre><code>public fun get_paired_transfer_ref&lt;CoinType&gt;(_: &amp;coin::FreezeCapability&lt;CoinType&gt;): (fungible_asset::TransferRef, coin::TransferRefReceipt)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_paired_transfer_ref&lt;CoinType&gt;(
    _: &amp;FreezeCapability&lt;CoinType&gt;
): (TransferRef, TransferRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    let transfer_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).transfer_ref_opt;
    assert!(option::is_some(transfer_ref_opt), error::not_found(ETRANSFER_REF_NOT_FOUND));
    (option::extract(transfer_ref_opt), TransferRefReceipt &#123; metadata &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_coin_return_paired_transfer_ref"></a>

## Function `return_paired_transfer_ref`

Return the <code>TransferRef</code> with the hot potato receipt.


<pre><code>public fun return_paired_transfer_ref(transfer_ref: fungible_asset::TransferRef, receipt: coin::TransferRefReceipt)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun return_paired_transfer_ref(
    transfer_ref: TransferRef,
    receipt: TransferRefReceipt
) acquires PairedFungibleAssetRefs &#123;
    let TransferRefReceipt &#123; metadata &#125; &#61; receipt;
    assert!(
        fungible_asset::transfer_ref_metadata(&amp;transfer_ref) &#61;&#61; metadata,
        error::invalid_argument(ETRANSFER_REF_RECEIPT_MISMATCH)
    );
    let metadata_addr &#61; object_address(&amp;metadata);
    let transfer_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).transfer_ref_opt;
    option::fill(transfer_ref_opt, transfer_ref);
&#125;
</code></pre>



</details>

<a id="0x1_coin_paired_burn_ref_exists"></a>

## Function `paired_burn_ref_exists`

Check whether <code>BurnRef</code> has not been taken.


<pre><code>&#35;[view]
public fun paired_burn_ref_exists&lt;CoinType&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun paired_burn_ref_exists&lt;CoinType&gt;(): bool acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    option::is_some(&amp;borrow_global&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt)
&#125;
</code></pre>



</details>

<a id="0x1_coin_get_paired_burn_ref"></a>

## Function `get_paired_burn_ref`

Get the <code>BurnRef</code> of paired fungible asset of a coin type from <code>BurnCapability</code>.


<pre><code>public fun get_paired_burn_ref&lt;CoinType&gt;(_: &amp;coin::BurnCapability&lt;CoinType&gt;): (fungible_asset::BurnRef, coin::BurnRefReceipt)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_paired_burn_ref&lt;CoinType&gt;(
    _: &amp;BurnCapability&lt;CoinType&gt;
): (BurnRef, BurnRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    let burn_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt;
    assert!(option::is_some(burn_ref_opt), error::not_found(EBURN_REF_NOT_FOUND));
    (option::extract(burn_ref_opt), BurnRefReceipt &#123; metadata &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_coin_return_paired_burn_ref"></a>

## Function `return_paired_burn_ref`

Return the <code>BurnRef</code> with the hot potato receipt.


<pre><code>public fun return_paired_burn_ref(burn_ref: fungible_asset::BurnRef, receipt: coin::BurnRefReceipt)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun return_paired_burn_ref(
    burn_ref: BurnRef,
    receipt: BurnRefReceipt
) acquires PairedFungibleAssetRefs &#123;
    let BurnRefReceipt &#123; metadata &#125; &#61; receipt;
    assert!(
        fungible_asset::burn_ref_metadata(&amp;burn_ref) &#61;&#61; metadata,
        error::invalid_argument(EBURN_REF_RECEIPT_MISMATCH)
    );
    let metadata_addr &#61; object_address(&amp;metadata);
    let burn_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt;
    option::fill(burn_ref_opt, burn_ref);
&#125;
</code></pre>



</details>

<a id="0x1_coin_borrow_paired_burn_ref"></a>

## Function `borrow_paired_burn_ref`



<pre><code>fun borrow_paired_burn_ref&lt;CoinType&gt;(_: &amp;coin::BurnCapability&lt;CoinType&gt;): &amp;fungible_asset::BurnRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_paired_burn_ref&lt;CoinType&gt;(
    _: &amp;BurnCapability&lt;CoinType&gt;
): &amp;BurnRef acquires CoinConversionMap, PairedFungibleAssetRefs &#123;
    let metadata &#61; assert_paired_metadata_exists&lt;CoinType&gt;();
    let metadata_addr &#61; object_address(&amp;metadata);
    assert!(exists&lt;PairedFungibleAssetRefs&gt;(metadata_addr), error::internal(EPAIRED_FUNGIBLE_ASSET_REFS_NOT_FOUND));
    let burn_ref_opt &#61; &amp;mut borrow_global_mut&lt;PairedFungibleAssetRefs&gt;(metadata_addr).burn_ref_opt;
    assert!(option::is_some(burn_ref_opt), error::not_found(EBURN_REF_NOT_FOUND));
    option::borrow(burn_ref_opt)
&#125;
</code></pre>



</details>

<a id="0x1_coin_initialize_supply_config"></a>

## Function `initialize_supply_config`

Publishes supply configuration. Initially, upgrading is not allowed.


<pre><code>public(friend) fun initialize_supply_config(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_supply_config(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    move_to(aptos_framework, SupplyConfig &#123; allow_upgrades: false &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_coin_allow_supply_upgrades"></a>

## Function `allow_supply_upgrades`

This should be called by on-chain governance to update the config and allow
or disallow upgradability of total supply.


<pre><code>public fun allow_supply_upgrades(aptos_framework: &amp;signer, allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun allow_supply_upgrades(aptos_framework: &amp;signer, allowed: bool) acquires SupplyConfig &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    let allow_upgrades &#61; &amp;mut borrow_global_mut&lt;SupplyConfig&gt;(@aptos_framework).allow_upgrades;
    &#42;allow_upgrades &#61; allowed;
&#125;
</code></pre>



</details>

<a id="0x1_coin_initialize_aggregatable_coin"></a>

## Function `initialize_aggregatable_coin`

Creates a new aggregatable coin with value overflowing on <code>limit</code>. Note that this function can
only be called by Aptos Framework (0x1) account for now because of <code>create_aggregator</code>.


<pre><code>public(friend) fun initialize_aggregatable_coin&lt;CoinType&gt;(aptos_framework: &amp;signer): coin::AggregatableCoin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_aggregatable_coin&lt;CoinType&gt;(aptos_framework: &amp;signer): AggregatableCoin&lt;CoinType&gt; &#123;
    let aggregator &#61; aggregator_factory::create_aggregator(aptos_framework, MAX_U64);
    AggregatableCoin&lt;CoinType&gt; &#123;
        value: aggregator,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_is_aggregatable_coin_zero"></a>

## Function `is_aggregatable_coin_zero`

Returns true if the value of aggregatable coin is zero.


<pre><code>public(friend) fun is_aggregatable_coin_zero&lt;CoinType&gt;(coin: &amp;coin::AggregatableCoin&lt;CoinType&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun is_aggregatable_coin_zero&lt;CoinType&gt;(coin: &amp;AggregatableCoin&lt;CoinType&gt;): bool &#123;
    let amount &#61; aggregator::read(&amp;coin.value);
    amount &#61;&#61; 0
&#125;
</code></pre>



</details>

<a id="0x1_coin_drain_aggregatable_coin"></a>

## Function `drain_aggregatable_coin`

Drains the aggregatable coin, setting it to zero and returning a standard coin.


<pre><code>public(friend) fun drain_aggregatable_coin&lt;CoinType&gt;(coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun drain_aggregatable_coin&lt;CoinType&gt;(coin: &amp;mut AggregatableCoin&lt;CoinType&gt;): Coin&lt;CoinType&gt; &#123;
    spec &#123;
        // TODO: The data invariant is not properly assumed from CollectedFeesPerBlock.
        assume aggregator::spec_get_limit(coin.value) &#61;&#61; MAX_U64;
    &#125;;
    let amount &#61; aggregator::read(&amp;coin.value);
    assert!(amount &lt;&#61; MAX_U64, error::out_of_range(EAGGREGATABLE_COIN_VALUE_TOO_LARGE));
    spec &#123;
        update aggregate_supply&lt;CoinType&gt; &#61; aggregate_supply&lt;CoinType&gt; &#45; amount;
    &#125;;
    aggregator::sub(&amp;mut coin.value, amount);
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; amount;
    &#125;;
    Coin&lt;CoinType&gt; &#123;
        value: (amount as u64),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_merge_aggregatable_coin"></a>

## Function `merge_aggregatable_coin`

Merges <code>coin</code> into aggregatable coin (<code>dst_coin</code>).


<pre><code>public(friend) fun merge_aggregatable_coin&lt;CoinType&gt;(dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;, coin: coin::Coin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun merge_aggregatable_coin&lt;CoinType&gt;(
    dst_coin: &amp;mut AggregatableCoin&lt;CoinType&gt;,
    coin: Coin&lt;CoinType&gt;
) &#123;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; coin.value;
    &#125;;
    let Coin &#123; value &#125; &#61; coin;
    let amount &#61; (value as u128);
    spec &#123;
        update aggregate_supply&lt;CoinType&gt; &#61; aggregate_supply&lt;CoinType&gt; &#43; amount;
    &#125;;
    aggregator::add(&amp;mut dst_coin.value, amount);
&#125;
</code></pre>



</details>

<a id="0x1_coin_collect_into_aggregatable_coin"></a>

## Function `collect_into_aggregatable_coin`

Collects a specified amount of coin form an account into aggregatable coin.


<pre><code>public(friend) fun collect_into_aggregatable_coin&lt;CoinType&gt;(account_addr: address, amount: u64, dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun collect_into_aggregatable_coin&lt;CoinType&gt;(
    account_addr: address,
    amount: u64,
    dst_coin: &amp;mut AggregatableCoin&lt;CoinType&gt;,
) acquires CoinStore, CoinConversionMap, CoinInfo, PairedCoinType &#123;
    // Skip collecting if amount is zero.
    if (amount &#61;&#61; 0) &#123;
        return
    &#125;;

    let (coin_amount_to_collect, fa_amount_to_collect) &#61; calculate_amount_to_withdraw&lt;CoinType&gt;(
        account_addr,
        amount
    );
    let coin &#61; if (coin_amount_to_collect &gt; 0) &#123;
        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
        extract(&amp;mut coin_store.coin, coin_amount_to_collect)
    &#125; else &#123;
        zero()
    &#125;;
    if (fa_amount_to_collect &gt; 0) &#123;
        let store_addr &#61; primary_fungible_store::primary_store_address(
            account_addr,
            option::destroy_some(paired_metadata&lt;CoinType&gt;())
        );
        let fa &#61; fungible_asset::withdraw_internal(store_addr, fa_amount_to_collect);
        merge(&amp;mut coin, fungible_asset_to_coin&lt;CoinType&gt;(fa));
    &#125;;
    merge_aggregatable_coin(dst_coin, coin);
&#125;
</code></pre>



</details>

<a id="0x1_coin_calculate_amount_to_withdraw"></a>

## Function `calculate_amount_to_withdraw`



<pre><code>fun calculate_amount_to_withdraw&lt;CoinType&gt;(account_addr: address, amount: u64): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun calculate_amount_to_withdraw&lt;CoinType&gt;(
    account_addr: address,
    amount: u64
): (u64, u64) &#123;
    let coin_balance &#61; coin_balance&lt;CoinType&gt;(account_addr);
    if (coin_balance &gt;&#61; amount) &#123;
        (amount, 0)
    &#125; else &#123;
        let metadata &#61; paired_metadata&lt;CoinType&gt;();
        if (option::is_some(&amp;metadata) &amp;&amp; primary_fungible_store::primary_store_exists(
            account_addr,
            option::destroy_some(metadata)
        ))
            (coin_balance, amount &#45; coin_balance)
        else
            abort error::invalid_argument(EINSUFFICIENT_BALANCE)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_maybe_convert_to_fungible_store"></a>

## Function `maybe_convert_to_fungible_store`



<pre><code>fun maybe_convert_to_fungible_store&lt;CoinType&gt;(account: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun maybe_convert_to_fungible_store&lt;CoinType&gt;(account: address) acquires CoinStore, CoinConversionMap, CoinInfo &#123;
    if (!features::coin_to_fungible_asset_migration_feature_enabled()) &#123;
        abort error::unavailable(ECOIN_TO_FUNGIBLE_ASSET_FEATURE_NOT_ENABLED)
    &#125;;
    assert!(is_coin_initialized&lt;CoinType&gt;(), error::invalid_argument(ECOIN_INFO_NOT_PUBLISHED));

    let metadata &#61; ensure_paired_metadata&lt;CoinType&gt;();
    let store &#61; primary_fungible_store::ensure_primary_store_exists(account, metadata);
    let store_address &#61; object::object_address(&amp;store);
    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account)) &#123;
        let CoinStore&lt;CoinType&gt; &#123; coin, frozen, deposit_events, withdraw_events &#125; &#61; move_from&lt;CoinStore&lt;CoinType&gt;&gt;(
            account
        );
        event::emit(
            CoinEventHandleDeletion &#123;
                event_handle_creation_address: guid::creator_address(
                    event::guid(&amp;deposit_events)
                ),
                deleted_deposit_event_handle_creation_number: guid::creation_num(event::guid(&amp;deposit_events)),
                deleted_withdraw_event_handle_creation_number: guid::creation_num(event::guid(&amp;withdraw_events))
            &#125;
        );
        event::destroy_handle(deposit_events);
        event::destroy_handle(withdraw_events);
        if (coin.value &#61;&#61; 0) &#123;
            destroy_zero(coin);
        &#125; else &#123;
            fungible_asset::deposit(store, coin_to_fungible_asset(coin));
        &#125;;
        // Note:
        // It is possible the primary fungible store may already exist before this function call.
        // In this case, if the account owns a frozen CoinStore and an unfrozen primary fungible store, this
        // function would convert and deposit the rest coin into the primary store and freeze it to make the
        // `frozen` semantic as consistent as possible.
        if (frozen !&#61; fungible_asset::is_frozen(store)) &#123;
            fungible_asset::set_frozen_flag_internal(store, frozen);
        &#125;
    &#125;;
    if (!exists&lt;MigrationFlag&gt;(store_address)) &#123;
        move_to(&amp;create_signer::create_signer(store_address), MigrationFlag &#123;&#125;);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_migrate_to_fungible_store"></a>

## Function `migrate_to_fungible_store`

Voluntarily migrate to fungible store for <code>CoinType</code> if not yet.


<pre><code>public entry fun migrate_to_fungible_store&lt;CoinType&gt;(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun migrate_to_fungible_store&lt;CoinType&gt;(
    account: &amp;signer
) acquires CoinStore, CoinConversionMap, CoinInfo &#123;
    maybe_convert_to_fungible_store&lt;CoinType&gt;(signer::address_of(account));
&#125;
</code></pre>



</details>

<a id="0x1_coin_coin_address"></a>

## Function `coin_address`

A helper function that returns the address of CoinType.


<pre><code>fun coin_address&lt;CoinType&gt;(): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun coin_address&lt;CoinType&gt;(): address &#123;
    let type_info &#61; type_info::type_of&lt;CoinType&gt;();
    type_info::account_address(&amp;type_info)
&#125;
</code></pre>



</details>

<a id="0x1_coin_balance"></a>

## Function `balance`

Returns the balance of <code>owner</code> for provided <code>CoinType</code> and its paired FA if exists.


<pre><code>&#35;[view]
public fun balance&lt;CoinType&gt;(owner: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance&lt;CoinType&gt;(owner: address): u64 acquires CoinConversionMap, CoinStore &#123;
    let paired_metadata &#61; paired_metadata&lt;CoinType&gt;();
    coin_balance&lt;CoinType&gt;(owner) &#43; if (option::is_some(&amp;paired_metadata)) &#123;
        primary_fungible_store::balance(
            owner,
            option::extract(&amp;mut paired_metadata)
        )
    &#125; else &#123; 0 &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_is_balance_at_least"></a>

## Function `is_balance_at_least`

Returns whether the balance of <code>owner</code> for provided <code>CoinType</code> and its paired FA is >= <code>amount</code>.


<pre><code>&#35;[view]
public fun is_balance_at_least&lt;CoinType&gt;(owner: address, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_balance_at_least&lt;CoinType&gt;(owner: address, amount: u64): bool acquires CoinConversionMap, CoinStore &#123;
    let coin_balance &#61; coin_balance&lt;CoinType&gt;(owner);
    if (coin_balance &gt;&#61; amount) &#123;
        return true
    &#125;;

    let paired_metadata &#61; paired_metadata&lt;CoinType&gt;();
    let left_amount &#61; amount &#45; coin_balance;
    if (option::is_some(&amp;paired_metadata)) &#123;
        primary_fungible_store::is_balance_at_least(
            owner,
            option::extract(&amp;mut paired_metadata),
            left_amount
        )
    &#125; else &#123; false &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_coin_balance"></a>

## Function `coin_balance`



<pre><code>fun coin_balance&lt;CoinType&gt;(owner: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun coin_balance&lt;CoinType&gt;(owner: address): u64 &#123;
    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(owner)) &#123;
        borrow_global&lt;CoinStore&lt;CoinType&gt;&gt;(owner).coin.value
    &#125; else &#123;
        0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_is_coin_initialized"></a>

## Function `is_coin_initialized`

Returns <code>true</code> if the type <code>CoinType</code> is an initialized coin.


<pre><code>&#35;[view]
public fun is_coin_initialized&lt;CoinType&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_coin_initialized&lt;CoinType&gt;(): bool &#123;
    exists&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;())
&#125;
</code></pre>



</details>

<a id="0x1_coin_is_coin_store_frozen"></a>

## Function `is_coin_store_frozen`

Returns <code>true</code> is account_addr has frozen the CoinStore or if it's not registered at all


<pre><code>&#35;[view]
public fun is_coin_store_frozen&lt;CoinType&gt;(account_addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_coin_store_frozen&lt;CoinType&gt;(
    account_addr: address
): bool acquires CoinStore, CoinConversionMap &#123;
    if (!is_account_registered&lt;CoinType&gt;(account_addr)) &#123;
        return true
    &#125;;

    let coin_store &#61; borrow_global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    coin_store.frozen
&#125;
</code></pre>



</details>

<a id="0x1_coin_is_account_registered"></a>

## Function `is_account_registered`

Returns <code>true</code> if <code>account_addr</code> is registered to receive <code>CoinType</code>.


<pre><code>&#35;[view]
public fun is_account_registered&lt;CoinType&gt;(account_addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_account_registered&lt;CoinType&gt;(account_addr: address): bool acquires CoinConversionMap &#123;
    assert!(is_coin_initialized&lt;CoinType&gt;(), error::invalid_argument(ECOIN_INFO_NOT_PUBLISHED));
    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)) &#123;
        true
    &#125; else &#123;
        let paired_metadata_opt &#61; paired_metadata&lt;CoinType&gt;();
        (option::is_some(
            &amp;paired_metadata_opt
        ) &amp;&amp; migrated_primary_fungible_store_exists(account_addr, option::destroy_some(paired_metadata_opt)))
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_name"></a>

## Function `name`

Returns the name of the coin.


<pre><code>&#35;[view]
public fun name&lt;CoinType&gt;(): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;CoinType&gt;(): string::String acquires CoinInfo &#123;
    borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).name
&#125;
</code></pre>



</details>

<a id="0x1_coin_symbol"></a>

## Function `symbol`

Returns the symbol of the coin, usually a shorter version of the name.


<pre><code>&#35;[view]
public fun symbol&lt;CoinType&gt;(): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun symbol&lt;CoinType&gt;(): string::String acquires CoinInfo &#123;
    borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).symbol
&#125;
</code></pre>



</details>

<a id="0x1_coin_decimals"></a>

## Function `decimals`

Returns the number of decimals used to get its user representation.
For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).


<pre><code>&#35;[view]
public fun decimals&lt;CoinType&gt;(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun decimals&lt;CoinType&gt;(): u8 acquires CoinInfo &#123;
    borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).decimals
&#125;
</code></pre>



</details>

<a id="0x1_coin_supply"></a>

## Function `supply`

Returns the amount of coin in existence.


<pre><code>&#35;[view]
public fun supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun supply&lt;CoinType&gt;(): Option&lt;u128&gt; acquires CoinInfo, CoinConversionMap &#123;
    let coin_supply &#61; coin_supply&lt;CoinType&gt;();
    let metadata &#61; paired_metadata&lt;CoinType&gt;();
    if (option::is_some(&amp;metadata)) &#123;
        let fungible_asset_supply &#61; fungible_asset::supply(option::extract(&amp;mut metadata));
        if (option::is_some(&amp;coin_supply)) &#123;
            let supply &#61; option::borrow_mut(&amp;mut coin_supply);
            &#42;supply &#61; &#42;supply &#43; option::destroy_some(fungible_asset_supply);
        &#125;;
    &#125;;
    coin_supply
&#125;
</code></pre>



</details>

<a id="0x1_coin_coin_supply"></a>

## Function `coin_supply`

Returns the amount of coin in existence.


<pre><code>&#35;[view]
public fun coin_supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun coin_supply&lt;CoinType&gt;(): Option&lt;u128&gt; acquires CoinInfo &#123;
    let maybe_supply &#61; &amp;borrow_global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).supply;
    if (option::is_some(maybe_supply)) &#123;
        // We do track supply, in this case read from optional aggregator.
        let supply &#61; option::borrow(maybe_supply);
        let value &#61; optional_aggregator::read(supply);
        option::some(value)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_burn"></a>

## Function `burn`

Burn <code>coin</code> with capability.
The capability <code>_cap</code> should be passed as a reference to <code>BurnCapability&lt;CoinType&gt;</code>.


<pre><code>public fun burn&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;, _cap: &amp;coin::BurnCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn&lt;CoinType&gt;(coin: Coin&lt;CoinType&gt;, _cap: &amp;BurnCapability&lt;CoinType&gt;) acquires CoinInfo &#123;
    burn_internal(coin);
&#125;
</code></pre>



</details>

<a id="0x1_coin_burn_from"></a>

## Function `burn_from`

Burn <code>coin</code> from the specified <code>account</code> with capability.
The capability <code>burn_cap</code> should be passed as a reference to <code>BurnCapability&lt;CoinType&gt;</code>.
This function shouldn't fail as it's called as part of transaction fee burning.

Note: This bypasses CoinStore::frozen -- coins within a frozen CoinStore can be burned.


<pre><code>public fun burn_from&lt;CoinType&gt;(account_addr: address, amount: u64, burn_cap: &amp;coin::BurnCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn_from&lt;CoinType&gt;(
    account_addr: address,
    amount: u64,
    burn_cap: &amp;BurnCapability&lt;CoinType&gt;,
) acquires CoinInfo, CoinStore, CoinConversionMap, PairedFungibleAssetRefs &#123;
    // Skip burning if amount is zero. This shouldn&apos;t error out as it&apos;s called as part of transaction fee burning.
    if (amount &#61;&#61; 0) &#123;
        return
    &#125;;

    let (coin_amount_to_burn, fa_amount_to_burn) &#61; calculate_amount_to_withdraw&lt;CoinType&gt;(
        account_addr,
        amount
    );
    if (coin_amount_to_burn &gt; 0) &#123;
        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
        let coin_to_burn &#61; extract(&amp;mut coin_store.coin, coin_amount_to_burn);
        burn(coin_to_burn, burn_cap);
    &#125;;
    if (fa_amount_to_burn &gt; 0) &#123;
        fungible_asset::burn_from(
            borrow_paired_burn_ref(burn_cap),
            primary_fungible_store::primary_store(account_addr, option::destroy_some(paired_metadata&lt;CoinType&gt;())),
            fa_amount_to_burn
        );
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_coin_deposit"></a>

## Function `deposit`

Deposit the coin balance into the recipient's account and emit an event.


<pre><code>public fun deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit&lt;CoinType&gt;(
    account_addr: address,
    coin: Coin&lt;CoinType&gt;
) acquires CoinStore, CoinConversionMap, CoinInfo &#123;
    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)) &#123;
        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
        assert!(
            !coin_store.frozen,
            error::permission_denied(EFROZEN),
        );
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(Deposit&lt;CoinType&gt; &#123; account: account_addr, amount: coin.value &#125;);
        &#125;;
        event::emit_event&lt;DepositEvent&gt;(
            &amp;mut coin_store.deposit_events,
            DepositEvent &#123; amount: coin.value &#125;,
        );
        merge(&amp;mut coin_store.coin, coin);
    &#125; else &#123;
        let metadata &#61; paired_metadata&lt;CoinType&gt;();
        if (option::is_some(&amp;metadata) &amp;&amp; migrated_primary_fungible_store_exists(
            account_addr,
            option::destroy_some(metadata)
        )) &#123;
            primary_fungible_store::deposit(account_addr, coin_to_fungible_asset(coin));
        &#125; else &#123;
            abort error::not_found(ECOIN_STORE_NOT_PUBLISHED)
        &#125;;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_migrated_primary_fungible_store_exists"></a>

## Function `migrated_primary_fungible_store_exists`



<pre><code>fun migrated_primary_fungible_store_exists(account_address: address, metadata: object::Object&lt;fungible_asset::Metadata&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun migrated_primary_fungible_store_exists(
    account_address: address,
    metadata: Object&lt;Metadata&gt;
): bool &#123;
    let primary_store_address &#61; primary_fungible_store::primary_store_address&lt;Metadata&gt;(account_address, metadata);
    fungible_asset::store_exists(primary_store_address) &amp;&amp; exists&lt;MigrationFlag&gt;(primary_store_address)
&#125;
</code></pre>



</details>

<a id="0x1_coin_force_deposit"></a>

## Function `force_deposit`

Deposit the coin balance into the recipient's account without checking if the account is frozen.
This is for internal use only and doesn't emit an DepositEvent.


<pre><code>public(friend) fun force_deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun force_deposit&lt;CoinType&gt;(
    account_addr: address,
    coin: Coin&lt;CoinType&gt;
) acquires CoinStore, CoinConversionMap, CoinInfo &#123;
    if (exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)) &#123;
        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
        merge(&amp;mut coin_store.coin, coin);
    &#125; else &#123;
        let metadata &#61; paired_metadata&lt;CoinType&gt;();
        if (option::is_some(&amp;metadata) &amp;&amp; migrated_primary_fungible_store_exists(
            account_addr,
            option::destroy_some(metadata)
        )) &#123;
            let fa &#61; coin_to_fungible_asset(coin);
            let metadata &#61; fungible_asset::asset_metadata(&amp;fa);
            let store &#61; primary_fungible_store::primary_store(account_addr, metadata);
            fungible_asset::deposit_internal(store, fa);
        &#125; else &#123;
            abort error::not_found(ECOIN_STORE_NOT_PUBLISHED)
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_destroy_zero"></a>

## Function `destroy_zero`

Destroys a zero-value coin. Calls will fail if the <code>value</code> in the passed-in <code>token</code> is non-zero
so it is impossible to "burn" any non-zero amount of <code>Coin</code> without having
a <code>BurnCapability</code> for the specific <code>CoinType</code>.


<pre><code>public fun destroy_zero&lt;CoinType&gt;(zero_coin: coin::Coin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_zero&lt;CoinType&gt;(zero_coin: Coin&lt;CoinType&gt;) &#123;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; zero_coin.value;
    &#125;;
    let Coin &#123; value &#125; &#61; zero_coin;
    assert!(value &#61;&#61; 0, error::invalid_argument(EDESTRUCTION_OF_NONZERO_TOKEN))
&#125;
</code></pre>



</details>

<a id="0x1_coin_extract"></a>

## Function `extract`

Extracts <code>amount</code> from the passed-in <code>coin</code>, where the original token is modified in place.


<pre><code>public fun extract&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;, amount: u64): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract&lt;CoinType&gt;(coin: &amp;mut Coin&lt;CoinType&gt;, amount: u64): Coin&lt;CoinType&gt; &#123;
    assert!(coin.value &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; amount;
    &#125;;
    coin.value &#61; coin.value &#45; amount;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; amount;
    &#125;;
    Coin &#123; value: amount &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_extract_all"></a>

## Function `extract_all`

Extracts the entire amount from the passed-in <code>coin</code>, where the original token is modified in place.


<pre><code>public fun extract_all&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract_all&lt;CoinType&gt;(coin: &amp;mut Coin&lt;CoinType&gt;): Coin&lt;CoinType&gt; &#123;
    let total_value &#61; coin.value;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; coin.value;
    &#125;;
    coin.value &#61; 0;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; total_value;
    &#125;;
    Coin &#123; value: total_value &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_freeze_coin_store"></a>

## Function `freeze_coin_store`

Freeze a CoinStore to prevent transfers


<pre><code>&#35;[legacy_entry_fun]
public entry fun freeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun freeze_coin_store&lt;CoinType&gt;(
    account_addr: address,
    _freeze_cap: &amp;FreezeCapability&lt;CoinType&gt;,
) acquires CoinStore &#123;
    let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    coin_store.frozen &#61; true;
&#125;
</code></pre>



</details>

<a id="0x1_coin_unfreeze_coin_store"></a>

## Function `unfreeze_coin_store`

Unfreeze a CoinStore to allow transfers


<pre><code>&#35;[legacy_entry_fun]
public entry fun unfreeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unfreeze_coin_store&lt;CoinType&gt;(
    account_addr: address,
    _freeze_cap: &amp;FreezeCapability&lt;CoinType&gt;,
) acquires CoinStore &#123;
    let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    coin_store.frozen &#61; false;
&#125;
</code></pre>



</details>

<a id="0x1_coin_upgrade_supply"></a>

## Function `upgrade_supply`

Upgrade total supply to use a parallelizable implementation if it is
available.


<pre><code>public entry fun upgrade_supply&lt;CoinType&gt;(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun upgrade_supply&lt;CoinType&gt;(account: &amp;signer) acquires CoinInfo, SupplyConfig &#123;
    let account_addr &#61; signer::address_of(account);

    // Only coin creators can upgrade total supply.
    assert!(
        coin_address&lt;CoinType&gt;() &#61;&#61; account_addr,
        error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),
    );

    // Can only succeed once on&#45;chain governance agreed on the upgrade.
    assert!(
        borrow_global_mut&lt;SupplyConfig&gt;(@aptos_framework).allow_upgrades,
        error::permission_denied(ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED)
    );

    let maybe_supply &#61; &amp;mut borrow_global_mut&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr).supply;
    if (option::is_some(maybe_supply)) &#123;
        let supply &#61; option::borrow_mut(maybe_supply);

        // If supply is tracked and the current implementation uses an integer &#45; upgrade.
        if (!optional_aggregator::is_parallelizable(supply)) &#123;
            optional_aggregator::switch(supply);
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_initialize"></a>

## Function `initialize`

Creates a new Coin with given <code>CoinType</code> and returns minting/freezing/burning capabilities.
The given signer also becomes the account hosting the information  about the coin
(name, supply, etc.). Supply is initialized as non-parallelizable integer.


<pre><code>public fun initialize&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize&lt;CoinType&gt;(
    account: &amp;signer,
    name: string::String,
    symbol: string::String,
    decimals: u8,
    monitor_supply: bool,
): (BurnCapability&lt;CoinType&gt;, FreezeCapability&lt;CoinType&gt;, MintCapability&lt;CoinType&gt;) &#123;
    initialize_internal(account, name, symbol, decimals, monitor_supply, false)
&#125;
</code></pre>



</details>

<a id="0x1_coin_initialize_with_parallelizable_supply"></a>

## Function `initialize_with_parallelizable_supply`

Same as <code>initialize</code> but supply can be initialized to parallelizable aggregator.


<pre><code>public(friend) fun initialize_with_parallelizable_supply&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_with_parallelizable_supply&lt;CoinType&gt;(
    account: &amp;signer,
    name: string::String,
    symbol: string::String,
    decimals: u8,
    monitor_supply: bool,
): (BurnCapability&lt;CoinType&gt;, FreezeCapability&lt;CoinType&gt;, MintCapability&lt;CoinType&gt;) &#123;
    system_addresses::assert_aptos_framework(account);
    initialize_internal(account, name, symbol, decimals, monitor_supply, true)
&#125;
</code></pre>



</details>

<a id="0x1_coin_initialize_internal"></a>

## Function `initialize_internal`



<pre><code>fun initialize_internal&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool, parallelizable: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_internal&lt;CoinType&gt;(
    account: &amp;signer,
    name: string::String,
    symbol: string::String,
    decimals: u8,
    monitor_supply: bool,
    parallelizable: bool,
): (BurnCapability&lt;CoinType&gt;, FreezeCapability&lt;CoinType&gt;, MintCapability&lt;CoinType&gt;) &#123;
    let account_addr &#61; signer::address_of(account);

    assert!(
        coin_address&lt;CoinType&gt;() &#61;&#61; account_addr,
        error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),
    );

    assert!(
        !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr),
        error::already_exists(ECOIN_INFO_ALREADY_PUBLISHED),
    );

    assert!(string::length(&amp;name) &lt;&#61; MAX_COIN_NAME_LENGTH, error::invalid_argument(ECOIN_NAME_TOO_LONG));
    assert!(string::length(&amp;symbol) &lt;&#61; MAX_COIN_SYMBOL_LENGTH, error::invalid_argument(ECOIN_SYMBOL_TOO_LONG));

    let coin_info &#61; CoinInfo&lt;CoinType&gt; &#123;
        name,
        symbol,
        decimals,
        supply: if (monitor_supply) &#123;
            option::some(
                optional_aggregator::new(MAX_U128, parallelizable)
            )
        &#125; else &#123; option::none() &#125;,
    &#125;;
    move_to(account, coin_info);

    (BurnCapability&lt;CoinType&gt; &#123;&#125;, FreezeCapability&lt;CoinType&gt; &#123;&#125;, MintCapability&lt;CoinType&gt; &#123;&#125;)
&#125;
</code></pre>



</details>

<a id="0x1_coin_merge"></a>

## Function `merge`

"Merges" the two given coins.  The coin passed in as <code>dst_coin</code> will have a value equal
to the sum of the two tokens (<code>dst_coin</code> and <code>source_coin</code>).


<pre><code>public fun merge&lt;CoinType&gt;(dst_coin: &amp;mut coin::Coin&lt;CoinType&gt;, source_coin: coin::Coin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun merge&lt;CoinType&gt;(dst_coin: &amp;mut Coin&lt;CoinType&gt;, source_coin: Coin&lt;CoinType&gt;) &#123;
    spec &#123;
        assume dst_coin.value &#43; source_coin.value &lt;&#61; MAX_U64;
    &#125;;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; source_coin.value;
    &#125;;
    let Coin &#123; value &#125; &#61; source_coin;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; value;
    &#125;;
    dst_coin.value &#61; dst_coin.value &#43; value;
&#125;
</code></pre>



</details>

<a id="0x1_coin_mint"></a>

## Function `mint`

Mint new <code>Coin</code> with capability.
The capability <code>_cap</code> should be passed as reference to <code>MintCapability&lt;CoinType&gt;</code>.
Returns minted <code>Coin</code>.


<pre><code>public fun mint&lt;CoinType&gt;(amount: u64, _cap: &amp;coin::MintCapability&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint&lt;CoinType&gt;(
    amount: u64,
    _cap: &amp;MintCapability&lt;CoinType&gt;,
): Coin&lt;CoinType&gt; acquires CoinInfo &#123;
    mint_internal&lt;CoinType&gt;(amount)
&#125;
</code></pre>



</details>

<a id="0x1_coin_register"></a>

## Function `register`



<pre><code>public fun register&lt;CoinType&gt;(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun register&lt;CoinType&gt;(account: &amp;signer) acquires CoinConversionMap &#123;
    let account_addr &#61; signer::address_of(account);
    // Short&#45;circuit and do nothing if account is already registered for CoinType.
    if (is_account_registered&lt;CoinType&gt;(account_addr)) &#123;
        return
    &#125;;

    account::register_coin&lt;CoinType&gt;(account_addr);
    let coin_store &#61; CoinStore&lt;CoinType&gt; &#123;
        coin: Coin &#123; value: 0 &#125;,
        frozen: false,
        deposit_events: account::new_event_handle&lt;DepositEvent&gt;(account),
        withdraw_events: account::new_event_handle&lt;WithdrawEvent&gt;(account),
    &#125;;
    move_to(account, coin_store);
&#125;
</code></pre>



</details>

<a id="0x1_coin_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of coins <code>CoinType</code> from <code>from</code> to <code>to</code>.


<pre><code>public entry fun transfer&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;CoinType&gt;(
    from: &amp;signer,
    to: address,
    amount: u64,
) acquires CoinStore, CoinConversionMap, CoinInfo, PairedCoinType &#123;
    let coin &#61; withdraw&lt;CoinType&gt;(from, amount);
    deposit(to, coin);
&#125;
</code></pre>



</details>

<a id="0x1_coin_value"></a>

## Function `value`

Returns the <code>value</code> passed in <code>coin</code>.


<pre><code>public fun value&lt;CoinType&gt;(coin: &amp;coin::Coin&lt;CoinType&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun value&lt;CoinType&gt;(coin: &amp;Coin&lt;CoinType&gt;): u64 &#123;
    coin.value
&#125;
</code></pre>



</details>

<a id="0x1_coin_withdraw"></a>

## Function `withdraw`

Withdraw specified <code>amount</code> of coin <code>CoinType</code> from the signing account.


<pre><code>public fun withdraw&lt;CoinType&gt;(account: &amp;signer, amount: u64): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;CoinType&gt;(
    account: &amp;signer,
    amount: u64,
): Coin&lt;CoinType&gt; acquires CoinStore, CoinConversionMap, CoinInfo, PairedCoinType &#123;
    let account_addr &#61; signer::address_of(account);

    let (coin_amount_to_withdraw, fa_amount_to_withdraw) &#61; calculate_amount_to_withdraw&lt;CoinType&gt;(
        account_addr,
        amount
    );
    let withdrawn_coin &#61; if (coin_amount_to_withdraw &gt; 0) &#123;
        let coin_store &#61; borrow_global_mut&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
        assert!(
            !coin_store.frozen,
            error::permission_denied(EFROZEN),
        );
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(Withdraw&lt;CoinType&gt; &#123; account: account_addr, amount: coin_amount_to_withdraw &#125;);
        &#125;;
        event::emit_event&lt;WithdrawEvent&gt;(
            &amp;mut coin_store.withdraw_events,
            WithdrawEvent &#123; amount: coin_amount_to_withdraw &#125;,
        );
        extract(&amp;mut coin_store.coin, coin_amount_to_withdraw)
    &#125; else &#123;
        zero()
    &#125;;
    if (fa_amount_to_withdraw &gt; 0) &#123;
        let fa &#61; primary_fungible_store::withdraw(
            account,
            option::destroy_some(paired_metadata&lt;CoinType&gt;()),
            fa_amount_to_withdraw
        );
        merge(&amp;mut withdrawn_coin, fungible_asset_to_coin(fa));
    &#125;;
    withdrawn_coin
&#125;
</code></pre>



</details>

<a id="0x1_coin_zero"></a>

## Function `zero`

Create a new <code>Coin&lt;CoinType&gt;</code> with a value of <code>0</code>.


<pre><code>public fun zero&lt;CoinType&gt;(): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun zero&lt;CoinType&gt;(): Coin&lt;CoinType&gt; &#123;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; 0;
    &#125;;
    Coin&lt;CoinType&gt; &#123;
        value: 0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_destroy_freeze_cap"></a>

## Function `destroy_freeze_cap`

Destroy a freeze capability. Freeze capability is dangerous and therefore should be destroyed if not used.


<pre><code>public fun destroy_freeze_cap&lt;CoinType&gt;(freeze_cap: coin::FreezeCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_freeze_cap&lt;CoinType&gt;(freeze_cap: FreezeCapability&lt;CoinType&gt;) &#123;
    let FreezeCapability&lt;CoinType&gt; &#123;&#125; &#61; freeze_cap;
&#125;
</code></pre>



</details>

<a id="0x1_coin_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Destroy a mint capability.


<pre><code>public fun destroy_mint_cap&lt;CoinType&gt;(mint_cap: coin::MintCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_mint_cap&lt;CoinType&gt;(mint_cap: MintCapability&lt;CoinType&gt;) &#123;
    let MintCapability&lt;CoinType&gt; &#123;&#125; &#61; mint_cap;
&#125;
</code></pre>



</details>

<a id="0x1_coin_destroy_burn_cap"></a>

## Function `destroy_burn_cap`

Destroy a burn capability.


<pre><code>public fun destroy_burn_cap&lt;CoinType&gt;(burn_cap: coin::BurnCapability&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_burn_cap&lt;CoinType&gt;(burn_cap: BurnCapability&lt;CoinType&gt;) &#123;
    let BurnCapability&lt;CoinType&gt; &#123;&#125; &#61; burn_cap;
&#125;
</code></pre>



</details>

<a id="0x1_coin_mint_internal"></a>

## Function `mint_internal`



<pre><code>fun mint_internal&lt;CoinType&gt;(amount: u64): coin::Coin&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun mint_internal&lt;CoinType&gt;(amount: u64): Coin&lt;CoinType&gt; acquires CoinInfo &#123;
    if (amount &#61;&#61; 0) &#123;
        return Coin&lt;CoinType&gt; &#123;
            value: 0
        &#125;
    &#125;;

    let maybe_supply &#61; &amp;mut borrow_global_mut&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).supply;
    if (option::is_some(maybe_supply)) &#123;
        let supply &#61; option::borrow_mut(maybe_supply);
        spec &#123;
            use aptos_framework::optional_aggregator;
            use aptos_framework::aggregator;
            assume optional_aggregator::is_parallelizable(supply) &#61;&#61;&gt; (aggregator::spec_aggregator_get_val(
                option::borrow(supply.aggregator)
            )
                &#43; amount &lt;&#61; aggregator::spec_get_limit(option::borrow(supply.aggregator)));
            assume !optional_aggregator::is_parallelizable(supply) &#61;&#61;&gt;
                (option::borrow(supply.integer).value &#43; amount &lt;&#61; option::borrow(supply.integer).limit);
        &#125;;
        optional_aggregator::add(supply, (amount as u128));
    &#125;;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#43; amount;
    &#125;;
    Coin&lt;CoinType&gt; &#123; value: amount &#125;
&#125;
</code></pre>



</details>

<a id="0x1_coin_burn_internal"></a>

## Function `burn_internal`



<pre><code>fun burn_internal&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun burn_internal&lt;CoinType&gt;(coin: Coin&lt;CoinType&gt;): u64 acquires CoinInfo &#123;
    spec &#123;
        update supply&lt;CoinType&gt; &#61; supply&lt;CoinType&gt; &#45; coin.value;
    &#125;;
    let Coin &#123; value: amount &#125; &#61; coin;
    if (amount !&#61; 0) &#123;
        let maybe_supply &#61; &amp;mut borrow_global_mut&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_address&lt;CoinType&gt;()).supply;
        if (option::is_some(maybe_supply)) &#123;
            let supply &#61; option::borrow_mut(maybe_supply);
            optional_aggregator::sub(supply, (amount as u128));
        &#125;;
    &#125;;
    amount
&#125;
</code></pre>



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


<pre><code>pragma verify &#61; true;
<a id="0x1_coin_supply"></a>
global supply&lt;CoinType&gt;: num;
<a id="0x1_coin_aggregate_supply"></a>
global aggregate_supply&lt;CoinType&gt;: num;
apply TotalSupplyTracked&lt;CoinType&gt; to &#42;&lt;CoinType&gt; except
initialize, initialize_internal, initialize_with_parallelizable_supply;
</code></pre>




<a id="0x1_coin_spec_fun_supply_tracked"></a>


<pre><code>fun spec_fun_supply_tracked&lt;CoinType&gt;(val: u64, supply: Option&lt;OptionalAggregator&gt;): bool &#123;
   option::spec_is_some(supply) &#61;&#61;&gt; val &#61;&#61; optional_aggregator::optional_aggregator_value
       (option::spec_borrow(supply))
&#125;
</code></pre>




<a id="0x1_coin_TotalSupplyTracked"></a>


<pre><code>schema TotalSupplyTracked&lt;CoinType&gt; &#123;
    ensures old(spec_fun_supply_tracked&lt;CoinType&gt;(supply&lt;CoinType&gt; &#43; aggregate_supply&lt;CoinType&gt;,
        global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply)) &#61;&#61;&gt;
        spec_fun_supply_tracked&lt;CoinType&gt;(supply&lt;CoinType&gt; &#43; aggregate_supply&lt;CoinType&gt;,
            global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply);
&#125;
</code></pre>




<a id="0x1_coin_spec_fun_supply_no_change"></a>


<pre><code>fun spec_fun_supply_no_change&lt;CoinType&gt;(old_supply: Option&lt;OptionalAggregator&gt;,
                                            supply: Option&lt;OptionalAggregator&gt;): bool &#123;
   option::spec_is_some(old_supply) &#61;&#61;&gt; optional_aggregator::optional_aggregator_value
       (option::spec_borrow(old_supply)) &#61;&#61; optional_aggregator::optional_aggregator_value
       (option::spec_borrow(supply))
&#125;
</code></pre>




<a id="0x1_coin_TotalSupplyNoChange"></a>


<pre><code>schema TotalSupplyNoChange&lt;CoinType&gt; &#123;
    let old_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply;
    let post supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply;
    ensures spec_fun_supply_no_change&lt;CoinType&gt;(old_supply, supply);
&#125;
</code></pre>



<a id="@Specification_1_AggregatableCoin"></a>

### Struct `AggregatableCoin`


<pre><code>struct AggregatableCoin&lt;CoinType&gt; has store
</code></pre>



<dl>
<dt>
<code>value: aggregator::Aggregator</code>
</dt>
<dd>
 Amount of aggregatable coin this address has.
</dd>
</dl>



<pre><code>invariant aggregator::spec_get_limit(value) &#61;&#61; MAX_U64;
</code></pre>



<a id="@Specification_1_coin_to_fungible_asset"></a>

### Function `coin_to_fungible_asset`


<pre><code>public fun coin_to_fungible_asset&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): fungible_asset::FungibleAsset
</code></pre>




<pre><code>pragma verify &#61; false;
let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
</code></pre>



<a id="@Specification_1_fungible_asset_to_coin"></a>

### Function `fungible_asset_to_coin`


<pre><code>fun fungible_asset_to_coin&lt;CoinType&gt;(fungible_asset: fungible_asset::FungibleAsset): coin::Coin&lt;CoinType&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_initialize_supply_config"></a>

### Function `initialize_supply_config`


<pre><code>public(friend) fun initialize_supply_config(aptos_framework: &amp;signer)
</code></pre>


Can only be initialized once.
Can only be published by reserved addresses.


<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
aborts_if exists&lt;SupplyConfig&gt;(aptos_addr);
ensures !global&lt;SupplyConfig&gt;(aptos_addr).allow_upgrades;
ensures exists&lt;SupplyConfig&gt;(aptos_addr);
</code></pre>



<a id="@Specification_1_allow_supply_upgrades"></a>

### Function `allow_supply_upgrades`


<pre><code>public fun allow_supply_upgrades(aptos_framework: &amp;signer, allowed: bool)
</code></pre>


Can only be updated by <code>@aptos_framework</code>.


<pre><code>modifies global&lt;SupplyConfig&gt;(@aptos_framework);
let aptos_addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
aborts_if !exists&lt;SupplyConfig&gt;(aptos_addr);
let post allow_upgrades_post &#61; global&lt;SupplyConfig&gt;(@aptos_framework);
ensures allow_upgrades_post.allow_upgrades &#61;&#61; allowed;
</code></pre>



<a id="@Specification_1_initialize_aggregatable_coin"></a>

### Function `initialize_aggregatable_coin`


<pre><code>public(friend) fun initialize_aggregatable_coin&lt;CoinType&gt;(aptos_framework: &amp;signer): coin::AggregatableCoin&lt;CoinType&gt;
</code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework &#123; account: aptos_framework &#125;;
include aggregator_factory::CreateAggregatorInternalAbortsIf;
</code></pre>



<a id="@Specification_1_is_aggregatable_coin_zero"></a>

### Function `is_aggregatable_coin_zero`


<pre><code>public(friend) fun is_aggregatable_coin_zero&lt;CoinType&gt;(coin: &amp;coin::AggregatableCoin&lt;CoinType&gt;): bool
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; (aggregator::spec_read(coin.value) &#61;&#61; 0);
</code></pre>



<a id="@Specification_1_drain_aggregatable_coin"></a>

### Function `drain_aggregatable_coin`


<pre><code>public(friend) fun drain_aggregatable_coin&lt;CoinType&gt;(coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;
</code></pre>




<pre><code>aborts_if aggregator::spec_read(coin.value) &gt; MAX_U64;
ensures result.value &#61;&#61; aggregator::spec_aggregator_get_val(old(coin).value);
</code></pre>



<a id="@Specification_1_merge_aggregatable_coin"></a>

### Function `merge_aggregatable_coin`


<pre><code>public(friend) fun merge_aggregatable_coin&lt;CoinType&gt;(dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;, coin: coin::Coin&lt;CoinType&gt;)
</code></pre>




<pre><code>let aggr &#61; dst_coin.value;
let post p_aggr &#61; dst_coin.value;
aborts_if aggregator::spec_aggregator_get_val(aggr)
    &#43; coin.value &gt; aggregator::spec_get_limit(aggr);
aborts_if aggregator::spec_aggregator_get_val(aggr)
    &#43; coin.value &gt; MAX_U128;
ensures aggregator::spec_aggregator_get_val(aggr) &#43; coin.value &#61;&#61; aggregator::spec_aggregator_get_val(p_aggr);
</code></pre>



<a id="@Specification_1_collect_into_aggregatable_coin"></a>

### Function `collect_into_aggregatable_coin`


<pre><code>public(friend) fun collect_into_aggregatable_coin&lt;CoinType&gt;(account_addr: address, amount: u64, dst_coin: &amp;mut coin::AggregatableCoin&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
let aggr &#61; dst_coin.value;
let post p_aggr &#61; dst_coin.value;
let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
let post p_coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
aborts_if amount &gt; 0 &amp;&amp; !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
aborts_if amount &gt; 0 &amp;&amp; coin_store.coin.value &lt; amount;
aborts_if amount &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)
    &#43; amount &gt; aggregator::spec_get_limit(aggr);
aborts_if amount &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)
    &#43; amount &gt; MAX_U128;
ensures aggregator::spec_aggregator_get_val(aggr) &#43; amount &#61;&#61; aggregator::spec_aggregator_get_val(p_aggr);
ensures coin_store.coin.value &#45; amount &#61;&#61; p_coin_store.coin.value;
</code></pre>



<a id="@Specification_1_maybe_convert_to_fungible_store"></a>

### Function `maybe_convert_to_fungible_store`


<pre><code>fun maybe_convert_to_fungible_store&lt;CoinType&gt;(account: address)
</code></pre>




<pre><code>pragma verify &#61; false;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account);
modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account);
</code></pre>




<a id="0x1_coin_DepositAbortsIf"></a>


<pre><code>schema DepositAbortsIf&lt;CoinType&gt; &#123;
    account_addr: address;
    let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    aborts_if coin_store.frozen;
&#125;
</code></pre>



<a id="@Specification_1_coin_address"></a>

### Function `coin_address`


<pre><code>fun coin_address&lt;CoinType&gt;(): address
</code></pre>


Get address by reflection.


<pre><code>pragma opaque;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; type_info::type_of&lt;CoinType&gt;().account_address;
</code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code>&#35;[view]
public fun balance&lt;CoinType&gt;(owner: address): u64
</code></pre>




<pre><code>pragma verify &#61; false;
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(owner);
ensures result &#61;&#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(owner).coin.value;
</code></pre>



<a id="@Specification_1_is_coin_initialized"></a>

### Function `is_coin_initialized`


<pre><code>&#35;[view]
public fun is_coin_initialized&lt;CoinType&gt;(): bool
</code></pre>




<pre><code>// This enforces <a id="high-level-req-7.1" href="#high-level-req">high-level requirement 7</a>:
aborts_if false;
</code></pre>



<a id="@Specification_1_is_account_registered"></a>

### Function `is_account_registered`


<pre><code>&#35;[view]
public fun is_account_registered&lt;CoinType&gt;(account_addr: address): bool
</code></pre>




<pre><code>pragma aborts_if_is_partial;
aborts_if false;
</code></pre>




<a id="0x1_coin_get_coin_supply_opt"></a>


<pre><code>fun get_coin_supply_opt&lt;CoinType&gt;(): Option&lt;OptionalAggregator&gt; &#123;
   global&lt;CoinInfo&lt;CoinType&gt;&gt;(type_info::type_of&lt;CoinType&gt;().account_address).supply
&#125;
</code></pre>




<a id="0x1_coin_spec_paired_metadata"></a>


<pre><code>fun spec_paired_metadata&lt;CoinType&gt;(): Option&lt;Object&lt;Metadata&gt;&gt; &#123;
   if (exists&lt;CoinConversionMap&gt;(@aptos_framework)) &#123;
       let map &#61; global&lt;CoinConversionMap&gt;(@aptos_framework).coin_to_fungible_asset_map;
       if (table::spec_contains(map, type_info::type_of&lt;CoinType&gt;())) &#123;
           let metadata &#61; table::spec_get(map, type_info::type_of&lt;CoinType&gt;());
           option::spec_some(metadata)
       &#125; else &#123;
           option::spec_none()
       &#125;
   &#125; else &#123;
       option::spec_none()
   &#125;
&#125;
</code></pre>




<a id="0x1_coin_spec_is_account_registered"></a>


<pre><code>fun spec_is_account_registered&lt;CoinType&gt;(account_addr: address): bool &#123;
   let paired_metadata_opt &#61; spec_paired_metadata&lt;CoinType&gt;();
   exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr) &#124;&#124; (option::spec_is_some(
       paired_metadata_opt
   ) &amp;&amp; primary_fungible_store::spec_primary_store_exists(account_addr, option::spec_borrow(paired_metadata_opt)))
&#125;
</code></pre>




<a id="0x1_coin_CoinSubAbortsIf"></a>


<pre><code>schema CoinSubAbortsIf&lt;CoinType&gt; &#123;
    amount: u64;
    let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
    let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;
    include (option::is_some(
        maybe_supply
    )) &#61;&#61;&gt; optional_aggregator::SubAbortsIf &#123; optional_aggregator: option::borrow(maybe_supply), value: amount &#125;;
&#125;
</code></pre>




<a id="0x1_coin_CoinAddAbortsIf"></a>


<pre><code>schema CoinAddAbortsIf&lt;CoinType&gt; &#123;
    amount: u64;
    let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
    let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;
    include (option::is_some(
        maybe_supply
    )) &#61;&#61;&gt; optional_aggregator::AddAbortsIf &#123; optional_aggregator: option::borrow(maybe_supply), value: amount &#125;;
&#125;
</code></pre>




<a id="0x1_coin_AbortsIfNotExistCoinInfo"></a>


<pre><code>schema AbortsIfNotExistCoinInfo&lt;CoinType&gt; &#123;
    let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
    aborts_if !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
&#125;
</code></pre>



<a id="@Specification_1_name"></a>

### Function `name`


<pre><code>&#35;[view]
public fun name&lt;CoinType&gt;(): string::String
</code></pre>




<pre><code>// This enforces <a id="high-level-req-7.3" href="#high-level-req">high-level requirement 7</a>:
include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;
</code></pre>



<a id="@Specification_1_symbol"></a>

### Function `symbol`


<pre><code>&#35;[view]
public fun symbol&lt;CoinType&gt;(): string::String
</code></pre>




<pre><code>// This enforces <a id="high-level-req-7.4" href="#high-level-req">high-level requirement 7</a>:
include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;
</code></pre>



<a id="@Specification_1_decimals"></a>

### Function `decimals`


<pre><code>&#35;[view]
public fun decimals&lt;CoinType&gt;(): u8
</code></pre>




<pre><code>include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;
</code></pre>



<a id="@Specification_1_supply"></a>

### Function `supply`


<pre><code>&#35;[view]
public fun supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_coin_supply"></a>

### Function `coin_supply`


<pre><code>&#35;[view]
public fun coin_supply&lt;CoinType&gt;(): option::Option&lt;u128&gt;
</code></pre>




<pre><code>let coin_addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
// This enforces <a id="high-level-req-7.5" href="#high-level-req">high-level requirement 7</a>:
aborts_if !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_addr);
let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(coin_addr).supply;
let supply &#61; option::spec_borrow(maybe_supply);
let value &#61; optional_aggregator::optional_aggregator_value(supply);
ensures if (option::spec_is_some(maybe_supply)) &#123;
    result &#61;&#61; option::spec_some(value)
&#125; else &#123;
    option::spec_is_none(result)
&#125;;
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public fun burn&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;, _cap: &amp;coin::BurnCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
include AbortsIfNotExistCoinInfo&lt;CoinType&gt;;
aborts_if coin.value &#61;&#61; 0;
include CoinSubAbortsIf&lt;CoinType&gt; &#123; amount: coin.value &#125;;
ensures supply&lt;CoinType&gt; &#61;&#61; old(supply&lt;CoinType&gt;) &#45; coin.value;
</code></pre>



<a id="@Specification_1_burn_from"></a>

### Function `burn_from`


<pre><code>public fun burn_from&lt;CoinType&gt;(account_addr: address, amount: u64, burn_cap: &amp;coin::BurnCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
let post post_coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
aborts_if amount !&#61; 0 &amp;&amp; !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
aborts_if amount !&#61; 0 &amp;&amp; !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
aborts_if coin_store.coin.value &lt; amount;
let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;
let supply_aggr &#61; option::spec_borrow(maybe_supply);
let value &#61; optional_aggregator::optional_aggregator_value(supply_aggr);
let post post_maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr).supply;
let post post_supply &#61; option::spec_borrow(post_maybe_supply);
let post post_value &#61; optional_aggregator::optional_aggregator_value(post_supply);
aborts_if option::spec_is_some(maybe_supply) &amp;&amp; value &lt; amount;
ensures post_coin_store.coin.value &#61;&#61; coin_store.coin.value &#45; amount;
// This enforces <a id="high-level-req-5" href="managed_coin.md#high-level-req">high-level requirement 5</a> of the <a href=managed_coin.md>managed_coin</a> module:
ensures if (option::spec_is_some(maybe_supply)) &#123;
    post_value &#61;&#61; value &#45; amount
&#125; else &#123;
    option::spec_is_none(post_maybe_supply)
&#125;;
ensures supply&lt;CoinType&gt; &#61;&#61; old(supply&lt;CoinType&gt;) &#45; amount;
</code></pre>



<a id="@Specification_1_deposit"></a>

### Function `deposit`


<pre><code>public fun deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)
</code></pre>


<code>account_addr</code> is not frozen.


<pre><code>pragma verify &#61; false;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
// This enforces <a id="high-level-req-8.3" href="#high-level-req">high-level requirement 8</a>:
include DepositAbortsIf&lt;CoinType&gt;;
ensures global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr).coin.value &#61;&#61; old(
    global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)
).coin.value &#43; coin.value;
</code></pre>



<a id="@Specification_1_force_deposit"></a>

### Function `force_deposit`


<pre><code>public(friend) fun force_deposit&lt;CoinType&gt;(account_addr: address, coin: coin::Coin&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
ensures global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr).coin.value &#61;&#61; old(
    global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr)
).coin.value &#43; coin.value;
</code></pre>



<a id="@Specification_1_destroy_zero"></a>

### Function `destroy_zero`


<pre><code>public fun destroy_zero&lt;CoinType&gt;(zero_coin: coin::Coin&lt;CoinType&gt;)
</code></pre>


The value of <code>zero_coin</code> must be 0.


<pre><code>aborts_if zero_coin.value &gt; 0;
</code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code>public fun extract&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;, amount: u64): coin::Coin&lt;CoinType&gt;
</code></pre>




<pre><code>aborts_if coin.value &lt; amount;
ensures result.value &#61;&#61; amount;
ensures coin.value &#61;&#61; old(coin.value) &#45; amount;
</code></pre>



<a id="@Specification_1_extract_all"></a>

### Function `extract_all`


<pre><code>public fun extract_all&lt;CoinType&gt;(coin: &amp;mut coin::Coin&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;
</code></pre>




<pre><code>ensures result.value &#61;&#61; old(coin).value;
ensures coin.value &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_freeze_coin_store"></a>

### Function `freeze_coin_store`


<pre><code>&#35;[legacy_entry_fun]
public entry fun freeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
// This enforces <a id="high-level-req-6.3" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
let post coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
ensures coin_store.frozen;
</code></pre>



<a id="@Specification_1_unfreeze_coin_store"></a>

### Function `unfreeze_coin_store`


<pre><code>&#35;[legacy_entry_fun]
public entry fun unfreeze_coin_store&lt;CoinType&gt;(account_addr: address, _freeze_cap: &amp;coin::FreezeCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
// This enforces <a id="high-level-req-6.4" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
let post coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
ensures !coin_store.frozen;
</code></pre>



<a id="@Specification_1_upgrade_supply"></a>

### Function `upgrade_supply`


<pre><code>public entry fun upgrade_supply&lt;CoinType&gt;(account: &amp;signer)
</code></pre>


The creator of <code>CoinType</code> must be <code>@aptos_framework</code>.
<code>SupplyConfig</code> allow upgrade.


<pre><code>let account_addr &#61; signer::address_of(account);
let coin_address &#61; type_info::type_of&lt;CoinType&gt;().account_address;
aborts_if coin_address !&#61; account_addr;
aborts_if !exists&lt;SupplyConfig&gt;(@aptos_framework);
// This enforces <a id="high-level-req-1.1" href="#high-level-req">high-level requirement 1</a>:
aborts_if !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
let supply_config &#61; global&lt;SupplyConfig&gt;(@aptos_framework);
aborts_if !supply_config.allow_upgrades;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
let maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr).supply;
let supply &#61; option::spec_borrow(maybe_supply);
let value &#61; optional_aggregator::optional_aggregator_value(supply);
let post post_maybe_supply &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr).supply;
let post post_supply &#61; option::spec_borrow(post_maybe_supply);
let post post_value &#61; optional_aggregator::optional_aggregator_value(post_supply);
let supply_no_parallel &#61; option::spec_is_some(maybe_supply) &amp;&amp;
    !optional_aggregator::is_parallelizable(supply);
aborts_if supply_no_parallel &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
ensures supply_no_parallel &#61;&#61;&gt;
    optional_aggregator::is_parallelizable(post_supply) &amp;&amp; post_value &#61;&#61; value;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public fun initialize&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>let account_addr &#61; signer::address_of(account);
// This enforces <a id="high-level-req-1.2" href="#high-level-req">high-level requirement 1</a>:
aborts_if type_info::type_of&lt;CoinType&gt;().account_address !&#61; account_addr;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
aborts_if string::length(name) &gt; MAX_COIN_NAME_LENGTH;
aborts_if string::length(symbol) &gt; MAX_COIN_SYMBOL_LENGTH;
</code></pre>



<a id="@Specification_1_initialize_with_parallelizable_supply"></a>

### Function `initialize_with_parallelizable_supply`


<pre><code>public(friend) fun initialize_with_parallelizable_supply&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>let addr &#61; signer::address_of(account);
aborts_if addr !&#61; @aptos_framework;
aborts_if monitor_supply &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
include InitializeInternalSchema&lt;CoinType&gt; &#123;
    name: name.bytes,
    symbol: symbol.bytes
&#125;;
ensures exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
</code></pre>


Make sure <code>name</code> and <code>symbol</code> are legal length.
Only the creator of <code>CoinType</code> can initialize.


<a id="0x1_coin_InitializeInternalSchema"></a>


<pre><code>schema InitializeInternalSchema&lt;CoinType&gt; &#123;
    account: signer;
    name: vector&lt;u8&gt;;
    symbol: vector&lt;u8&gt;;
    let account_addr &#61; signer::address_of(account);
    let coin_address &#61; type_info::type_of&lt;CoinType&gt;().account_address;
    aborts_if coin_address !&#61; account_addr;
    aborts_if exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
    aborts_if len(name) &gt; MAX_COIN_NAME_LENGTH;
    aborts_if len(symbol) &gt; MAX_COIN_SYMBOL_LENGTH;
&#125;
</code></pre>



<a id="@Specification_1_initialize_internal"></a>

### Function `initialize_internal`


<pre><code>fun initialize_internal&lt;CoinType&gt;(account: &amp;signer, name: string::String, symbol: string::String, decimals: u8, monitor_supply: bool, parallelizable: bool): (coin::BurnCapability&lt;CoinType&gt;, coin::FreezeCapability&lt;CoinType&gt;, coin::MintCapability&lt;CoinType&gt;)
</code></pre>




<pre><code>include InitializeInternalSchema&lt;CoinType&gt; &#123;
    name: name.bytes,
    symbol: symbol.bytes
&#125;;
let account_addr &#61; signer::address_of(account);
let post coin_info &#61; global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
let post supply &#61; option::spec_borrow(coin_info.supply);
let post value &#61; optional_aggregator::optional_aggregator_value(supply);
let post limit &#61; optional_aggregator::optional_aggregator_limit(supply);
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr);
aborts_if monitor_supply &amp;&amp; parallelizable
    &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
// This enforces <a id="high-level-req-2" href="managed_coin.md#high-level-req">high-level requirement 2</a> of the <a href=managed_coin.md>managed_coin</a> module:
ensures exists&lt;CoinInfo&lt;CoinType&gt;&gt;(account_addr)
    &amp;&amp; coin_info.name &#61;&#61; name
    &amp;&amp; coin_info.symbol &#61;&#61; symbol
    &amp;&amp; coin_info.decimals &#61;&#61; decimals;
ensures if (monitor_supply) &#123;
    value &#61;&#61; 0 &amp;&amp; limit &#61;&#61; MAX_U128
        &amp;&amp; (parallelizable &#61;&#61; optional_aggregator::is_parallelizable(supply))
&#125; else &#123;
    option::spec_is_none(coin_info.supply)
&#125;;
ensures result_1 &#61;&#61; BurnCapability&lt;CoinType&gt; &#123;&#125;;
ensures result_2 &#61;&#61; FreezeCapability&lt;CoinType&gt; &#123;&#125;;
ensures result_3 &#61;&#61; MintCapability&lt;CoinType&gt; &#123;&#125;;
</code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code>public fun merge&lt;CoinType&gt;(dst_coin: &amp;mut coin::Coin&lt;CoinType&gt;, source_coin: coin::Coin&lt;CoinType&gt;)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures dst_coin.value &#61;&#61; old(dst_coin.value) &#43; source_coin.value;
</code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code>public fun mint&lt;CoinType&gt;(amount: u64, _cap: &amp;coin::MintCapability&lt;CoinType&gt;): coin::Coin&lt;CoinType&gt;
</code></pre>




<pre><code>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
</code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code>public fun register&lt;CoinType&gt;(account: &amp;signer)
</code></pre>


An account can only be registered once.
Updating <code>Account.guid_creation_num</code> will not overflow.


<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public entry fun transfer&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)
</code></pre>


<code>from</code> and <code>to</code> account not frozen.
<code>from</code> and <code>to</code> not the same address.
<code>from</code> account sufficient balance.


<pre><code>pragma verify &#61; false;
let account_addr_from &#61; signer::address_of(from);
let coin_store_from &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr_from);
let post coin_store_post_from &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr_from);
let coin_store_to &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(to);
let post coin_store_post_to &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(to);
// This enforces <a id="high-level-req-6.5" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr_from);
aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(to);
// This enforces <a id="high-level-req-8.2" href="#high-level-req">high-level requirement 8</a>:
aborts_if coin_store_from.frozen;
aborts_if coin_store_to.frozen;
aborts_if coin_store_from.coin.value &lt; amount;
ensures account_addr_from !&#61; to &#61;&#61;&gt; coin_store_post_from.coin.value &#61;&#61;
    coin_store_from.coin.value &#45; amount;
ensures account_addr_from !&#61; to &#61;&#61;&gt; coin_store_post_to.coin.value &#61;&#61; coin_store_to.coin.value &#43; amount;
ensures account_addr_from &#61;&#61; to &#61;&#61;&gt; coin_store_post_from.coin.value &#61;&#61; coin_store_from.coin.value;
</code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code>public fun withdraw&lt;CoinType&gt;(account: &amp;signer, amount: u64): coin::Coin&lt;CoinType&gt;
</code></pre>


Account is not frozen and sufficient balance.


<pre><code>pragma verify &#61; false;
include WithdrawAbortsIf&lt;CoinType&gt;;
modifies global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
let account_addr &#61; signer::address_of(account);
let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
let balance &#61; coin_store.coin.value;
let post coin_post &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr).coin.value;
ensures coin_post &#61;&#61; balance &#45; amount;
ensures result &#61;&#61; Coin&lt;CoinType&gt; &#123; value: amount &#125;;
</code></pre>




<a id="0x1_coin_WithdrawAbortsIf"></a>


<pre><code>schema WithdrawAbortsIf&lt;CoinType&gt; &#123;
    account: &amp;signer;
    amount: u64;
    let account_addr &#61; signer::address_of(account);
    let coin_store &#61; global&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    let balance &#61; coin_store.coin.value;
    // This enforces <a id="high-level-req-6.6" href="#high-level-req">high-level requirement 6</a>:
    aborts_if !exists&lt;CoinStore&lt;CoinType&gt;&gt;(account_addr);
    // This enforces <a id="high-level-req-8.1" href="#high-level-req">high-level requirement 8</a>:
    aborts_if coin_store.frozen;
    aborts_if balance &lt; amount;
&#125;
</code></pre>



<a id="@Specification_1_mint_internal"></a>

### Function `mint_internal`


<pre><code>fun mint_internal&lt;CoinType&gt;(amount: u64): coin::Coin&lt;CoinType&gt;
</code></pre>




<pre><code>let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
aborts_if (amount !&#61; 0) &amp;&amp; !exists&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
ensures supply&lt;CoinType&gt; &#61;&#61; old(supply&lt;CoinType&gt;) &#43; amount;
ensures result.value &#61;&#61; amount;
</code></pre>



<a id="@Specification_1_burn_internal"></a>

### Function `burn_internal`


<pre><code>fun burn_internal&lt;CoinType&gt;(coin: coin::Coin&lt;CoinType&gt;): u64
</code></pre>




<pre><code>pragma verify &#61; false;
let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
modifies global&lt;CoinInfo&lt;CoinType&gt;&gt;(addr);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
