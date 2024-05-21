
<a id="0x1_fungible_asset"></a>

# Module `0x1::fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code>Metadata</code> object. The
metadata object can be any object that equipped with <code>Metadata</code> resource.


-  [Resource `Supply`](#0x1_fungible_asset_Supply)
-  [Resource `ConcurrentSupply`](#0x1_fungible_asset_ConcurrentSupply)
-  [Resource `Metadata`](#0x1_fungible_asset_Metadata)
-  [Resource `Untransferable`](#0x1_fungible_asset_Untransferable)
-  [Resource `FungibleStore`](#0x1_fungible_asset_FungibleStore)
-  [Resource `DispatchFunctionStore`](#0x1_fungible_asset_DispatchFunctionStore)
-  [Struct `FungibleAsset`](#0x1_fungible_asset_FungibleAsset)
-  [Struct `MintRef`](#0x1_fungible_asset_MintRef)
-  [Struct `TransferRef`](#0x1_fungible_asset_TransferRef)
-  [Struct `BurnRef`](#0x1_fungible_asset_BurnRef)
-  [Struct `Deposit`](#0x1_fungible_asset_Deposit)
-  [Struct `Withdraw`](#0x1_fungible_asset_Withdraw)
-  [Struct `Frozen`](#0x1_fungible_asset_Frozen)
-  [Resource `FungibleAssetEvents`](#0x1_fungible_asset_FungibleAssetEvents)
-  [Struct `DepositEvent`](#0x1_fungible_asset_DepositEvent)
-  [Struct `WithdrawEvent`](#0x1_fungible_asset_WithdrawEvent)
-  [Struct `FrozenEvent`](#0x1_fungible_asset_FrozenEvent)
-  [Constants](#@Constants_0)
-  [Function `add_fungibility`](#0x1_fungible_asset_add_fungibility)
-  [Function `set_untransferable`](#0x1_fungible_asset_set_untransferable)
-  [Function `is_untransferable`](#0x1_fungible_asset_is_untransferable)
-  [Function `register_dispatch_functions`](#0x1_fungible_asset_register_dispatch_functions)
-  [Function `generate_mint_ref`](#0x1_fungible_asset_generate_mint_ref)
-  [Function `generate_burn_ref`](#0x1_fungible_asset_generate_burn_ref)
-  [Function `generate_transfer_ref`](#0x1_fungible_asset_generate_transfer_ref)
-  [Function `supply`](#0x1_fungible_asset_supply)
-  [Function `maximum`](#0x1_fungible_asset_maximum)
-  [Function `name`](#0x1_fungible_asset_name)
-  [Function `symbol`](#0x1_fungible_asset_symbol)
-  [Function `decimals`](#0x1_fungible_asset_decimals)
-  [Function `store_exists`](#0x1_fungible_asset_store_exists)
-  [Function `metadata_from_asset`](#0x1_fungible_asset_metadata_from_asset)
-  [Function `store_metadata`](#0x1_fungible_asset_store_metadata)
-  [Function `amount`](#0x1_fungible_asset_amount)
-  [Function `balance`](#0x1_fungible_asset_balance)
-  [Function `is_balance_at_least`](#0x1_fungible_asset_is_balance_at_least)
-  [Function `is_frozen`](#0x1_fungible_asset_is_frozen)
-  [Function `is_store_dispatchable`](#0x1_fungible_asset_is_store_dispatchable)
-  [Function `deposit_dispatch_function`](#0x1_fungible_asset_deposit_dispatch_function)
-  [Function `has_deposit_dispatch_function`](#0x1_fungible_asset_has_deposit_dispatch_function)
-  [Function `withdraw_dispatch_function`](#0x1_fungible_asset_withdraw_dispatch_function)
-  [Function `has_withdraw_dispatch_function`](#0x1_fungible_asset_has_withdraw_dispatch_function)
-  [Function `derived_balance_dispatch_function`](#0x1_fungible_asset_derived_balance_dispatch_function)
-  [Function `asset_metadata`](#0x1_fungible_asset_asset_metadata)
-  [Function `mint_ref_metadata`](#0x1_fungible_asset_mint_ref_metadata)
-  [Function `transfer_ref_metadata`](#0x1_fungible_asset_transfer_ref_metadata)
-  [Function `burn_ref_metadata`](#0x1_fungible_asset_burn_ref_metadata)
-  [Function `transfer`](#0x1_fungible_asset_transfer)
-  [Function `create_store`](#0x1_fungible_asset_create_store)
-  [Function `remove_store`](#0x1_fungible_asset_remove_store)
-  [Function `withdraw`](#0x1_fungible_asset_withdraw)
-  [Function `withdraw_sanity_check`](#0x1_fungible_asset_withdraw_sanity_check)
-  [Function `deposit_sanity_check`](#0x1_fungible_asset_deposit_sanity_check)
-  [Function `deposit`](#0x1_fungible_asset_deposit)
-  [Function `mint`](#0x1_fungible_asset_mint)
-  [Function `mint_internal`](#0x1_fungible_asset_mint_internal)
-  [Function `mint_to`](#0x1_fungible_asset_mint_to)
-  [Function `set_frozen_flag`](#0x1_fungible_asset_set_frozen_flag)
-  [Function `set_frozen_flag_internal`](#0x1_fungible_asset_set_frozen_flag_internal)
-  [Function `burn`](#0x1_fungible_asset_burn)
-  [Function `burn_internal`](#0x1_fungible_asset_burn_internal)
-  [Function `burn_from`](#0x1_fungible_asset_burn_from)
-  [Function `withdraw_with_ref`](#0x1_fungible_asset_withdraw_with_ref)
-  [Function `deposit_with_ref`](#0x1_fungible_asset_deposit_with_ref)
-  [Function `transfer_with_ref`](#0x1_fungible_asset_transfer_with_ref)
-  [Function `zero`](#0x1_fungible_asset_zero)
-  [Function `extract`](#0x1_fungible_asset_extract)
-  [Function `merge`](#0x1_fungible_asset_merge)
-  [Function `destroy_zero`](#0x1_fungible_asset_destroy_zero)
-  [Function `deposit_internal`](#0x1_fungible_asset_deposit_internal)
-  [Function `withdraw_internal`](#0x1_fungible_asset_withdraw_internal)
-  [Function `increase_supply`](#0x1_fungible_asset_increase_supply)
-  [Function `decrease_supply`](#0x1_fungible_asset_decrease_supply)
-  [Function `borrow_fungible_metadata`](#0x1_fungible_asset_borrow_fungible_metadata)
-  [Function `borrow_fungible_metadata_mut`](#0x1_fungible_asset_borrow_fungible_metadata_mut)
-  [Function `borrow_store_resource`](#0x1_fungible_asset_borrow_store_resource)
-  [Function `upgrade_to_concurrent`](#0x1_fungible_asset_upgrade_to_concurrent)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)


<pre><code>use 0x1::aggregator_v2;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::function_info;
use 0x1::object;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
</code></pre>



<a id="0x1_fungible_asset_Supply"></a>

## Resource `Supply`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct Supply has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: option::Option&lt;u128&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_ConcurrentSupply"></a>

## Resource `ConcurrentSupply`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct ConcurrentSupply has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current: aggregator_v2::Aggregator&lt;u128&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_Metadata"></a>

## Resource `Metadata`

Metadata of a Fungible asset


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct Metadata has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 Name of the fungible metadata, i.e., "USDT".
</dd>
<dt>
<code>symbol: string::String</code>
</dt>
<dd>
 Symbol of the fungible metadata, usually a shorter version of the name.
 For example, Singapore Dollar is SGD.
</dd>
<dt>
<code>decimals: u8</code>
</dt>
<dd>
 Number of decimals used for display purposes.
 For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
 be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).
</dd>
<dt>
<code>icon_uri: string::String</code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to an image that can be used as the icon for this fungible
 asset.
</dd>
<dt>
<code>project_uri: string::String</code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the website for the fungible asset.
</dd>
</dl>


</details>

<a id="0x1_fungible_asset_Untransferable"></a>

## Resource `Untransferable`

Defines a <code>FungibleAsset</code>, such that all <code>FungibleStore</code>s stores are untransferable at
the object layer.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct Untransferable has key
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

<a id="0x1_fungible_asset_FungibleStore"></a>

## Resource `FungibleStore`

The store object that holds fungible assets of a specific type associated with an account.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct FungibleStore has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: object::Object&lt;fungible_asset::Metadata&gt;</code>
</dt>
<dd>
 The address of the base metadata object.
</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>
 The balance of the fungible metadata.
</dd>
<dt>
<code>frozen: bool</code>
</dt>
<dd>
 If true, owner transfer is disabled that only <code>TransferRef</code> can move in/out from this store.
</dd>
</dl>


</details>

<a id="0x1_fungible_asset_DispatchFunctionStore"></a>

## Resource `DispatchFunctionStore`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct DispatchFunctionStore has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>withdraw_function: option::Option&lt;function_info::FunctionInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_function: option::Option&lt;function_info::FunctionInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>derived_balance_function: option::Option&lt;function_info::FunctionInfo&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_FungibleAsset"></a>

## Struct `FungibleAsset`

FungibleAsset can be passed into function for type safety and to guarantee a specific amount.
FungibleAsset is ephemeral and cannot be stored directly. It must be deposited back into a store.


<pre><code>struct FungibleAsset
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: object::Object&lt;fungible_asset::Metadata&gt;</code>
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

<a id="0x1_fungible_asset_MintRef"></a>

## Struct `MintRef`

MintRef can be used to mint the fungible asset into an account's store.


<pre><code>struct MintRef has drop, store
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

<a id="0x1_fungible_asset_TransferRef"></a>

## Struct `TransferRef`

TransferRef can be used to allow or disallow the owner of fungible assets from transferring the asset
and allow the holder of TransferRef to transfer fungible assets from any account.


<pre><code>struct TransferRef has drop, store
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

<a id="0x1_fungible_asset_BurnRef"></a>

## Struct `BurnRef`

BurnRef can be used to burn fungible assets from a given holder account.


<pre><code>struct BurnRef has drop, store
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

<a id="0x1_fungible_asset_Deposit"></a>

## Struct `Deposit`

Emitted when fungible assets are deposited into a store.


<pre><code>&#35;[event]
struct Deposit has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: address</code>
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

<a id="0x1_fungible_asset_Withdraw"></a>

## Struct `Withdraw`

Emitted when fungible assets are withdrawn from a store.


<pre><code>&#35;[event]
struct Withdraw has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: address</code>
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

<a id="0x1_fungible_asset_Frozen"></a>

## Struct `Frozen`

Emitted when a store's frozen status is updated.


<pre><code>&#35;[event]
struct Frozen has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: address</code>
</dt>
<dd>

</dd>
<dt>
<code>frozen: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_FungibleAssetEvents"></a>

## Resource `FungibleAssetEvents`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
&#35;[deprecated]
struct FungibleAssetEvents has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>deposit_events: event::EventHandle&lt;fungible_asset::DepositEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: event::EventHandle&lt;fungible_asset::WithdrawEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>frozen_events: event::EventHandle&lt;fungible_asset::FrozenEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_DepositEvent"></a>

## Struct `DepositEvent`



<pre><code>&#35;[deprecated]
struct DepositEvent has drop, store
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

<a id="0x1_fungible_asset_WithdrawEvent"></a>

## Struct `WithdrawEvent`



<pre><code>&#35;[deprecated]
struct WithdrawEvent has drop, store
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

<a id="0x1_fungible_asset_FrozenEvent"></a>

## Struct `FrozenEvent`



<pre><code>&#35;[deprecated]
struct FrozenEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>frozen: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_fungible_asset_MAX_U128"></a>

Maximum possible coin supply.


<pre><code>const MAX_U128: u128 &#61; 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_fungible_asset_EALREADY_REGISTERED"></a>

Trying to re-register dispatch hook on a fungible asset.


<pre><code>const EALREADY_REGISTERED: u64 &#61; 29;
</code></pre>



<a id="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code>const EAMOUNT_CANNOT_BE_ZERO: u64 &#61; 1;
</code></pre>



<a id="0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO"></a>

Cannot destroy non-empty fungible assets.


<pre><code>const EAMOUNT_IS_NOT_ZERO: u64 &#61; 12;
</code></pre>



<a id="0x1_fungible_asset_EAPT_NOT_DISPATCHABLE"></a>

Cannot register dispatch hook for APT.


<pre><code>const EAPT_NOT_DISPATCHABLE: u64 &#61; 31;
</code></pre>



<a id="0x1_fungible_asset_EBALANCE_IS_NOT_ZERO"></a>

Cannot destroy fungible stores with a non-zero balance.


<pre><code>const EBALANCE_IS_NOT_ZERO: u64 &#61; 14;
</code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

Burn ref and fungible asset do not match.


<pre><code>const EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH: u64 &#61; 13;
</code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH"></a>

Burn ref and store do not match.


<pre><code>const EBURN_REF_AND_STORE_MISMATCH: u64 &#61; 10;
</code></pre>



<a id="0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED"></a>

Flag for Concurrent Supply not enabled


<pre><code>const ECONCURRENT_SUPPLY_NOT_ENABLED: u64 &#61; 22;
</code></pre>



<a id="0x1_fungible_asset_EDECIMALS_TOO_LARGE"></a>

Decimals is over the maximum of 32


<pre><code>const EDECIMALS_TOO_LARGE: u64 &#61; 17;
</code></pre>



<a id="0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided deposit function type doesn't meet the signature requirement.


<pre><code>const EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH: u64 &#61; 26;
</code></pre>



<a id="0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided derived_balance function type doesn't meet the signature requirement.


<pre><code>const EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH: u64 &#61; 27;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH"></a>

Fungible asset and store do not match.


<pre><code>const EFUNGIBLE_ASSET_AND_STORE_MISMATCH: u64 &#61; 11;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH"></a>

Fungible asset do not match when merging.


<pre><code>const EFUNGIBLE_ASSET_MISMATCH: u64 &#61; 6;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE"></a>

Fungible metadata does not exist on this account.


<pre><code>const EFUNGIBLE_METADATA_EXISTENCE: u64 &#61; 30;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE"></a>

Flag for the existence of fungible store.


<pre><code>const EFUNGIBLE_STORE_EXISTENCE: u64 &#61; 23;
</code></pre>



<a id="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient balance to withdraw or transfer.


<pre><code>const EINSUFFICIENT_BALANCE: u64 &#61; 4;
</code></pre>



<a id="0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS"></a>

Invalid withdraw/deposit on dispatchable token. The specified token has a dispatchable function hook.
Need to invoke dispatchable_fungible_asset::withdraw/deposit to perform transfer.


<pre><code>const EINVALID_DISPATCHABLE_OPERATIONS: u64 &#61; 28;
</code></pre>



<a id="0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED"></a>

The fungible asset's supply has exceeded maximum.


<pre><code>const EMAX_SUPPLY_EXCEEDED: u64 &#61; 5;
</code></pre>



<a id="0x1_fungible_asset_EMINT_REF_AND_STORE_MISMATCH"></a>

The mint ref and the store do not match.


<pre><code>const EMINT_REF_AND_STORE_MISMATCH: u64 &#61; 7;
</code></pre>



<a id="0x1_fungible_asset_ENAME_TOO_LONG"></a>

Name of the fungible asset metadata is too long


<pre><code>const ENAME_TOO_LONG: u64 &#61; 15;
</code></pre>



<a id="0x1_fungible_asset_ENOT_METADATA_OWNER"></a>

Account is not the owner of metadata object.


<pre><code>const ENOT_METADATA_OWNER: u64 &#61; 24;
</code></pre>



<a id="0x1_fungible_asset_ENOT_STORE_OWNER"></a>

Account is not the store's owner.


<pre><code>const ENOT_STORE_OWNER: u64 &#61; 8;
</code></pre>



<a id="0x1_fungible_asset_EOBJECT_IS_DELETABLE"></a>

Fungibility is only available for non-deletable objects.


<pre><code>const EOBJECT_IS_DELETABLE: u64 &#61; 18;
</code></pre>



<a id="0x1_fungible_asset_ESTORE_IS_FROZEN"></a>

Store is disabled from sending and receiving this fungible asset.


<pre><code>const ESTORE_IS_FROZEN: u64 &#61; 3;
</code></pre>



<a id="0x1_fungible_asset_ESUPPLY_NOT_FOUND"></a>

Supply resource is not found for a metadata object.


<pre><code>const ESUPPLY_NOT_FOUND: u64 &#61; 21;
</code></pre>



<a id="0x1_fungible_asset_ESUPPLY_UNDERFLOW"></a>

The fungible asset's supply will be negative which should be impossible.


<pre><code>const ESUPPLY_UNDERFLOW: u64 &#61; 20;
</code></pre>



<a id="0x1_fungible_asset_ESYMBOL_TOO_LONG"></a>

Symbol of the fungible asset metadata is too long


<pre><code>const ESYMBOL_TOO_LONG: u64 &#61; 16;
</code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

The transfer ref and the fungible asset do not match.


<pre><code>const ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH: u64 &#61; 2;
</code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH"></a>

Transfer ref and store do not match.


<pre><code>const ETRANSFER_REF_AND_STORE_MISMATCH: u64 &#61; 9;
</code></pre>



<a id="0x1_fungible_asset_EURI_TOO_LONG"></a>

URI for the icon of the fungible asset metadata is too long


<pre><code>const EURI_TOO_LONG: u64 &#61; 19;
</code></pre>



<a id="0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided withdraw function type doesn't meet the signature requirement.


<pre><code>const EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH: u64 &#61; 25;
</code></pre>



<a id="0x1_fungible_asset_MAX_DECIMALS"></a>



<pre><code>const MAX_DECIMALS: u8 &#61; 32;
</code></pre>



<a id="0x1_fungible_asset_MAX_NAME_LENGTH"></a>



<pre><code>const MAX_NAME_LENGTH: u64 &#61; 32;
</code></pre>



<a id="0x1_fungible_asset_MAX_SYMBOL_LENGTH"></a>



<pre><code>const MAX_SYMBOL_LENGTH: u64 &#61; 10;
</code></pre>



<a id="0x1_fungible_asset_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;
</code></pre>



<a id="0x1_fungible_asset_add_fungibility"></a>

## Function `add_fungibility`

Make an existing object fungible by adding the Metadata resource.
This returns the capabilities to mint, burn, and transfer.
maximum_supply defines the behavior of maximum supply when monitoring:
- option::none(): Monitoring unlimited supply
(width of the field - MAX_U128 is the implicit maximum supply)
if option::some(MAX_U128) is used, it is treated as unlimited supply.
- option::some(max): Monitoring fixed supply with <code>max</code> as the maximum supply.


<pre><code>public fun add_fungibility(constructor_ref: &amp;object::ConstructorRef, maximum_supply: option::Option&lt;u128&gt;, name: string::String, symbol: string::String, decimals: u8, icon_uri: string::String, project_uri: string::String): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_fungibility(
    constructor_ref: &amp;ConstructorRef,
    maximum_supply: Option&lt;u128&gt;,
    name: String,
    symbol: String,
    decimals: u8,
    icon_uri: String,
    project_uri: String,
): Object&lt;Metadata&gt; &#123;
    assert!(!object::can_generate_delete_ref(constructor_ref), error::invalid_argument(EOBJECT_IS_DELETABLE));
    let metadata_object_signer &#61; &amp;object::generate_signer(constructor_ref);
    assert!(string::length(&amp;name) &lt;&#61; MAX_NAME_LENGTH, error::out_of_range(ENAME_TOO_LONG));
    assert!(string::length(&amp;symbol) &lt;&#61; MAX_SYMBOL_LENGTH, error::out_of_range(ESYMBOL_TOO_LONG));
    assert!(decimals &lt;&#61; MAX_DECIMALS, error::out_of_range(EDECIMALS_TOO_LARGE));
    assert!(string::length(&amp;icon_uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
    assert!(string::length(&amp;project_uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
    move_to(metadata_object_signer,
        Metadata &#123;
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri,
        &#125;
    );

    if (features::concurrent_fungible_assets_enabled()) &#123;
        let unlimited &#61; option::is_none(&amp;maximum_supply);
        move_to(metadata_object_signer, ConcurrentSupply &#123;
            current: if (unlimited) &#123;
                aggregator_v2::create_unbounded_aggregator()
            &#125; else &#123;
                aggregator_v2::create_aggregator(option::extract(&amp;mut maximum_supply))
            &#125;,
        &#125;);
    &#125; else &#123;
        move_to(metadata_object_signer, Supply &#123;
            current: 0,
            maximum: maximum_supply
        &#125;);
    &#125;;

    object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_set_untransferable"></a>

## Function `set_untransferable`

Set that only untransferable stores can be created for this fungible asset.


<pre><code>public fun set_untransferable(constructor_ref: &amp;object::ConstructorRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_untransferable(constructor_ref: &amp;ConstructorRef) &#123;
    let metadata_addr &#61; object::address_from_constructor_ref(constructor_ref);
    assert!(exists&lt;Metadata&gt;(metadata_addr), error::not_found(EFUNGIBLE_METADATA_EXISTENCE));
    let metadata_signer &#61; &amp;object::generate_signer(constructor_ref);
    move_to(metadata_signer, Untransferable &#123;&#125;);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_is_untransferable"></a>

## Function `is_untransferable`

Returns true if the FA is untransferable.


<pre><code>&#35;[view]
public fun is_untransferable&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_untransferable&lt;T: key&gt;(metadata: Object&lt;T&gt;): bool &#123;
    exists&lt;Untransferable&gt;(object::object_address(&amp;metadata))
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`

Create a fungible asset store whose transfer rule would be overloaded by the provided function.


<pre><code>public(friend) fun register_dispatch_functions(constructor_ref: &amp;object::ConstructorRef, withdraw_function: option::Option&lt;function_info::FunctionInfo&gt;, deposit_function: option::Option&lt;function_info::FunctionInfo&gt;, derived_balance_function: option::Option&lt;function_info::FunctionInfo&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun register_dispatch_functions(
    constructor_ref: &amp;ConstructorRef,
    withdraw_function: Option&lt;FunctionInfo&gt;,
    deposit_function: Option&lt;FunctionInfo&gt;,
    derived_balance_function: Option&lt;FunctionInfo&gt;,
) &#123;
    // Verify that caller type matches callee type so wrongly typed function cannot be registered.
    option::for_each_ref(&amp;withdraw_function, &#124;withdraw_function&#124; &#123;
        let dispatcher_withdraw_function_info &#61; function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b&quot;dispatchable_fungible_asset&quot;),
            string::utf8(b&quot;dispatchable_withdraw&quot;),
        );

        assert!(
            function_info::check_dispatch_type_compatibility(
                &amp;dispatcher_withdraw_function_info,
                withdraw_function
            ),
            error::invalid_argument(
                EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH
            )
        );
    &#125;);

    option::for_each_ref(&amp;deposit_function, &#124;deposit_function&#124; &#123;
        let dispatcher_deposit_function_info &#61; function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b&quot;dispatchable_fungible_asset&quot;),
            string::utf8(b&quot;dispatchable_deposit&quot;),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        assert!(
            function_info::check_dispatch_type_compatibility(
                &amp;dispatcher_deposit_function_info,
                deposit_function
            ),
            error::invalid_argument(
                EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH
            )
        );
    &#125;);

    option::for_each_ref(&amp;derived_balance_function, &#124;balance_function&#124; &#123;
        let dispatcher_derived_balance_function_info &#61; function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b&quot;dispatchable_fungible_asset&quot;),
            string::utf8(b&quot;dispatchable_derived_balance&quot;),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        assert!(
            function_info::check_dispatch_type_compatibility(
                &amp;dispatcher_derived_balance_function_info,
                balance_function
            ),
            error::invalid_argument(
                EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH
            )
        );
    &#125;);

    // Cannot register hook for APT.
    assert!(
        object::address_from_constructor_ref(constructor_ref) !&#61; @aptos_fungible_asset,
        error::permission_denied(EAPT_NOT_DISPATCHABLE)
    );
    assert!(
        !object::can_generate_delete_ref(constructor_ref),
        error::invalid_argument(EOBJECT_IS_DELETABLE)
    );
    assert!(
        !exists&lt;DispatchFunctionStore&gt;(
            object::address_from_constructor_ref(constructor_ref)
        ),
        error::already_exists(EALREADY_REGISTERED)
    );
    assert!(
        exists&lt;Metadata&gt;(
            object::address_from_constructor_ref(constructor_ref)
        ),
        error::not_found(EFUNGIBLE_METADATA_EXISTENCE),
    );

    let store_obj &#61; &amp;object::generate_signer(constructor_ref);

    // Store the overload function hook.
    move_to&lt;DispatchFunctionStore&gt;(
        store_obj,
        DispatchFunctionStore &#123;
            withdraw_function,
            deposit_function,
            derived_balance_function,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_mint_ref"></a>

## Function `generate_mint_ref`

Creates a mint ref that can be used to mint fungible assets from the given fungible object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code>public fun generate_mint_ref(constructor_ref: &amp;object::ConstructorRef): fungible_asset::MintRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mint_ref(constructor_ref: &amp;ConstructorRef): MintRef &#123;
    let metadata &#61; object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref);
    MintRef &#123; metadata &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a burn ref that can be used to burn fungible assets from the given fungible object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code>public fun generate_burn_ref(constructor_ref: &amp;object::ConstructorRef): fungible_asset::BurnRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_burn_ref(constructor_ref: &amp;ConstructorRef): BurnRef &#123;
    let metadata &#61; object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref);
    BurnRef &#123; metadata &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Creates a transfer ref that can be used to freeze/unfreeze/transfer fungible assets from the given fungible
object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code>public fun generate_transfer_ref(constructor_ref: &amp;object::ConstructorRef): fungible_asset::TransferRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_transfer_ref(constructor_ref: &amp;ConstructorRef): TransferRef &#123;
    let metadata &#61; object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref);
    TransferRef &#123; metadata &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_supply"></a>

## Function `supply`

Get the current supply from the <code>metadata</code> object.


<pre><code>&#35;[view]
public fun supply&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): option::Option&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun supply&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; acquires Supply, ConcurrentSupply &#123;
    let metadata_address &#61; object::object_address(&amp;metadata);
    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global&lt;ConcurrentSupply&gt;(metadata_address);
        option::some(aggregator_v2::read(&amp;supply.current))
    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global&lt;Supply&gt;(metadata_address);
        option::some(supply.current)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_maximum"></a>

## Function `maximum`

Get the maximum supply from the <code>metadata</code> object.
If supply is unlimited (or set explicitly to MAX_U128), none is returned


<pre><code>&#35;[view]
public fun maximum&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): option::Option&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun maximum&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; acquires Supply, ConcurrentSupply &#123;
    let metadata_address &#61; object::object_address(&amp;metadata);
    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global&lt;ConcurrentSupply&gt;(metadata_address);
        let max_value &#61; aggregator_v2::max_value(&amp;supply.current);
        if (max_value &#61;&#61; MAX_U128) &#123;
            option::none()
        &#125; else &#123;
            option::some(max_value)
        &#125;
    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global&lt;Supply&gt;(metadata_address);
        supply.maximum
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_name"></a>

## Function `name`

Get the name of the fungible asset from the <code>metadata</code> object.


<pre><code>&#35;[view]
public fun name&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;T: key&gt;(metadata: Object&lt;T&gt;): String acquires Metadata &#123;
    borrow_fungible_metadata(&amp;metadata).name
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_symbol"></a>

## Function `symbol`

Get the symbol of the fungible asset from the <code>metadata</code> object.


<pre><code>&#35;[view]
public fun symbol&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun symbol&lt;T: key&gt;(metadata: Object&lt;T&gt;): String acquires Metadata &#123;
    borrow_fungible_metadata(&amp;metadata).symbol
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_decimals"></a>

## Function `decimals`

Get the decimals from the <code>metadata</code> object.


<pre><code>&#35;[view]
public fun decimals&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun decimals&lt;T: key&gt;(metadata: Object&lt;T&gt;): u8 acquires Metadata &#123;
    borrow_fungible_metadata(&amp;metadata).decimals
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_store_exists"></a>

## Function `store_exists`

Return whether the provided address has a store initialized.


<pre><code>&#35;[view]
public fun store_exists(store: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun store_exists(store: address): bool &#123;
    exists&lt;FungibleStore&gt;(store)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_metadata_from_asset"></a>

## Function `metadata_from_asset`

Return the underlying metadata object


<pre><code>public fun metadata_from_asset(fa: &amp;fungible_asset::FungibleAsset): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun metadata_from_asset(fa: &amp;FungibleAsset): Object&lt;Metadata&gt; &#123;
    fa.metadata
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_store_metadata"></a>

## Function `store_metadata`

Return the underlying metadata object.


<pre><code>&#35;[view]
public fun store_metadata&lt;T: key&gt;(store: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun store_metadata&lt;T: key&gt;(store: Object&lt;T&gt;): Object&lt;Metadata&gt; acquires FungibleStore &#123;
    borrow_store_resource(&amp;store).metadata
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_amount"></a>

## Function `amount`

Return the <code>amount</code> of a given fungible asset.


<pre><code>public fun amount(fa: &amp;fungible_asset::FungibleAsset): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun amount(fa: &amp;FungibleAsset): u64 &#123;
    fa.amount
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of a given store.


<pre><code>&#35;[view]
public fun balance&lt;T: key&gt;(store: object::Object&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance&lt;T: key&gt;(store: Object&lt;T&gt;): u64 acquires FungibleStore &#123;
    if (store_exists(object::object_address(&amp;store))) &#123;
        borrow_store_resource(&amp;store).balance
    &#125; else &#123;
        0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_is_balance_at_least"></a>

## Function `is_balance_at_least`

Check whether the balance of a given store is >= <code>amount</code>.


<pre><code>&#35;[view]
public fun is_balance_at_least&lt;T: key&gt;(store: object::Object&lt;T&gt;, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_balance_at_least&lt;T: key&gt;(store: Object&lt;T&gt;, amount: u64): bool acquires FungibleStore &#123;
    let store_addr &#61; object::object_address(&amp;store);
    if (store_exists(store_addr)) &#123;
        borrow_store_resource(&amp;store).balance &gt;&#61; amount
    &#125; else &#123;
        amount &#61;&#61; 0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_is_frozen"></a>

## Function `is_frozen`

Return whether a store is frozen.

If the store has not been created, we default to returning false so deposits can be sent to it.


<pre><code>&#35;[view]
public fun is_frozen&lt;T: key&gt;(store: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_frozen&lt;T: key&gt;(store: Object&lt;T&gt;): bool acquires FungibleStore &#123;
    store_exists(object::object_address(&amp;store)) &amp;&amp; borrow_store_resource(&amp;store).frozen
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_is_store_dispatchable"></a>

## Function `is_store_dispatchable`

Return whether a fungible asset type is dispatchable.


<pre><code>&#35;[view]
public fun is_store_dispatchable&lt;T: key&gt;(store: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_store_dispatchable&lt;T: key&gt;(store: Object&lt;T&gt;): bool acquires FungibleStore &#123;
    let fa_store &#61; borrow_store_resource(&amp;store);
    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);
    exists&lt;DispatchFunctionStore&gt;(metadata_addr)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_dispatch_function"></a>

## Function `deposit_dispatch_function`



<pre><code>public fun deposit_dispatch_function&lt;T: key&gt;(store: object::Object&lt;T&gt;): option::Option&lt;function_info::FunctionInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_dispatch_function&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; acquires FungibleStore, DispatchFunctionStore &#123;
    let fa_store &#61; borrow_store_resource(&amp;store);
    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);
    if(exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;
        borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).deposit_function
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_has_deposit_dispatch_function"></a>

## Function `has_deposit_dispatch_function`



<pre><code>fun has_deposit_dispatch_function(metadata: object::Object&lt;fungible_asset::Metadata&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun has_deposit_dispatch_function(metadata: Object&lt;Metadata&gt;): bool acquires DispatchFunctionStore &#123;
    let metadata_addr &#61; object::object_address(&amp;metadata);
    // Short circuit on APT for better perf
    if(metadata_addr !&#61; @aptos_fungible_asset &amp;&amp; exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;
        option::is_some(&amp;borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).deposit_function)
    &#125; else &#123;
        false
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_dispatch_function"></a>

## Function `withdraw_dispatch_function`



<pre><code>public fun withdraw_dispatch_function&lt;T: key&gt;(store: object::Object&lt;T&gt;): option::Option&lt;function_info::FunctionInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_dispatch_function&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; acquires FungibleStore, DispatchFunctionStore &#123;
    let fa_store &#61; borrow_store_resource(&amp;store);
    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);
    if(exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;
        borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).withdraw_function
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_has_withdraw_dispatch_function"></a>

## Function `has_withdraw_dispatch_function`



<pre><code>fun has_withdraw_dispatch_function(metadata: object::Object&lt;fungible_asset::Metadata&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun has_withdraw_dispatch_function(metadata: Object&lt;Metadata&gt;): bool acquires DispatchFunctionStore &#123;
    let metadata_addr &#61; object::object_address(&amp;metadata);
    // Short circuit on APT for better perf
    if (metadata_addr !&#61; @aptos_fungible_asset &amp;&amp; exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;
        option::is_some(&amp;borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).withdraw_function)
    &#125; else &#123;
        false
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_derived_balance_dispatch_function"></a>

## Function `derived_balance_dispatch_function`



<pre><code>public(friend) fun derived_balance_dispatch_function&lt;T: key&gt;(store: object::Object&lt;T&gt;): option::Option&lt;function_info::FunctionInfo&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun derived_balance_dispatch_function&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; acquires FungibleStore, DispatchFunctionStore &#123;
    let fa_store &#61; borrow_store_resource(&amp;store);
    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);
    if (exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;
        borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).derived_balance_function
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_asset_metadata"></a>

## Function `asset_metadata`



<pre><code>public fun asset_metadata(fa: &amp;fungible_asset::FungibleAsset): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun asset_metadata(fa: &amp;FungibleAsset): Object&lt;Metadata&gt; &#123;
    fa.metadata
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_mint_ref_metadata"></a>

## Function `mint_ref_metadata`

Get the underlying metadata object from the <code>MintRef</code>.


<pre><code>public fun mint_ref_metadata(ref: &amp;fungible_asset::MintRef): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_ref_metadata(ref: &amp;MintRef): Object&lt;Metadata&gt; &#123;
    ref.metadata
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_transfer_ref_metadata"></a>

## Function `transfer_ref_metadata`

Get the underlying metadata object from the <code>TransferRef</code>.


<pre><code>public fun transfer_ref_metadata(ref: &amp;fungible_asset::TransferRef): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_ref_metadata(ref: &amp;TransferRef): Object&lt;Metadata&gt; &#123;
    ref.metadata
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_burn_ref_metadata"></a>

## Function `burn_ref_metadata`

Get the underlying metadata object from the <code>BurnRef</code>.


<pre><code>public fun burn_ref_metadata(ref: &amp;fungible_asset::BurnRef): object::Object&lt;fungible_asset::Metadata&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn_ref_metadata(ref: &amp;BurnRef): Object&lt;Metadata&gt; &#123;
    ref.metadata
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code>public entry fun transfer&lt;T: key&gt;(sender: &amp;signer, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;T: key&gt;(
    sender: &amp;signer,
    from: Object&lt;T&gt;,
    to: Object&lt;T&gt;,
    amount: u64,
) acquires FungibleStore, DispatchFunctionStore &#123;
    let fa &#61; withdraw(sender, from, amount);
    deposit(to, fa);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_create_store"></a>

## Function `create_store`

Allow an object to hold a store for fungible assets.
Applications can use this to create multiple stores for isolating fungible assets for different purposes.


<pre><code>public fun create_store&lt;T: key&gt;(constructor_ref: &amp;object::ConstructorRef, metadata: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::FungibleStore&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_store&lt;T: key&gt;(
    constructor_ref: &amp;ConstructorRef,
    metadata: Object&lt;T&gt;,
): Object&lt;FungibleStore&gt; &#123;
    let store_obj &#61; &amp;object::generate_signer(constructor_ref);
    move_to(store_obj, FungibleStore &#123;
        metadata: object::convert(metadata),
        balance: 0,
        frozen: false,
    &#125;);
    if (is_untransferable(metadata)) &#123;
        object::set_untransferable(constructor_ref);
    &#125;;
    object::object_from_constructor_ref&lt;FungibleStore&gt;(constructor_ref)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_remove_store"></a>

## Function `remove_store`

Used to delete a store.  Requires the store to be completely empty prior to removing it


<pre><code>public fun remove_store(delete_ref: &amp;object::DeleteRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_store(delete_ref: &amp;DeleteRef) acquires FungibleStore, FungibleAssetEvents &#123;
    let store &#61; &amp;object::object_from_delete_ref&lt;FungibleStore&gt;(delete_ref);
    let addr &#61; object::object_address(store);
    let FungibleStore &#123; metadata: _, balance, frozen: _ &#125;
        &#61; move_from&lt;FungibleStore&gt;(addr);
    assert!(balance &#61;&#61; 0, error::permission_denied(EBALANCE_IS_NOT_ZERO));
    // Cleanup deprecated event handles if exist.
    if (exists&lt;FungibleAssetEvents&gt;(addr)) &#123;
        let FungibleAssetEvents &#123;
            deposit_events,
            withdraw_events,
            frozen_events,
        &#125; &#61; move_from&lt;FungibleAssetEvents&gt;(addr);
        event::destroy_handle(deposit_events);
        event::destroy_handle(withdraw_events);
        event::destroy_handle(frozen_events);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.


<pre><code>public fun withdraw&lt;T: key&gt;(owner: &amp;signer, store: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;T: key&gt;(
    owner: &amp;signer,
    store: Object&lt;T&gt;,
    amount: u64,
): FungibleAsset acquires FungibleStore, DispatchFunctionStore &#123;
    withdraw_sanity_check(owner, store, true);
    withdraw_internal(object::object_address(&amp;store), amount)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_sanity_check"></a>

## Function `withdraw_sanity_check`

Check the permission for withdraw operation.


<pre><code>public(friend) fun withdraw_sanity_check&lt;T: key&gt;(owner: &amp;signer, store: object::Object&lt;T&gt;, abort_on_dispatch: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun withdraw_sanity_check&lt;T: key&gt;(
    owner: &amp;signer,
    store: Object&lt;T&gt;,
    abort_on_dispatch: bool,
) acquires FungibleStore, DispatchFunctionStore &#123;
    assert!(object::owns(store, signer::address_of(owner)), error::permission_denied(ENOT_STORE_OWNER));
    let fa_store &#61; borrow_store_resource(&amp;store);
    assert!(
        !abort_on_dispatch &#124;&#124; !has_withdraw_dispatch_function(fa_store.metadata),
        error::invalid_argument(EINVALID_DISPATCHABLE_OPERATIONS)
    );
    assert!(!fa_store.frozen, error::permission_denied(ESTORE_IS_FROZEN));
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_sanity_check"></a>

## Function `deposit_sanity_check`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code>public fun deposit_sanity_check&lt;T: key&gt;(store: object::Object&lt;T&gt;, abort_on_dispatch: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_sanity_check&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    abort_on_dispatch: bool
) acquires FungibleStore, DispatchFunctionStore &#123;
    let fa_store &#61; borrow_store_resource(&amp;store);
    assert!(
        !abort_on_dispatch &#124;&#124; !has_deposit_dispatch_function(fa_store.metadata),
        error::invalid_argument(EINVALID_DISPATCHABLE_OPERATIONS)
    );
    assert!(!fa_store.frozen, error::permission_denied(ESTORE_IS_FROZEN));
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code>public fun deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) acquires FungibleStore, DispatchFunctionStore &#123;
    deposit_sanity_check(store, true);
    deposit_internal(store, fa);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint the specified <code>amount</code> of the fungible asset.


<pre><code>public fun mint(ref: &amp;fungible_asset::MintRef, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint(ref: &amp;MintRef, amount: u64): FungibleAsset acquires Supply, ConcurrentSupply &#123;
    let metadata &#61; ref.metadata;
    mint_internal(metadata, amount)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_mint_internal"></a>

## Function `mint_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code>public(friend) fun mint_internal(metadata: object::Object&lt;fungible_asset::Metadata&gt;, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun mint_internal(
    metadata: Object&lt;Metadata&gt;,
    amount: u64
): FungibleAsset acquires Supply, ConcurrentSupply &#123;
    increase_supply(&amp;metadata, amount);
    FungibleAsset &#123;
        metadata,
        amount
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_mint_to"></a>

## Function `mint_to`

Mint the specified <code>amount</code> of the fungible asset to a destination store.


<pre><code>public fun mint_to&lt;T: key&gt;(ref: &amp;fungible_asset::MintRef, store: object::Object&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_to&lt;T: key&gt;(ref: &amp;MintRef, store: Object&lt;T&gt;, amount: u64)
acquires FungibleStore, Supply, ConcurrentSupply, DispatchFunctionStore &#123;
    deposit_sanity_check(store, false);
    deposit_internal(store, mint(ref, amount));
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag"></a>

## Function `set_frozen_flag`

Enable/disable a store's ability to do direct transfers of the fungible asset.


<pre><code>public fun set_frozen_flag&lt;T: key&gt;(ref: &amp;fungible_asset::TransferRef, store: object::Object&lt;T&gt;, frozen: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_frozen_flag&lt;T: key&gt;(
    ref: &amp;TransferRef,
    store: Object&lt;T&gt;,
    frozen: bool,
) acquires FungibleStore &#123;
    assert!(
        ref.metadata &#61;&#61; store_metadata(store),
        error::invalid_argument(ETRANSFER_REF_AND_STORE_MISMATCH),
    );
    set_frozen_flag_internal(store, frozen)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag_internal"></a>

## Function `set_frozen_flag_internal`



<pre><code>public(friend) fun set_frozen_flag_internal&lt;T: key&gt;(store: object::Object&lt;T&gt;, frozen: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun set_frozen_flag_internal&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    frozen: bool
) acquires FungibleStore &#123;
    let store_addr &#61; object::object_address(&amp;store);
    borrow_global_mut&lt;FungibleStore&gt;(store_addr).frozen &#61; frozen;

    event::emit(Frozen &#123; store: store_addr, frozen &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_burn"></a>

## Function `burn`

Burns a fungible asset


<pre><code>public fun burn(ref: &amp;fungible_asset::BurnRef, fa: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn(ref: &amp;BurnRef, fa: FungibleAsset) acquires Supply, ConcurrentSupply &#123;
    assert!(
        ref.metadata &#61;&#61; metadata_from_asset(&amp;fa),
        error::invalid_argument(EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH)
    );
    burn_internal(fa);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_burn_internal"></a>

## Function `burn_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code>public(friend) fun burn_internal(fa: fungible_asset::FungibleAsset): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun burn_internal(
    fa: FungibleAsset
): u64 acquires Supply, ConcurrentSupply &#123;
    let FungibleAsset &#123;
        metadata,
        amount
    &#125; &#61; fa;
    decrease_supply(&amp;metadata, amount);
    amount
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_burn_from"></a>

## Function `burn_from`

Burn the <code>amount</code> of the fungible asset from the given store.


<pre><code>public fun burn_from&lt;T: key&gt;(ref: &amp;fungible_asset::BurnRef, store: object::Object&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn_from&lt;T: key&gt;(
    ref: &amp;BurnRef,
    store: Object&lt;T&gt;,
    amount: u64
) acquires FungibleStore, Supply, ConcurrentSupply &#123;
    let metadata &#61; ref.metadata;
    assert!(metadata &#61;&#61; store_metadata(store), error::invalid_argument(EBURN_REF_AND_STORE_MISMATCH));
    let store_addr &#61; object::object_address(&amp;store);
    burn(ref, withdraw_internal(store_addr, amount));
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw <code>amount</code> of the fungible asset from the <code>store</code> ignoring <code>frozen</code>.


<pre><code>public fun withdraw_with_ref&lt;T: key&gt;(ref: &amp;fungible_asset::TransferRef, store: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_ref&lt;T: key&gt;(
    ref: &amp;TransferRef,
    store: Object&lt;T&gt;,
    amount: u64
): FungibleAsset acquires FungibleStore &#123;
    assert!(
        ref.metadata &#61;&#61; store_metadata(store),
        error::invalid_argument(ETRANSFER_REF_AND_STORE_MISMATCH),
    );
    withdraw_internal(object::object_address(&amp;store), amount)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit the fungible asset into the <code>store</code> ignoring <code>frozen</code>.


<pre><code>public fun deposit_with_ref&lt;T: key&gt;(ref: &amp;fungible_asset::TransferRef, store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_with_ref&lt;T: key&gt;(
    ref: &amp;TransferRef,
    store: Object&lt;T&gt;,
    fa: FungibleAsset
) acquires FungibleStore &#123;
    assert!(
        ref.metadata &#61;&#61; fa.metadata,
        error::invalid_argument(ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH)
    );
    deposit_internal(store, fa);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>amount</code> of the fungible asset with <code>TransferRef</code> even it is frozen.


<pre><code>public fun transfer_with_ref&lt;T: key&gt;(transfer_ref: &amp;fungible_asset::TransferRef, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_with_ref&lt;T: key&gt;(
    transfer_ref: &amp;TransferRef,
    from: Object&lt;T&gt;,
    to: Object&lt;T&gt;,
    amount: u64,
) acquires FungibleStore &#123;
    let fa &#61; withdraw_with_ref(transfer_ref, from, amount);
    deposit_with_ref(transfer_ref, to, fa);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_zero"></a>

## Function `zero`

Create a fungible asset with zero amount.
This can be useful when starting a series of computations where the initial value is 0.


<pre><code>public fun zero&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun zero&lt;T: key&gt;(metadata: Object&lt;T&gt;): FungibleAsset &#123;
    FungibleAsset &#123;
        metadata: object::convert(metadata),
        amount: 0,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract a given amount from the given fungible asset and return a new one.


<pre><code>public fun extract(fungible_asset: &amp;mut fungible_asset::FungibleAsset, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract(fungible_asset: &amp;mut FungibleAsset, amount: u64): FungibleAsset &#123;
    assert!(fungible_asset.amount &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
    fungible_asset.amount &#61; fungible_asset.amount &#45; amount;
    FungibleAsset &#123;
        metadata: fungible_asset.metadata,
        amount,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_merge"></a>

## Function `merge`

"Merges" the two given fungible assets. The fungible asset passed in as <code>dst_fungible_asset</code> will have a value
equal to the sum of the two (<code>dst_fungible_asset</code> and <code>src_fungible_asset</code>).


<pre><code>public fun merge(dst_fungible_asset: &amp;mut fungible_asset::FungibleAsset, src_fungible_asset: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun merge(dst_fungible_asset: &amp;mut FungibleAsset, src_fungible_asset: FungibleAsset) &#123;
    let FungibleAsset &#123; metadata, amount &#125; &#61; src_fungible_asset;
    assert!(metadata &#61;&#61; dst_fungible_asset.metadata, error::invalid_argument(EFUNGIBLE_ASSET_MISMATCH));
    dst_fungible_asset.amount &#61; dst_fungible_asset.amount &#43; amount;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_destroy_zero"></a>

## Function `destroy_zero`

Destroy an empty fungible asset.


<pre><code>public fun destroy_zero(fungible_asset: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_zero(fungible_asset: FungibleAsset) &#123;
    let FungibleAsset &#123; amount, metadata: _ &#125; &#61; fungible_asset;
    assert!(amount &#61;&#61; 0, error::invalid_argument(EAMOUNT_IS_NOT_ZERO));
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_internal"></a>

## Function `deposit_internal`



<pre><code>public(friend) fun deposit_internal&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun deposit_internal&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) acquires FungibleStore &#123;
    let FungibleAsset &#123; metadata, amount &#125; &#61; fa;
    if (amount &#61;&#61; 0) return;

    let store_metadata &#61; store_metadata(store);
    assert!(metadata &#61;&#61; store_metadata, error::invalid_argument(EFUNGIBLE_ASSET_AND_STORE_MISMATCH));
    let store_addr &#61; object::object_address(&amp;store);
    let store &#61; borrow_global_mut&lt;FungibleStore&gt;(store_addr);
    store.balance &#61; store.balance &#43; amount;

    event::emit(Deposit &#123; store: store_addr, amount &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_internal"></a>

## Function `withdraw_internal`

Extract <code>amount</code> of the fungible asset from <code>store</code>.


<pre><code>public(friend) fun withdraw_internal(store_addr: address, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun withdraw_internal(
    store_addr: address,
    amount: u64,
): FungibleAsset acquires FungibleStore &#123;
    assert!(exists&lt;FungibleStore&gt;(store_addr), error::not_found(EFUNGIBLE_STORE_EXISTENCE));
    let store &#61; borrow_global_mut&lt;FungibleStore&gt;(store_addr);
    let metadata &#61; store.metadata;
    if (amount !&#61; 0) &#123;
        assert!(store.balance &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        store.balance &#61; store.balance &#45; amount;
        event::emit&lt;Withdraw&gt;(Withdraw &#123; store: store_addr, amount &#125;);
    &#125;;
    FungibleAsset &#123; metadata, amount &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_increase_supply"></a>

## Function `increase_supply`

Increase the supply of a fungible asset by minting.


<pre><code>fun increase_supply&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun increase_supply&lt;T: key&gt;(metadata: &amp;Object&lt;T&gt;, amount: u64) acquires Supply, ConcurrentSupply &#123;
    if (amount &#61;&#61; 0) &#123;
        return
    &#125;;
    let metadata_address &#61; object::object_address(metadata);

    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(metadata_address);
        assert!(
            aggregator_v2::try_add(&amp;mut supply.current, (amount as u128)),
            error::out_of_range(EMAX_SUPPLY_EXCEEDED)
        );
    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global_mut&lt;Supply&gt;(metadata_address);
        if (option::is_some(&amp;supply.maximum)) &#123;
            let max &#61; &#42;option::borrow_mut(&amp;mut supply.maximum);
            assert!(
                max &#45; supply.current &gt;&#61; (amount as u128),
                error::out_of_range(EMAX_SUPPLY_EXCEEDED)
            )
        &#125;;
        supply.current &#61; supply.current &#43; (amount as u128);
    &#125; else &#123;
        abort error::not_found(ESUPPLY_NOT_FOUND)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_decrease_supply"></a>

## Function `decrease_supply`

Decrease the supply of a fungible asset by burning.


<pre><code>fun decrease_supply&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun decrease_supply&lt;T: key&gt;(metadata: &amp;Object&lt;T&gt;, amount: u64) acquires Supply, ConcurrentSupply &#123;
    if (amount &#61;&#61; 0) &#123;
        return
    &#125;;
    let metadata_address &#61; object::object_address(metadata);

    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;
        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(metadata_address);

        assert!(
            aggregator_v2::try_sub(&amp;mut supply.current, (amount as u128)),
            error::out_of_range(ESUPPLY_UNDERFLOW)
        );
    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;
        assert!(exists&lt;Supply&gt;(metadata_address), error::not_found(ESUPPLY_NOT_FOUND));
        let supply &#61; borrow_global_mut&lt;Supply&gt;(metadata_address);
        assert!(
            supply.current &gt;&#61; (amount as u128),
            error::invalid_state(ESUPPLY_UNDERFLOW)
        );
        supply.current &#61; supply.current &#45; (amount as u128);
    &#125; else &#123;
        assert!(false, error::not_found(ESUPPLY_NOT_FOUND));
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata"></a>

## Function `borrow_fungible_metadata`



<pre><code>fun borrow_fungible_metadata&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;): &amp;fungible_asset::Metadata
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_fungible_metadata&lt;T: key&gt;(
    metadata: &amp;Object&lt;T&gt;
): &amp;Metadata acquires Metadata &#123;
    let addr &#61; object::object_address(metadata);
    borrow_global&lt;Metadata&gt;(addr)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata_mut"></a>

## Function `borrow_fungible_metadata_mut`



<pre><code>fun borrow_fungible_metadata_mut&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;): &amp;mut fungible_asset::Metadata
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_fungible_metadata_mut&lt;T: key&gt;(
    metadata: &amp;Object&lt;T&gt;
): &amp;mut Metadata acquires Metadata &#123;
    let addr &#61; object::object_address(metadata);
    borrow_global_mut&lt;Metadata&gt;(addr)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_borrow_store_resource"></a>

## Function `borrow_store_resource`



<pre><code>fun borrow_store_resource&lt;T: key&gt;(store: &amp;object::Object&lt;T&gt;): &amp;fungible_asset::FungibleStore
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_store_resource&lt;T: key&gt;(store: &amp;Object&lt;T&gt;): &amp;FungibleStore acquires FungibleStore &#123;
    let store_addr &#61; object::object_address(store);
    assert!(exists&lt;FungibleStore&gt;(store_addr), error::not_found(EFUNGIBLE_STORE_EXISTENCE));
    borrow_global&lt;FungibleStore&gt;(store_addr)
&#125;
</code></pre>



</details>

<a id="0x1_fungible_asset_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code>public fun upgrade_to_concurrent(ref: &amp;object::ExtendRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_to_concurrent(
    ref: &amp;ExtendRef,
) acquires Supply &#123;
    let metadata_object_address &#61; object::address_from_extend_ref(ref);
    let metadata_object_signer &#61; object::generate_signer_for_extending(ref);
    assert!(
        features::concurrent_fungible_assets_enabled(),
        error::invalid_argument(ECONCURRENT_SUPPLY_NOT_ENABLED)
    );
    assert!(exists&lt;Supply&gt;(metadata_object_address), error::not_found(ESUPPLY_NOT_FOUND));
    let Supply &#123;
        current,
        maximum,
    &#125; &#61; move_from&lt;Supply&gt;(metadata_object_address);

    let unlimited &#61; option::is_none(&amp;maximum);
    let supply &#61; ConcurrentSupply &#123;
        current: if (unlimited) &#123;
            aggregator_v2::create_unbounded_aggregator()
        &#125;
        else &#123;
            aggregator_v2::create_aggregator(option::extract(&amp;mut maximum))
        &#125;,
    &#125;;
    // update current state:
    aggregator_v2::add(&amp;mut supply.current, current);
    move_to(&amp;metadata_object_signer, supply);
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
<td>The metadata associated with the fungible asset is subject to precise size constraints.</td>
<td>Medium</td>
<td>The add_fungibility function has size limitations for the name, symbol, number of decimals, icon_uri, and project_uri field of the Metadata resource.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>2</td>
<td>Adding fungibility to an existing object should initialize the metadata and supply resources and store them under the metadata object address.</td>
<td>Low</td>
<td>The add_fungibility function initializes the Metadata and Supply resources and moves them under the metadata object.</td>
<td>Audited that the Metadata and Supply resources are initialized properly.</td>
</tr>

<tr>
<td>3</td>
<td>Generating mint, burn and transfer references can only be done at object creation time and if the object was added fungibility.</td>
<td>Low</td>
<td>The following functions generate the related references of the Metadata object: 1. generate_mint_ref 2. generate_burn_ref 3. generate_transfer_ref</td>
<td>Audited that the Metadata object exists within the constructor ref.</td>
</tr>

<tr>
<td>4</td>
<td>Only the owner of a store should be allowed to withdraw fungible assets from it.</td>
<td>High</td>
<td>The fungible_asset::withdraw function ensures that the signer owns the store by asserting that the object address matches the address of the signer.</td>
<td>Audited that the address of the signer owns the object.</td>
</tr>

<tr>
<td>5</td>
<td>The transfer, withdrawal and deposit operation should never change the current supply of the fungible asset.</td>
<td>High</td>
<td>The transfer function withdraws the fungible assets from the store and deposits them to the receiver. The withdraw function extracts the fungible asset from the fungible asset store. The deposit function adds the balance to the fungible asset store.</td>
<td>Audited that the supply before and after the operation remains constant.</td>
</tr>

<tr>
<td>6</td>
<td>The owner of the store should only be able to withdraw a certain amount if its store has sufficient balance and is not frozen, unless the withdrawal is performed with a reference, and afterwards the store balance should be decreased.</td>
<td>High</td>
<td>The withdraw function ensures that the store is not frozen before calling withdraw_internal which ensures that the withdrawing amount is greater than 0 and less than the total balance from the store. The withdraw_with_ref ensures that the reference's metadata matches the store metadata.</td>
<td>Audited that it aborts if the withdrawing store is frozen. Audited that it aborts if the store doesn't have sufficient balance. Audited that the balance of the withdrawing store is reduced by amount.</td>
</tr>

<tr>
<td>7</td>
<td>Only the same type of fungible assets should be deposited in a fungible asset store, if the store is not frozen, unless the deposit is performed with a reference, and afterwards the store balance should be increased.</td>
<td>High</td>
<td>The deposit function ensures that store is not frozen and proceeds to call the deposit_internal function which validates the store's metadata and the depositing asset's metadata followed by increasing the store balance by the given amount. The deposit_with_ref ensures that the reference's metadata matches the depositing asset's metadata.</td>
<td>Audited that it aborts if the store is frozen. Audited that it aborts if the asset and asset store are different. Audited that the store's balance is increased by the deposited amount.</td>
</tr>

<tr>
<td>8</td>
<td>An object should only be allowed to hold one store for fungible assets.</td>
<td>Medium</td>
<td>The create_store function initializes a new FungibleStore resource and moves it under the object address.</td>
<td>Audited that the resource was moved under the object.</td>
</tr>

<tr>
<td>9</td>
<td>When a new store is created, the balance should be set by default to the value zero.</td>
<td>High</td>
<td>The create_store function initializes a new fungible asset store with zero balance and stores it under the given construtorRef object.</td>
<td>Audited that the store is properly initialized with zero balance.</td>
</tr>

<tr>
<td>10</td>
<td>A store should only be deleted if its balance is zero.</td>
<td>Medium</td>
<td>The remove_store function validates the store's balance and removes the store under the object address.</td>
<td>Audited that aborts if the balance of the store is not zero. Audited that store is removed from the object address.</td>
</tr>

<tr>
<td>11</td>
<td>Minting and burning should alter the total supply value, and the store balances.</td>
<td>High</td>
<td>The mint process increases the total supply by the amount minted using the increase_supply function. The burn process withdraws the burn amount from the given store and decreases the total supply by the amount burned using the decrease_supply function.</td>
<td>Audited the mint and burn functions that the supply was adjusted accordingly.</td>
</tr>

<tr>
<td>12</td>
<td>It must not be possible to burn an amount of fungible assets larger than their current supply.</td>
<td>High</td>
<td>The burn process ensures that the store has enough balance to burn, by asserting that the supply.current >= amount inside the decrease_supply function.</td>
<td>Audited that it aborts if the provided store doesn't have sufficient balance.</td>
</tr>

<tr>
<td>13</td>
<td>Enabling or disabling store's frozen status should only be done with a valid transfer reference.</td>
<td>High</td>
<td>The set_frozen_flag function ensures that the TransferRef is provided via function argument and that the store's metadata matches the metadata from the reference. It then proceeds to update the frozen flag of the store.</td>
<td>Audited that it aborts if the metadata doesn't match. Audited that the frozen flag is updated properly.</td>
</tr>

<tr>
<td>14</td>
<td>Extracting a specific amount from the fungible asset should be possible only if the total amount that it holds is greater or equal to the provided amount.</td>
<td>High</td>
<td>The extract function validates that the fungible asset has enough balance to extract and then updates it by subtracting the extracted amount.</td>
<td>Audited that it aborts if the asset didn't have sufficient balance. Audited that the balance of the asset is updated. Audited that the extract function returns the extracted asset.</td>
</tr>

<tr>
<td>15</td>
<td>Merging two fungible assets should only be possible if both share the same metadata.</td>
<td>Medium</td>
<td>The merge function validates the metadata of the src and dst asset.</td>
<td>Audited that it aborts if the metadata of the src and dst are not the same.</td>
</tr>

<tr>
<td>16</td>
<td>Post merging two fungible assets, the source asset should have the amount value equal to the sum of the two.</td>
<td>High</td>
<td>The merge function increases dst_fungible_asset.amount by src_fungible_asset.amount.</td>
<td>Audited that the dst_fungible_asset balance is increased by amount.</td>
</tr>

<tr>
<td>17</td>
<td>Fungible assets with zero balance should be destroyed when the amount reaches value 0.</td>
<td>Medium</td>
<td>The destroy_zero ensures that the balance of the asset has the value 0 and destroy the asset.</td>
<td>Audited that it aborts if the balance of the asset is non zero.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify&#61;false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
