
<a id="0x1_fungible_asset"></a>

# Module `0x1::fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code><a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a></code> object. The
metadata object can be any object that equipped with <code><a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a></code> resource.


-  [Resource `Supply`](#0x1_fungible_asset_Supply)
-  [Resource `ConcurrentSupply`](#0x1_fungible_asset_ConcurrentSupply)
-  [Resource `Metadata`](#0x1_fungible_asset_Metadata)
-  [Resource `Untransferable`](#0x1_fungible_asset_Untransferable)
-  [Resource `FungibleStore`](#0x1_fungible_asset_FungibleStore)
-  [Resource `DispatchFunctionStore`](#0x1_fungible_asset_DispatchFunctionStore)
-  [Resource `DeriveSupply`](#0x1_fungible_asset_DeriveSupply)
-  [Resource `ConcurrentFungibleBalance`](#0x1_fungible_asset_ConcurrentFungibleBalance)
-  [Struct `FungibleAsset`](#0x1_fungible_asset_FungibleAsset)
-  [Struct `MintRef`](#0x1_fungible_asset_MintRef)
-  [Struct `TransferRef`](#0x1_fungible_asset_TransferRef)
-  [Struct `BurnRef`](#0x1_fungible_asset_BurnRef)
-  [Struct `MutateMetadataRef`](#0x1_fungible_asset_MutateMetadataRef)
-  [Struct `WithdrawPermission`](#0x1_fungible_asset_WithdrawPermission)
-  [Struct `Deposit`](#0x1_fungible_asset_Deposit)
-  [Struct `Withdraw`](#0x1_fungible_asset_Withdraw)
-  [Struct `Frozen`](#0x1_fungible_asset_Frozen)
-  [Resource `FungibleAssetEvents`](#0x1_fungible_asset_FungibleAssetEvents)
-  [Struct `DepositEvent`](#0x1_fungible_asset_DepositEvent)
-  [Struct `WithdrawEvent`](#0x1_fungible_asset_WithdrawEvent)
-  [Struct `FrozenEvent`](#0x1_fungible_asset_FrozenEvent)
-  [Constants](#@Constants_0)
-  [Function `default_to_concurrent_fungible_supply`](#0x1_fungible_asset_default_to_concurrent_fungible_supply)
-  [Function `allow_upgrade_to_concurrent_fungible_balance`](#0x1_fungible_asset_allow_upgrade_to_concurrent_fungible_balance)
-  [Function `default_to_concurrent_fungible_balance`](#0x1_fungible_asset_default_to_concurrent_fungible_balance)
-  [Function `add_fungibility`](#0x1_fungible_asset_add_fungibility)
-  [Function `set_untransferable`](#0x1_fungible_asset_set_untransferable)
-  [Function `is_untransferable`](#0x1_fungible_asset_is_untransferable)
-  [Function `register_dispatch_functions`](#0x1_fungible_asset_register_dispatch_functions)
-  [Function `register_derive_supply_dispatch_function`](#0x1_fungible_asset_register_derive_supply_dispatch_function)
-  [Function `register_dispatch_function_sanity_check`](#0x1_fungible_asset_register_dispatch_function_sanity_check)
-  [Function `generate_mint_ref`](#0x1_fungible_asset_generate_mint_ref)
-  [Function `generate_burn_ref`](#0x1_fungible_asset_generate_burn_ref)
-  [Function `generate_transfer_ref`](#0x1_fungible_asset_generate_transfer_ref)
-  [Function `generate_mutate_metadata_ref`](#0x1_fungible_asset_generate_mutate_metadata_ref)
-  [Function `supply`](#0x1_fungible_asset_supply)
-  [Function `maximum`](#0x1_fungible_asset_maximum)
-  [Function `name`](#0x1_fungible_asset_name)
-  [Function `symbol`](#0x1_fungible_asset_symbol)
-  [Function `decimals`](#0x1_fungible_asset_decimals)
-  [Function `icon_uri`](#0x1_fungible_asset_icon_uri)
-  [Function `project_uri`](#0x1_fungible_asset_project_uri)
-  [Function `metadata`](#0x1_fungible_asset_metadata)
-  [Function `store_exists`](#0x1_fungible_asset_store_exists)
-  [Function `store_exists_inline`](#0x1_fungible_asset_store_exists_inline)
-  [Function `concurrent_fungible_balance_exists_inline`](#0x1_fungible_asset_concurrent_fungible_balance_exists_inline)
-  [Function `metadata_from_asset`](#0x1_fungible_asset_metadata_from_asset)
-  [Function `store_metadata`](#0x1_fungible_asset_store_metadata)
-  [Function `amount`](#0x1_fungible_asset_amount)
-  [Function `balance`](#0x1_fungible_asset_balance)
-  [Function `is_balance_at_least`](#0x1_fungible_asset_is_balance_at_least)
-  [Function `is_address_balance_at_least`](#0x1_fungible_asset_is_address_balance_at_least)
-  [Function `is_frozen`](#0x1_fungible_asset_is_frozen)
-  [Function `is_store_dispatchable`](#0x1_fungible_asset_is_store_dispatchable)
-  [Function `deposit_dispatch_function`](#0x1_fungible_asset_deposit_dispatch_function)
-  [Function `has_deposit_dispatch_function`](#0x1_fungible_asset_has_deposit_dispatch_function)
-  [Function `withdraw_dispatch_function`](#0x1_fungible_asset_withdraw_dispatch_function)
-  [Function `has_withdraw_dispatch_function`](#0x1_fungible_asset_has_withdraw_dispatch_function)
-  [Function `derived_balance_dispatch_function`](#0x1_fungible_asset_derived_balance_dispatch_function)
-  [Function `derived_supply_dispatch_function`](#0x1_fungible_asset_derived_supply_dispatch_function)
-  [Function `asset_metadata`](#0x1_fungible_asset_asset_metadata)
-  [Function `mint_ref_metadata`](#0x1_fungible_asset_mint_ref_metadata)
-  [Function `transfer_ref_metadata`](#0x1_fungible_asset_transfer_ref_metadata)
-  [Function `burn_ref_metadata`](#0x1_fungible_asset_burn_ref_metadata)
-  [Function `object_from_metadata_ref`](#0x1_fungible_asset_object_from_metadata_ref)
-  [Function `transfer`](#0x1_fungible_asset_transfer)
-  [Function `create_store`](#0x1_fungible_asset_create_store)
-  [Function `remove_store`](#0x1_fungible_asset_remove_store)
-  [Function `withdraw`](#0x1_fungible_asset_withdraw)
-  [Function `withdraw_with_permission`](#0x1_fungible_asset_withdraw_with_permission)
-  [Function `withdraw_permission_check`](#0x1_fungible_asset_withdraw_permission_check)
-  [Function `withdraw_permission_check_by_address`](#0x1_fungible_asset_withdraw_permission_check_by_address)
-  [Function `withdraw_sanity_check`](#0x1_fungible_asset_withdraw_sanity_check)
-  [Function `withdraw_sanity_check_impl`](#0x1_fungible_asset_withdraw_sanity_check_impl)
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
-  [Function `address_burn_from`](#0x1_fungible_asset_address_burn_from)
-  [Function `withdraw_with_ref`](#0x1_fungible_asset_withdraw_with_ref)
-  [Function `deposit_with_ref`](#0x1_fungible_asset_deposit_with_ref)
-  [Function `transfer_with_ref`](#0x1_fungible_asset_transfer_with_ref)
-  [Function `mutate_metadata`](#0x1_fungible_asset_mutate_metadata)
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
-  [Function `upgrade_store_to_concurrent`](#0x1_fungible_asset_upgrade_store_to_concurrent)
-  [Function `ensure_store_upgraded_to_concurrent_internal`](#0x1_fungible_asset_ensure_store_upgraded_to_concurrent_internal)
-  [Function `grant_permission`](#0x1_fungible_asset_grant_permission)
-  [Function `grant_apt_permission`](#0x1_fungible_asset_grant_apt_permission)
-  [Function `refill_permission_with_fa`](#0x1_fungible_asset_refill_permission_with_fa)
-  [Function `revoke_permission`](#0x1_fungible_asset_revoke_permission)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)


<pre><code><b>use</b> <a href="aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_fungible_asset_Supply"></a>

## Resource `Supply`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> <b>has</b> key
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
<code>maximum: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_ConcurrentSupply"></a>

## Resource `ConcurrentSupply`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current: <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u128&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_Metadata"></a>

## Resource `Metadata`

Metadata of a Fungible asset


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of the fungible metadata, i.e., "USDT".
</dd>
<dt>
<code>symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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
 be displayed to a user as <code>5.05</code> (<code>505 / 10 ** 2</code>).
</dd>
<dt>
<code>icon_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to an image that can be used as the icon for this fungible
 asset.
</dd>
<dt>
<code>project_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the website for the fungible asset.
</dd>
</dl>


</details>

<a id="0x1_fungible_asset_Untransferable"></a>

## Resource `Untransferable`

Defines a <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a></code>, such that all <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a></code>s stores are untransferable at
the object layer.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Untransferable">Untransferable</a> <b>has</b> key
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


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
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
 If true, owner transfer is disabled that only <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code> can move in/out from this store.
</dd>
</dl>


</details>

<a id="0x1_fungible_asset_DispatchFunctionStore"></a>

## Resource `DispatchFunctionStore`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>withdraw_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>derived_balance_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_DeriveSupply"></a>

## Resource `DeriveSupply`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dispatch_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_ConcurrentFungibleBalance"></a>

## Resource `ConcurrentFungibleBalance`

The store object that holds concurrent fungible asset balance.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>balance: <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;</code>
</dt>
<dd>
 The balance of the fungible metadata.
</dd>
</dl>


</details>

<a id="0x1_fungible_asset_FungibleAsset"></a>

## Struct `FungibleAsset`

FungibleAsset can be passed into function for type safety and to guarantee a specific amount.
FungibleAsset is ephemeral and cannot be stored directly. It must be deposited back into a store.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
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


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> <b>has</b> drop, store
</code></pre>



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

<a id="0x1_fungible_asset_TransferRef"></a>

## Struct `TransferRef`

TransferRef can be used to allow or disallow the owner of fungible assets from transferring the asset
and allow the holder of TransferRef to transfer fungible assets from any account.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> <b>has</b> drop, store
</code></pre>



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

<a id="0x1_fungible_asset_BurnRef"></a>

## Struct `BurnRef`

BurnRef can be used to burn fungible assets from a given holder account.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> <b>has</b> drop, store
</code></pre>



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

<a id="0x1_fungible_asset_MutateMetadataRef"></a>

## Struct `MutateMetadataRef`

MutateMetadataRef can be used to directly modify the fungible asset's Metadata.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">MutateMetadataRef</a> <b>has</b> drop, store
</code></pre>



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

<a id="0x1_fungible_asset_WithdrawPermission"></a>

## Struct `WithdrawPermission`



<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_Deposit"></a>

## Struct `Deposit`

Emitted when fungible assets are deposited into a store.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Deposit">Deposit</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: <b>address</b></code>
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


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Withdraw">Withdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: <b>address</b></code>
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


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Frozen">Frozen</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: <b>address</b></code>
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



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
#[deprecated]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>deposit_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">fungible_asset::DepositEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">fungible_asset::WithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>frozen_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FrozenEvent">fungible_asset::FrozenEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_DepositEvent"></a>

## Struct `DepositEvent`



<pre><code>#[deprecated]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">DepositEvent</a> <b>has</b> drop, store
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



<pre><code>#[deprecated]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store
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



<pre><code>#[deprecated]
<b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FrozenEvent">FrozenEvent</a> <b>has</b> drop, store
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


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_U128">MAX_U128</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_fungible_asset_EALREADY_REGISTERED"></a>

Trying to re-register dispatch hook on a fungible asset.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>: u64 = 29;
</code></pre>



<a id="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>: u64 = 1;
</code></pre>



<a id="0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO"></a>

Cannot destroy non-empty fungible assets.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO">EAMOUNT_IS_NOT_ZERO</a>: u64 = 12;
</code></pre>



<a id="0x1_fungible_asset_EAPT_NOT_DISPATCHABLE"></a>

Cannot register dispatch hook for APT.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAPT_NOT_DISPATCHABLE">EAPT_NOT_DISPATCHABLE</a>: u64 = 31;
</code></pre>



<a id="0x1_fungible_asset_EBALANCE_IS_NOT_ZERO"></a>

Cannot destroy fungible stores with a non-zero balance.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBALANCE_IS_NOT_ZERO">EBALANCE_IS_NOT_ZERO</a>: u64 = 14;
</code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

Burn ref and fungible asset do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH">EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>: u64 = 13;
</code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH"></a>

Burn ref and store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH">EBURN_REF_AND_STORE_MISMATCH</a>: u64 = 10;
</code></pre>



<a id="0x1_fungible_asset_ECONCURRENT_BALANCE_NOT_ENABLED"></a>

Flag for Concurrent Supply not enabled


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ECONCURRENT_BALANCE_NOT_ENABLED">ECONCURRENT_BALANCE_NOT_ENABLED</a>: u64 = 32;
</code></pre>



<a id="0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED"></a>

Flag for Concurrent Supply not enabled


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED">ECONCURRENT_SUPPLY_NOT_ENABLED</a>: u64 = 22;
</code></pre>



<a id="0x1_fungible_asset_EDECIMALS_TOO_LARGE"></a>

Decimals is over the maximum of 32


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDECIMALS_TOO_LARGE">EDECIMALS_TOO_LARGE</a>: u64 = 17;
</code></pre>



<a id="0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided deposit function type doesn't meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH">EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 26;
</code></pre>



<a id="0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided derived_balance function type doesn't meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH">EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 27;
</code></pre>



<a id="0x1_fungible_asset_EDERIVED_SUPPLY_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided derived_supply function type doesn't meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDERIVED_SUPPLY_FUNCTION_SIGNATURE_MISMATCH">EDERIVED_SUPPLY_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 33;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH"></a>

Fungible asset and store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH">EFUNGIBLE_ASSET_AND_STORE_MISMATCH</a>: u64 = 11;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH"></a>

Fungible asset do not match when merging.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH">EFUNGIBLE_ASSET_MISMATCH</a>: u64 = 6;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE"></a>

Fungible metadata does not exist on this account.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE">EFUNGIBLE_METADATA_EXISTENCE</a>: u64 = 30;
</code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE"></a>

Flag for the existence of fungible store.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>: u64 = 23;
</code></pre>



<a id="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient balance to withdraw or transfer.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 4;
</code></pre>



<a id="0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS"></a>

Invalid withdraw/deposit on dispatchable token. The specified token has a dispatchable function hook.
Need to invoke dispatchable_fungible_asset::withdraw/deposit to perform transfer.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS">EINVALID_DISPATCHABLE_OPERATIONS</a>: u64 = 28;
</code></pre>



<a id="0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED"></a>

The fungible asset's supply has exceeded maximum.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>: u64 = 5;
</code></pre>



<a id="0x1_fungible_asset_EMINT_REF_AND_STORE_MISMATCH"></a>

The mint ref and the store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EMINT_REF_AND_STORE_MISMATCH">EMINT_REF_AND_STORE_MISMATCH</a>: u64 = 7;
</code></pre>



<a id="0x1_fungible_asset_ENAME_TOO_LONG"></a>

Name of the fungible asset metadata is too long


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENAME_TOO_LONG">ENAME_TOO_LONG</a>: u64 = 15;
</code></pre>



<a id="0x1_fungible_asset_ENOT_METADATA_OWNER"></a>

Account is not the owner of metadata object.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENOT_METADATA_OWNER">ENOT_METADATA_OWNER</a>: u64 = 24;
</code></pre>



<a id="0x1_fungible_asset_ENOT_STORE_OWNER"></a>

Account is not the store's owner.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENOT_STORE_OWNER">ENOT_STORE_OWNER</a>: u64 = 8;
</code></pre>



<a id="0x1_fungible_asset_EOBJECT_IS_DELETABLE"></a>

Fungibility is only available for non-deletable objects.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>: u64 = 18;
</code></pre>



<a id="0x1_fungible_asset_ESTORE_IS_FROZEN"></a>

Store is disabled from sending and receiving this fungible asset.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>: u64 = 3;
</code></pre>



<a id="0x1_fungible_asset_ESUPPLY_NOT_FOUND"></a>

Supply resource is not found for a metadata object.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>: u64 = 21;
</code></pre>



<a id="0x1_fungible_asset_ESUPPLY_UNDERFLOW"></a>

The fungible asset's supply will be negative which should be impossible.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>: u64 = 20;
</code></pre>



<a id="0x1_fungible_asset_ESYMBOL_TOO_LONG"></a>

Symbol of the fungible asset metadata is too long


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESYMBOL_TOO_LONG">ESYMBOL_TOO_LONG</a>: u64 = 16;
</code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

The transfer ref and the fungible asset do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>: u64 = 2;
</code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH"></a>

Transfer ref and store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH">ETRANSFER_REF_AND_STORE_MISMATCH</a>: u64 = 9;
</code></pre>



<a id="0x1_fungible_asset_EURI_TOO_LONG"></a>

URI for the icon of the fungible asset metadata is too long


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 = 19;
</code></pre>



<a id="0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided withdraw function type doesn't meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH">EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 25;
</code></pre>



<a id="0x1_fungible_asset_EWITHDRAW_PERMISSION_DENIED"></a>

signer don't have the permission to perform withdraw operation


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_PERMISSION_DENIED">EWITHDRAW_PERMISSION_DENIED</a>: u64 = 34;
</code></pre>



<a id="0x1_fungible_asset_MAX_DECIMALS"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_DECIMALS">MAX_DECIMALS</a>: u8 = 32;
</code></pre>



<a id="0x1_fungible_asset_MAX_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_NAME_LENGTH">MAX_NAME_LENGTH</a>: u64 = 32;
</code></pre>



<a id="0x1_fungible_asset_MAX_SYMBOL_LENGTH"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_SYMBOL_LENGTH">MAX_SYMBOL_LENGTH</a>: u64 = 10;
</code></pre>



<a id="0x1_fungible_asset_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 = 512;
</code></pre>



<a id="0x1_fungible_asset_default_to_concurrent_fungible_supply"></a>

## Function `default_to_concurrent_fungible_supply`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_default_to_concurrent_fungible_supply">default_to_concurrent_fungible_supply</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_default_to_concurrent_fungible_supply">default_to_concurrent_fungible_supply</a>(): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_concurrent_fungible_assets_enabled">features::concurrent_fungible_assets_enabled</a>()
}
</code></pre>



</details>

<a id="0x1_fungible_asset_allow_upgrade_to_concurrent_fungible_balance"></a>

## Function `allow_upgrade_to_concurrent_fungible_balance`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_allow_upgrade_to_concurrent_fungible_balance">allow_upgrade_to_concurrent_fungible_balance</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_allow_upgrade_to_concurrent_fungible_balance">allow_upgrade_to_concurrent_fungible_balance</a>(): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_concurrent_fungible_balance_enabled">features::concurrent_fungible_balance_enabled</a>()
}
</code></pre>



</details>

<a id="0x1_fungible_asset_default_to_concurrent_fungible_balance"></a>

## Function `default_to_concurrent_fungible_balance`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_default_to_concurrent_fungible_balance">default_to_concurrent_fungible_balance</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_default_to_concurrent_fungible_balance">default_to_concurrent_fungible_balance</a>(): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_default_to_concurrent_fungible_balance_enabled">features::default_to_concurrent_fungible_balance_enabled</a>()
}
</code></pre>



</details>

<a id="0x1_fungible_asset_add_fungibility"></a>

## Function `add_fungibility`

Make an existing object fungible by adding the Metadata resource.
This returns the capabilities to mint, burn, and transfer.
maximum_supply defines the behavior of maximum supply when monitoring:
- option::none(): Monitoring unlimited supply
(width of the field - MAX_U128 is the implicit maximum supply)
if option::some(MAX_U128) is used, it is treated as unlimited supply.
- option::some(max): Monitoring fixed supply with <code>max</code> as the maximum supply.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">add_fungibility</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, icon_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, project_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">add_fungibility</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: Option&lt;u128&gt;,
    name: String,
    symbol: String,
    decimals: u8,
    icon_uri: String,
    project_uri: String,
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    <b>assert</b>!(!<a href="object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(constructor_ref), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>));
    <b>let</b> metadata_object_signer = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&name) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_NAME_LENGTH">MAX_NAME_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENAME_TOO_LONG">ENAME_TOO_LONG</a>));
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&symbol) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_SYMBOL_LENGTH">MAX_SYMBOL_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESYMBOL_TOO_LONG">ESYMBOL_TOO_LONG</a>));
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a> &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_DECIMALS">MAX_DECIMALS</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EDECIMALS_TOO_LARGE">EDECIMALS_TOO_LARGE</a>));
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&icon_uri) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&project_uri) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>move_to</b>(metadata_object_signer,
        <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri,
        }
    );

    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_default_to_concurrent_fungible_supply">default_to_concurrent_fungible_supply</a>()) {
        <b>let</b> unlimited = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&maximum_supply);
        <b>move_to</b>(metadata_object_signer, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
            current: <b>if</b> (unlimited) {
                <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>()
            } <b>else</b> {
                <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> maximum_supply))
            },
        });
    } <b>else</b> {
        <b>move_to</b>(metadata_object_signer, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> {
            current: 0,
            maximum: maximum_supply
        });
    };

    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_set_untransferable"></a>

## Function `set_untransferable`

Set that only untransferable stores can be created for this fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_untransferable">set_untransferable</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_untransferable">set_untransferable</a>(constructor_ref: &ConstructorRef) {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref);
    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE">EFUNGIBLE_METADATA_EXISTENCE</a>));
    <b>let</b> metadata_signer = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(metadata_signer, <a href="fungible_asset.md#0x1_fungible_asset_Untransferable">Untransferable</a> {});
}
</code></pre>



</details>

<a id="0x1_fungible_asset_is_untransferable"></a>

## Function `is_untransferable`

Returns true if the FA is untransferable.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_untransferable">is_untransferable</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_untransferable">is_untransferable</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): bool {
    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Untransferable">Untransferable</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata))
}
</code></pre>



</details>

<a id="0x1_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`

Create a fungible asset store whose transfer rule would be overloaded by the provided function.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, withdraw_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, deposit_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, derived_balance_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(
    constructor_ref: &ConstructorRef,
    withdraw_function: Option&lt;FunctionInfo&gt;,
    deposit_function: Option&lt;FunctionInfo&gt;,
    derived_balance_function: Option&lt;FunctionInfo&gt;,
) {
    // Verify that caller type matches callee type so wrongly typed function cannot be registered.
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&withdraw_function, |withdraw_function| {
        <b>let</b> dispatcher_withdraw_function_info = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
            @aptos_framework,
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>"),
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_withdraw"),
        );

        <b>assert</b>!(
            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(
                &dispatcher_withdraw_function_info,
                withdraw_function
            ),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(
                <a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH">EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH</a>
            )
        );
    });

    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&deposit_function, |deposit_function| {
        <b>let</b> dispatcher_deposit_function_info = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
            @aptos_framework,
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>"),
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_deposit"),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        <b>assert</b>!(
            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(
                &dispatcher_deposit_function_info,
                deposit_function
            ),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(
                <a href="fungible_asset.md#0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH">EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH</a>
            )
        );
    });

    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&derived_balance_function, |balance_function| {
        <b>let</b> dispatcher_derived_balance_function_info = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
            @aptos_framework,
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>"),
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_derived_balance"),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        <b>assert</b>!(
            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(
                &dispatcher_derived_balance_function_info,
                balance_function
            ),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(
                <a href="fungible_asset.md#0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH">EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH</a>
            )
        );
    });
    <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_function_sanity_check">register_dispatch_function_sanity_check</a>(constructor_ref);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(
            <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref)
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="fungible_asset.md#0x1_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>)
    );

    <b>let</b> store_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);

    // Store the overload function hook.
    <b>move_to</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(
        store_obj,
        <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
            withdraw_function,
            deposit_function,
            derived_balance_function,
        }
    );
}
</code></pre>



</details>

<a id="0x1_fungible_asset_register_derive_supply_dispatch_function"></a>

## Function `register_derive_supply_dispatch_function`

Define the derived supply dispatch with the provided function.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_derive_supply_dispatch_function">register_derive_supply_dispatch_function</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, dispatch_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_derive_supply_dispatch_function">register_derive_supply_dispatch_function</a>(
    constructor_ref: &ConstructorRef,
    dispatch_function: Option&lt;FunctionInfo&gt;
) {
    // Verify that caller type matches callee type so wrongly typed function cannot be registered.
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&dispatch_function, |supply_function| {
        <b>let</b> <a href="function_info.md#0x1_function_info">function_info</a> = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
            @aptos_framework,
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>"),
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_derived_supply"),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        <b>assert</b>!(
            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(
                &<a href="function_info.md#0x1_function_info">function_info</a>,
                supply_function
            ),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(
                <a href="fungible_asset.md#0x1_fungible_asset_EDERIVED_SUPPLY_FUNCTION_SIGNATURE_MISMATCH">EDERIVED_SUPPLY_FUNCTION_SIGNATURE_MISMATCH</a>
            )
        );
    });
    <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_function_sanity_check">register_dispatch_function_sanity_check</a>(constructor_ref);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a>&gt;(
            <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref)
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="fungible_asset.md#0x1_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>)
    );


    <b>let</b> store_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);

    // Store the overload function hook.
    <b>move_to</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a>&gt;(
        store_obj,
        <a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a> {
            dispatch_function
        }
    );
}
</code></pre>



</details>

<a id="0x1_fungible_asset_register_dispatch_function_sanity_check"></a>

## Function `register_dispatch_function_sanity_check`

Check the requirements for registering a dispatchable function.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_function_sanity_check">register_dispatch_function_sanity_check</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_function_sanity_check">register_dispatch_function_sanity_check</a>(
    constructor_ref: &ConstructorRef,
)  {
    // Cannot register hook for APT.
    <b>assert</b>!(
        <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref) != @aptos_fungible_asset,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAPT_NOT_DISPATCHABLE">EAPT_NOT_DISPATCHABLE</a>)
    );
    <b>assert</b>!(
        !<a href="object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(constructor_ref),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>)
    );
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(
            <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref)
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE">EFUNGIBLE_METADATA_EXISTENCE</a>),
    );
}
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_mint_ref"></a>

## Function `generate_mint_ref`

Creates a mint ref that can be used to mint fungible assets from the given fungible object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_mint_ref">generate_mint_ref</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_mint_ref">generate_mint_ref</a>(constructor_ref: &ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> {
    <b>let</b> metadata = <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> { metadata }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a burn ref that can be used to burn fungible assets from the given fungible object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_burn_ref">generate_burn_ref</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_burn_ref">generate_burn_ref</a>(constructor_ref: &ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> {
    <b>let</b> metadata = <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> { metadata }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Creates a transfer ref that can be used to freeze/unfreeze/transfer fungible assets from the given fungible
object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">generate_transfer_ref</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">generate_transfer_ref</a>(constructor_ref: &ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> {
    <b>let</b> metadata = <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> { metadata }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_generate_mutate_metadata_ref"></a>

## Function `generate_mutate_metadata_ref`

Creates a mutate metadata ref that can be used to change the metadata information of fungible assets from the
given fungible object's constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_mutate_metadata_ref">generate_mutate_metadata_ref</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">fungible_asset::MutateMetadataRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_mutate_metadata_ref">generate_mutate_metadata_ref</a>(constructor_ref: &ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">MutateMetadataRef</a> {
    <b>let</b> metadata = <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">MutateMetadataRef</a> { metadata }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_supply"></a>

## Function `supply`

Get the current supply from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>let</b> metadata_address = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&supply.current))
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current)
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_maximum"></a>

## Function `maximum`

Get the maximum supply from the <code>metadata</code> object.
If supply is unlimited (or set explicitly to MAX_U128), none is returned


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>let</b> metadata_address = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);
        <b>let</b> max_value = <a href="aggregator_v2.md#0x1_aggregator_v2_max_value">aggregator_v2::max_value</a>(&supply.current);
        <b>if</b> (max_value == <a href="fungible_asset.md#0x1_fungible_asset_MAX_U128">MAX_U128</a>) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        } <b>else</b> {
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(max_value)
        }
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);
        supply.maximum
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_name"></a>

## Function `name`

Get the name of the fungible asset from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&metadata).name
}
</code></pre>



</details>

<a id="0x1_fungible_asset_symbol"></a>

## Function `symbol`

Get the symbol of the fungible asset from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&metadata).symbol
}
</code></pre>



</details>

<a id="0x1_fungible_asset_decimals"></a>

## Function `decimals`

Get the decimals from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): u8 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&metadata).decimals
}
</code></pre>



</details>

<a id="0x1_fungible_asset_icon_uri"></a>

## Function `icon_uri`

Get the icon uri from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_icon_uri">icon_uri</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_icon_uri">icon_uri</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&metadata).icon_uri
}
</code></pre>



</details>

<a id="0x1_fungible_asset_project_uri"></a>

## Function `project_uri`

Get the project uri from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_project_uri">project_uri</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_project_uri">project_uri</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&metadata).project_uri
}
</code></pre>



</details>

<a id="0x1_fungible_asset_metadata"></a>

## Function `metadata`

Get the metadata struct from the <code>metadata</code> object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata">metadata</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata">metadata</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    *<a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&metadata)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_store_exists"></a>

## Function `store_exists`

Return whether the provided address has a store initialized.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(store: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(store: <b>address</b>): bool {
    <a href="fungible_asset.md#0x1_fungible_asset_store_exists_inline">store_exists_inline</a>(store)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_store_exists_inline"></a>

## Function `store_exists_inline`

Return whether the provided address has a store initialized.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_exists_inline">store_exists_inline</a>(store: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_exists_inline">store_exists_inline</a>(store: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_concurrent_fungible_balance_exists_inline"></a>

## Function `concurrent_fungible_balance_exists_inline`

Return whether the provided address has a concurrent fungible balance initialized,
at the fungible store address.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(store: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(store: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(store)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_metadata_from_asset"></a>

## Function `metadata_from_asset`

Return the underlying metadata object


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    fa.metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_store_metadata"></a>

## Function `store_metadata`

Return the underlying metadata object.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store).metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_amount"></a>

## Function `amount`

Return the <code>amount</code> of a given fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): u64 {
    fa.amount
}
</code></pre>



</details>

<a id="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of a given store.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>&lt;T: key&gt;(store: Object&lt;T&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>let</b> store_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&store);
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists_inline">store_exists_inline</a>(store_addr)) {
        <b>let</b> store_balance = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store).balance;
        <b>if</b> (store_balance == 0 && <a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(store_addr)) {
            <b>let</b> balance_resource = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(store_addr);
            <a href="aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&balance_resource.balance)
        } <b>else</b> {
            store_balance
        }
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_is_balance_at_least"></a>

## Function `is_balance_at_least`

Check whether the balance of a given store is >= <code>amount</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_balance_at_least">is_balance_at_least</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_balance_at_least">is_balance_at_least</a>&lt;T: key&gt;(store: Object&lt;T&gt;, amount: u64): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>let</b> store_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&store);
    <a href="fungible_asset.md#0x1_fungible_asset_is_address_balance_at_least">is_address_balance_at_least</a>(store_addr, amount)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_is_address_balance_at_least"></a>

## Function `is_address_balance_at_least`

Check whether the balance of a given store is >= <code>amount</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_address_balance_at_least">is_address_balance_at_least</a>(store_addr: <b>address</b>, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_address_balance_at_least">is_address_balance_at_least</a>(store_addr: <b>address</b>, amount: u64): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists_inline">store_exists_inline</a>(store_addr)) {
        <b>let</b> store_balance = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr).balance;
        <b>if</b> (store_balance == 0 && <a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(store_addr)) {
            <b>let</b> balance_resource = <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(store_addr);
            <a href="aggregator_v2.md#0x1_aggregator_v2_is_at_least">aggregator_v2::is_at_least</a>(&balance_resource.balance, amount)
        } <b>else</b> {
            store_balance &gt;= amount
        }
    } <b>else</b> {
        amount == 0
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_is_frozen"></a>

## Function `is_frozen`

Return whether a store is frozen.

If the store has not been created, we default to returning false so deposits can be sent to it.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_frozen">is_frozen</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_frozen">is_frozen</a>&lt;T: key&gt;(store: Object&lt;T&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>let</b> store_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&store);
    <a href="fungible_asset.md#0x1_fungible_asset_store_exists_inline">store_exists_inline</a>(store_addr) && <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr).frozen
}
</code></pre>



</details>

<a id="0x1_fungible_asset_is_store_dispatchable"></a>

## Function `is_store_dispatchable`

Return whether a fungible asset type is dispatchable.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_store_dispatchable">is_store_dispatchable</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_store_dispatchable">is_store_dispatchable</a>&lt;T: key&gt;(store: Object&lt;T&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>let</b> fa_store = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store);
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&fa_store.metadata);
    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_dispatch_function"></a>

## Function `deposit_dispatch_function`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">deposit_dispatch_function</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">deposit_dispatch_function</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>let</b> fa_store = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store);
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&fa_store.metadata);
    <b>if</b>(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) {
        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).deposit_function
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_has_deposit_dispatch_function"></a>

## Function `has_deposit_dispatch_function`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_deposit_dispatch_function">has_deposit_dispatch_function</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_deposit_dispatch_function">has_deposit_dispatch_function</a>(metadata: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    // Short circuit on APT for better perf
    <b>if</b>(metadata_addr != @aptos_fungible_asset && <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).deposit_function)
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_dispatch_function"></a>

## Function `withdraw_dispatch_function`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">withdraw_dispatch_function</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">withdraw_dispatch_function</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>let</b> fa_store = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store);
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&fa_store.metadata);
    <b>if</b>(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) {
        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).withdraw_function
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_has_withdraw_dispatch_function"></a>

## Function `has_withdraw_dispatch_function`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_withdraw_dispatch_function">has_withdraw_dispatch_function</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_withdraw_dispatch_function">has_withdraw_dispatch_function</a>(metadata: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    // Short circuit on APT for better perf
    <b>if</b> (metadata_addr != @aptos_fungible_asset && <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).withdraw_function)
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_derived_balance_dispatch_function"></a>

## Function `derived_balance_dispatch_function`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">derived_balance_dispatch_function</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">derived_balance_dispatch_function</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>let</b> fa_store = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store);
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&fa_store.metadata);
    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) {
        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).derived_balance_function
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_derived_supply_dispatch_function"></a>

## Function `derived_supply_dispatch_function`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_derived_supply_dispatch_function">derived_supply_dispatch_function</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_derived_supply_dispatch_function">derived_supply_dispatch_function</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a>&gt;(metadata_addr)) {
        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DeriveSupply">DeriveSupply</a>&gt;(metadata_addr).dispatch_function
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_asset_metadata"></a>

## Function `asset_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">asset_metadata</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">asset_metadata</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    fa.metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_mint_ref_metadata"></a>

## Function `mint_ref_metadata`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">mint_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">mint_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_transfer_ref_metadata"></a>

## Function `transfer_ref_metadata`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">transfer_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">transfer_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_burn_ref_metadata"></a>

## Function `burn_ref_metadata`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">burn_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">burn_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_object_from_metadata_ref"></a>

## Function `object_from_metadata_ref`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">MutateMetadataRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_object_from_metadata_ref">object_from_metadata_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">fungible_asset::MutateMetadataRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_object_from_metadata_ref">object_from_metadata_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">MutateMetadataRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a id="0x1_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>(sender, from, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
}
</code></pre>



</details>

<a id="0x1_fungible_asset_create_store"></a>

## Function `create_store`

Allow an object to hold a store for fungible assets.
Applications can use this to create multiple stores for isolating fungible assets for different purposes.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_create_store">create_store</a>&lt;T: key&gt;(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_create_store">create_store</a>&lt;T: key&gt;(
    constructor_ref: &ConstructorRef,
    metadata: Object&lt;T&gt;,
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt; {
    <b>let</b> store_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(store_obj, <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
        metadata: <a href="object.md#0x1_object_convert">object::convert</a>(metadata),
        balance: 0,
        frozen: <b>false</b>,
    });

    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_is_untransferable">is_untransferable</a>(metadata)) {
        <a href="object.md#0x1_object_set_untransferable">object::set_untransferable</a>(constructor_ref);
    };

    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_default_to_concurrent_fungible_balance">default_to_concurrent_fungible_balance</a>()) {
        <b>move_to</b>(store_obj, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
            balance: <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),
        });
    };

    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(constructor_ref)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_remove_store"></a>

## Function `remove_store`

Used to delete a store.  Requires the store to be completely empty prior to removing it


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_remove_store">remove_store</a>(delete_ref: &<a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_remove_store">remove_store</a>(delete_ref: &DeleteRef) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>let</b> store = &<a href="object.md#0x1_object_object_from_delete_ref">object::object_from_delete_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(delete_ref);
    <b>let</b> addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(store);
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> { metadata: _, balance, frozen: _ }
        = <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(addr);
    <b>assert</b>!(balance == 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBALANCE_IS_NOT_ZERO">EBALANCE_IS_NOT_ZERO</a>));

    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(addr)) {
        <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> { balance } = <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(addr);
        <b>assert</b>!(<a href="aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&balance) == 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBALANCE_IS_NOT_ZERO">EBALANCE_IS_NOT_ZERO</a>));
    };

    // Cleanup deprecated <a href="event.md#0x1_event">event</a> handles <b>if</b> exist.
    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a>&gt;(addr)) {
        <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a> {
            deposit_events,
            withdraw_events,
            frozen_events,
        } = <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a>&gt;(addr);
        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(deposit_events);
        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(withdraw_events);
        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(frozen_events);
    };
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    store: Object&lt;T&gt;,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">withdraw_sanity_check</a>(owner, store, <b>true</b>);
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check">withdraw_permission_check</a>(owner, store, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), amount)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_with_permission"></a>

## Function `withdraw_with_permission`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_permission">withdraw_with_permission</a>&lt;T: key&gt;(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">fungible_asset::WithdrawPermission</a>&gt;, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_permission">withdraw_with_permission</a>&lt;T: key&gt;(
    perm: &<b>mut</b> Permission&lt;<a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a>&gt;,
    store: Object&lt;T&gt;,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check_impl">withdraw_sanity_check_impl</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_address_of">permissioned_signer::address_of</a>(perm), store, <b>true</b>);
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_consume_permission">permissioned_signer::consume_permission</a>(perm, amount <b>as</b> u256, <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> {
            metadata_address: <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store).metadata)
        }),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_PERMISSION_DENIED">EWITHDRAW_PERMISSION_DENIED</a>)
    );
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), amount)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_permission_check"></a>

## Function `withdraw_permission_check`

Check the permission for withdraw operation.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check">withdraw_permission_check</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check">withdraw_permission_check</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    store: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>assert</b>!(<a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">permissioned_signer::check_permission_consume</a>(owner, amount <b>as</b> u256, <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> {
        metadata_address: <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store).metadata)
    }), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_PERMISSION_DENIED">EWITHDRAW_PERMISSION_DENIED</a>));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_permission_check_by_address"></a>

## Function `withdraw_permission_check_by_address`

Check the permission for withdraw operation.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check_by_address">withdraw_permission_check_by_address</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_address: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check_by_address">withdraw_permission_check_by_address</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata_address: <b>address</b>,
    amount: u64,
) {
    <b>assert</b>!(<a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">permissioned_signer::check_permission_consume</a>(owner, amount <b>as</b> u256, <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> {
        metadata_address,
    }), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_PERMISSION_DENIED">EWITHDRAW_PERMISSION_DENIED</a>));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_sanity_check"></a>

## Function `withdraw_sanity_check`

Check the permission for withdraw operation.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">withdraw_sanity_check</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, abort_on_dispatch: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">withdraw_sanity_check</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    store: Object&lt;T&gt;,
    abort_on_dispatch: bool,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check_impl">withdraw_sanity_check_impl</a>(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner),
        store,
        abort_on_dispatch,
    )
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_sanity_check_impl"></a>

## Function `withdraw_sanity_check_impl`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check_impl">withdraw_sanity_check_impl</a>&lt;T: key&gt;(owner_address: <b>address</b>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, abort_on_dispatch: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check_impl">withdraw_sanity_check_impl</a>&lt;T: key&gt;(
    owner_address: <b>address</b>,
    store: Object&lt;T&gt;,
    abort_on_dispatch: bool,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>assert</b>!(<a href="object.md#0x1_object_owns">object::owns</a>(store, owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENOT_STORE_OWNER">ENOT_STORE_OWNER</a>));
    <b>let</b> fa_store = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store);
    <b>assert</b>!(
        !abort_on_dispatch || !<a href="fungible_asset.md#0x1_fungible_asset_has_withdraw_dispatch_function">has_withdraw_dispatch_function</a>(fa_store.metadata),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS">EINVALID_DISPATCHABLE_OPERATIONS</a>)
    );
    <b>assert</b>!(!fa_store.frozen, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_sanity_check"></a>

## Function `deposit_sanity_check`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, abort_on_dispatch: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    abort_on_dispatch: bool
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> {
    <b>let</b> fa_store = <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&store);
    <b>assert</b>!(
        !abort_on_dispatch || !<a href="fungible_asset.md#0x1_fungible_asset_has_deposit_dispatch_function">has_deposit_dispatch_function</a>(fa_store.metadata),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS">EINVALID_DISPATCHABLE_OPERATIONS</a>)
    );
    <b>assert</b>!(!fa_store.frozen, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: Object&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>(store, <b>true</b>);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), fa);
}
</code></pre>



</details>

<a id="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint the specified <code>amount</code> of the fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>let</b> metadata = ref.metadata;
    <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">mint_internal</a>(metadata, amount)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_mint_internal"></a>

## Function `mint_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">mint_internal</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">mint_internal</a>(
    metadata: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;,
    amount: u64
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>(&metadata, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata,
        amount
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_mint_to"></a>

## Function `mint_to`

Mint the specified <code>amount</code> of the fungible asset to a destination store.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_to">mint_to</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_to">mint_to</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>, store: Object&lt;T&gt;, amount: u64)
<b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>(store, <b>false</b>);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref, amount));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag"></a>

## Function `set_frozen_flag`

Enable/disable a store's ability to do direct transfers of the fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag">set_frozen_flag</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, frozen: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag">set_frozen_flag</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    store: Object&lt;T&gt;,
    frozen: bool,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>assert</b>!(
        ref.metadata == <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>(store),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH">ETRANSFER_REF_AND_STORE_MISMATCH</a>),
    );
    <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">set_frozen_flag_internal</a>(store, frozen)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag_internal"></a>

## Function `set_frozen_flag_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">set_frozen_flag_internal</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, frozen: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">set_frozen_flag_internal</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    frozen: bool
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>let</b> store_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&store);
    <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr).frozen = frozen;

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="fungible_asset.md#0x1_fungible_asset_Frozen">Frozen</a> { store: store_addr, frozen });
}
</code></pre>



</details>

<a id="0x1_fungible_asset_burn"></a>

## Function `burn`

Burns a fungible asset


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>assert</b>!(
        ref.metadata == <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(&fa),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH">EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>)
    );
    <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">burn_internal</a>(fa);
}
</code></pre>



</details>

<a id="0x1_fungible_asset_burn_internal"></a>

## Function `burn_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">burn_internal</a>(fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">burn_internal</a>(
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata,
        amount
    } = fa;
    <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>(&metadata, amount);
    amount
}
</code></pre>



</details>

<a id="0x1_fungible_asset_burn_from"></a>

## Function `burn_from`

Burn the <code>amount</code> of the fungible asset from the given store.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_from">burn_from</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_from">burn_from</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>,
    store: Object&lt;T&gt;,
    amount: u64
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    // ref metadata match is checked in <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>() call
    <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref, <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), amount));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_address_burn_from"></a>

## Function `address_burn_from`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_address_burn_from">address_burn_from</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, store_addr: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_address_burn_from">address_burn_from</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>,
    store_addr: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    // ref metadata match is checked in <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>() call
    <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref, <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(store_addr, amount));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw <code>amount</code> of the fungible asset from the <code>store</code> ignoring <code>frozen</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    store: Object&lt;T&gt;,
    amount: u64
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>assert</b>!(
        ref.metadata == <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>(store),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH">ETRANSFER_REF_AND_STORE_MISMATCH</a>),
    );
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), amount)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit the fungible asset into the <code>store</code> ignoring <code>frozen</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    store: Object&lt;T&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>assert</b>!(
        ref.metadata == fa.metadata,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>)
    );
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), fa);
}
</code></pre>



</details>

<a id="0x1_fungible_asset_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>amount</code> of the fungible asset with <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code> even it is frozen.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>&lt;T: key&gt;(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>&lt;T: key&gt;(
    transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>(transfer_ref, from, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>(transfer_ref, <b>to</b>, fa);
}
</code></pre>



</details>

<a id="0x1_fungible_asset_mutate_metadata"></a>

## Function `mutate_metadata`

Mutate specified fields of the fungible asset's <code><a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mutate_metadata">mutate_metadata</a>(metadata_ref: &<a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">fungible_asset::MutateMetadataRef</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, decimals: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u8&gt;, icon_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, project_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mutate_metadata">mutate_metadata</a>(
    metadata_ref: &<a href="fungible_asset.md#0x1_fungible_asset_MutateMetadataRef">MutateMetadataRef</a>,
    name: Option&lt;String&gt;,
    symbol: Option&lt;String&gt;,
    decimals: Option&lt;u8&gt;,
    icon_uri: Option&lt;String&gt;,
    project_uri: Option&lt;String&gt;,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <b>let</b> metadata_address = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata_ref.metadata);
    <b>let</b> mutable_metadata = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(metadata_address);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&name)){
        <b>let</b> name = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> name);
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&name) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_NAME_LENGTH">MAX_NAME_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENAME_TOO_LONG">ENAME_TOO_LONG</a>));
        mutable_metadata.name = name;
    };
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&symbol)){
        <b>let</b> symbol = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> symbol);
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&symbol) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_SYMBOL_LENGTH">MAX_SYMBOL_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESYMBOL_TOO_LONG">ESYMBOL_TOO_LONG</a>));
        mutable_metadata.symbol = symbol;
    };
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&decimals)){
        <b>let</b> decimals = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> decimals);
        <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a> &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_DECIMALS">MAX_DECIMALS</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EDECIMALS_TOO_LARGE">EDECIMALS_TOO_LARGE</a>));
        mutable_metadata.decimals = decimals;
    };
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&icon_uri)){
        <b>let</b> icon_uri = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> icon_uri);
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&icon_uri) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>));
        mutable_metadata.icon_uri = icon_uri;
    };
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&project_uri)){
        <b>let</b> project_uri = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> project_uri);
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&project_uri) &lt;= <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>));
        mutable_metadata.project_uri = project_uri;
    };
}
</code></pre>



</details>

<a id="0x1_fungible_asset_zero"></a>

## Function `zero`

Create a fungible asset with zero amount.
This can be useful when starting a series of computations where the initial value is 0.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_zero">zero</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_zero">zero</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata: <a href="object.md#0x1_object_convert">object::convert</a>(metadata),
        amount: 0,
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract a given amount from the given fungible asset and return a new one.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount = <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount - amount;
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata: <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.metadata,
        amount,
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_merge"></a>

## Function `merge`

"Merges" the two given fungible assets. The fungible asset passed in as <code>dst_fungible_asset</code> will have a value
equal to the sum of the two (<code>dst_fungible_asset</code> and <code>src_fungible_asset</code>).


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(dst_fungible_asset: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, src_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(dst_fungible_asset: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>, src_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata, amount } = src_fungible_asset;
    <b>assert</b>!(metadata == dst_fungible_asset.metadata, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH">EFUNGIBLE_ASSET_MISMATCH</a>));
    dst_fungible_asset.amount = dst_fungible_asset.amount + amount;
}
</code></pre>



</details>

<a id="0x1_fungible_asset_destroy_zero"></a>

## Function `destroy_zero`

Destroy an empty fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destroy_zero">destroy_zero</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destroy_zero">destroy_zero</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { amount, metadata: _ } = <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>;
    <b>assert</b>!(amount == 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO">EAMOUNT_IS_NOT_ZERO</a>));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_deposit_internal"></a>

## Function `deposit_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(store_addr: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(store_addr: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata, amount } = fa;
    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>));
    <b>let</b> store = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr);
    <b>assert</b>!(metadata == store.metadata, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH">EFUNGIBLE_ASSET_AND_STORE_MISMATCH</a>));

    <b>if</b> (amount == 0) <b>return</b>;

    <b>if</b> (store.balance == 0 && <a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(store_addr)) {
        <b>let</b> balance_resource = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(store_addr);
        <a href="aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&<b>mut</b> balance_resource.balance, amount);
    } <b>else</b> {
        store.balance = store.balance + amount;
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="fungible_asset.md#0x1_fungible_asset_Deposit">Deposit</a> { store: store_addr, amount });
}
</code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_internal"></a>

## Function `withdraw_internal`

Extract <code>amount</code> of the fungible asset from <code>store</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(store_addr: <b>address</b>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(
    store_addr: <b>address</b>,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>));

    <b>let</b> store = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr);
    <b>let</b> metadata = store.metadata;
    <b>if</b> (amount != 0) {
        <b>if</b> (store.balance == 0 && <a href="fungible_asset.md#0x1_fungible_asset_concurrent_fungible_balance_exists_inline">concurrent_fungible_balance_exists_inline</a>(store_addr)) {
            <b>let</b> balance_resource = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(store_addr);
            <b>assert</b>!(
                <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">aggregator_v2::try_sub</a>(&<b>mut</b> balance_resource.balance, amount),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>)
            );
        } <b>else</b> {
            <b>assert</b>!(store.balance &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
            store.balance = store.balance - amount;
        };

        <a href="event.md#0x1_event_emit">event::emit</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Withdraw">Withdraw</a>&gt;(<a href="fungible_asset.md#0x1_fungible_asset_Withdraw">Withdraw</a> { store: store_addr, amount });
    };
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata, amount }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_increase_supply"></a>

## Function `increase_supply`

Increase the supply of a fungible asset by minting.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>if</b> (amount == 0) {
        <b>return</b>
    };
    <b>let</b> metadata_address = <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);

    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);
        <b>assert</b>!(
            <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">aggregator_v2::try_add</a>(&<b>mut</b> supply.current, (amount <b>as</b> u128)),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>)
        );
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&supply.maximum)) {
            <b>let</b> max = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> supply.maximum);
            <b>assert</b>!(
                max - supply.current &gt;= (amount <b>as</b> u128),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>)
            )
        };
        supply.current = supply.current + (amount <b>as</b> u128);
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>)
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_decrease_supply"></a>

## Function `decrease_supply`

Decrease the supply of a fungible asset by burning.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
    <b>if</b> (amount == 0) {
        <b>return</b>
    };
    <b>let</b> metadata_address = <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);

    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);

        <b>assert</b>!(
            <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">aggregator_v2::try_sub</a>(&<b>mut</b> supply.current, (amount <b>as</b> u128)),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>)
        );
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) {
        <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>));
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);
        <b>assert</b>!(
            supply.current &gt;= (amount <b>as</b> u128),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>)
        );
        supply.current = supply.current - (amount <b>as</b> u128);
    } <b>else</b> {
        <b>assert</b>!(<b>false</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>));
    }
}
</code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata"></a>

## Function `borrow_fungible_metadata`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>&lt;T: key&gt;(
    metadata: &Object&lt;T&gt;
): &<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <b>let</b> addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);
    <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata_mut"></a>

## Function `borrow_fungible_metadata_mut`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata_mut">borrow_fungible_metadata_mut</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata_mut">borrow_fungible_metadata_mut</a>&lt;T: key&gt;(
    metadata: &Object&lt;T&gt;
): &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> {
    <b>let</b> addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);
    <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_borrow_store_resource"></a>

## Function `borrow_store_resource`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>&lt;T: key&gt;(store: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>&lt;T: key&gt;(store: &Object&lt;T&gt;): &<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>let</b> store_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(store);
    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>));
    <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_upgrade_to_concurrent">upgrade_to_concurrent</a>(ref: &<a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_upgrade_to_concurrent">upgrade_to_concurrent</a>(
    ref: &ExtendRef,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> {
    <b>let</b> metadata_object_address = <a href="object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(ref);
    <b>let</b> metadata_object_signer = <a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(ref);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_concurrent_fungible_assets_enabled">features::concurrent_fungible_assets_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED">ECONCURRENT_SUPPLY_NOT_ENABLED</a>)
    );
    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_object_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>));
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> {
        current,
        maximum,
    } = <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_object_address);

    <b>let</b> unlimited = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&maximum);
    <b>let</b> supply = <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> {
        current: <b>if</b> (unlimited) {
            <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator_with_value">aggregator_v2::create_unbounded_aggregator_with_value</a>(current)
        }
        <b>else</b> {
            <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator_with_value">aggregator_v2::create_aggregator_with_value</a>(current, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> maximum))
        },
    };
    <b>move_to</b>(&metadata_object_signer, supply);
}
</code></pre>



</details>

<a id="0x1_fungible_asset_upgrade_store_to_concurrent"></a>

## Function `upgrade_store_to_concurrent`



<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_upgrade_store_to_concurrent">upgrade_store_to_concurrent</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_upgrade_store_to_concurrent">upgrade_store_to_concurrent</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    store: Object&lt;T&gt;,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>assert</b>!(<a href="object.md#0x1_object_owns">object::owns</a>(store, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENOT_STORE_OWNER">ENOT_STORE_OWNER</a>));
    <b>assert</b>!(!<a href="fungible_asset.md#0x1_fungible_asset_is_frozen">is_frozen</a>(store), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>));
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_allow_upgrade_to_concurrent_fungible_balance">allow_upgrade_to_concurrent_fungible_balance</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ECONCURRENT_BALANCE_NOT_ENABLED">ECONCURRENT_BALANCE_NOT_ENABLED</a>));
    <a href="fungible_asset.md#0x1_fungible_asset_ensure_store_upgraded_to_concurrent_internal">ensure_store_upgraded_to_concurrent_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store));
}
</code></pre>



</details>

<a id="0x1_fungible_asset_ensure_store_upgraded_to_concurrent_internal"></a>

## Function `ensure_store_upgraded_to_concurrent_internal`

Ensure a known <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a></code> has <code><a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a></code>.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ensure_store_upgraded_to_concurrent_internal">ensure_store_upgraded_to_concurrent_internal</a>(fungible_store_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ensure_store_upgraded_to_concurrent_internal">ensure_store_upgraded_to_concurrent_internal</a>(
    fungible_store_address: <b>address</b>,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a>&gt;(fungible_store_address)) {
        <b>return</b>
    };
    <b>let</b> store = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(fungible_store_address);
    <b>let</b> balance = <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator_with_value">aggregator_v2::create_unbounded_aggregator_with_value</a>(store.balance);
    store.balance = 0;
    <b>let</b> object_signer = <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(fungible_store_address);
    <b>move_to</b>(&object_signer, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">ConcurrentFungibleBalance</a> { balance });
}
</code></pre>



</details>

<a id="0x1_fungible_asset_grant_permission"></a>

## Function `grant_permission`

Permission management

Master signer grant permissioned signer ability to withdraw a given amount of fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_grant_permission">grant_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_grant_permission">grant_permission</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;,
    amount: u64
) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">permissioned_signer::authorize</a>(
        master,
        permissioned,
        amount <b>as</b> u256,
        <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> {
            metadata_address: <a href="object.md#0x1_object_object_address">object::object_address</a>(&token_type),
        }
    )
}
</code></pre>



</details>

<a id="0x1_fungible_asset_grant_apt_permission"></a>

## Function `grant_apt_permission`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_grant_apt_permission">grant_apt_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_grant_apt_permission">grant_apt_permission</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    amount: u64
) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">permissioned_signer::authorize</a>(
        master,
        permissioned,
        amount <b>as</b> u256,
        <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> { metadata_address: @aptos_fungible_asset }
    )
}
</code></pre>



</details>

<a id="0x1_fungible_asset_refill_permission_with_fa"></a>

## Function `refill_permission_with_fa`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_refill_permission_with_fa">refill_permission_with_fa</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_refill_permission_with_fa">refill_permission_with_fa</a>(
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_increase_limit">permissioned_signer::increase_limit</a>(
        permissioned,
        <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa) <b>as</b> u256,
        <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> {
            metadata_address: <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa)),
        }
    )
}
</code></pre>



</details>

<a id="0x1_fungible_asset_revoke_permission"></a>

## Function `revoke_permission`

Removing permissions from permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_revoke_permission">revoke_permission</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_revoke_permission">revoke_permission</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">permissioned_signer::revoke_permission</a>(permissioned, <a href="fungible_asset.md#0x1_fungible_asset_WithdrawPermission">WithdrawPermission</a> {
        metadata_address: <a href="object.md#0x1_object_object_address">object::object_address</a>(&token_type),
    })
}
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


<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
