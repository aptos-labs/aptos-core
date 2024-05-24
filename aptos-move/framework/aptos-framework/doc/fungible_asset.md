
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


<pre><code><b>use</b> <a href="aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;<br /><b>use</b> <a href="object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /></code></pre>



<a id="0x1_fungible_asset_Supply"></a>

## Resource `Supply`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> <b>has</b> key<br /></code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> <b>has</b> key<br /></code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of the fungible metadata, i.e., &quot;USDT&quot;.
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
 be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).
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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Untransferable">Untransferable</a> <b>has</b> key<br /></code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> <b>has</b> key<br /></code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_fungible_asset_FungibleAsset"></a>

## Struct `FungibleAsset`

FungibleAsset can be passed into function for type safety and to guarantee a specific amount.
FungibleAsset is ephemeral and cannot be stored directly. It must be deposited back into a store.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a><br /></code></pre>



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

MintRef can be used to mint the fungible asset into an account&apos;s store.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_fungible_asset_Deposit"></a>

## Struct `Deposit`

Emitted when fungible assets are deposited into a store.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Deposit">Deposit</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Withdraw">Withdraw</a> <b>has</b> drop, store<br /></code></pre>



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

Emitted when a store&apos;s frozen status is updated.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_Frozen">Frozen</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br />&#35;[deprecated]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a> <b>has</b> key<br /></code></pre>



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



<pre><code>&#35;[deprecated]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">DepositEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[deprecated]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[deprecated]<br /><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FrozenEvent">FrozenEvent</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_U128">MAX_U128</a>: u128 &#61; 340282366920938463463374607431768211455;<br /></code></pre>



<a id="0x1_fungible_asset_EALREADY_REGISTERED"></a>

Trying to re&#45;register dispatch hook on a fungible asset.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>: u64 &#61; 29;<br /></code></pre>



<a id="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO"></a>

Cannot destroy non&#45;empty fungible assets.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO">EAMOUNT_IS_NOT_ZERO</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_fungible_asset_EAPT_NOT_DISPATCHABLE"></a>

Cannot register dispatch hook for APT.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAPT_NOT_DISPATCHABLE">EAPT_NOT_DISPATCHABLE</a>: u64 &#61; 31;<br /></code></pre>



<a id="0x1_fungible_asset_EBALANCE_IS_NOT_ZERO"></a>

Cannot destroy fungible stores with a non&#45;zero balance.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBALANCE_IS_NOT_ZERO">EBALANCE_IS_NOT_ZERO</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

Burn ref and fungible asset do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH">EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH"></a>

Burn ref and store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH">EBURN_REF_AND_STORE_MISMATCH</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED"></a>

Flag for Concurrent Supply not enabled


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED">ECONCURRENT_SUPPLY_NOT_ENABLED</a>: u64 &#61; 22;<br /></code></pre>



<a id="0x1_fungible_asset_EDECIMALS_TOO_LARGE"></a>

Decimals is over the maximum of 32


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDECIMALS_TOO_LARGE">EDECIMALS_TOO_LARGE</a>: u64 &#61; 17;<br /></code></pre>



<a id="0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided deposit function type doesn&apos;t meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH">EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH</a>: u64 &#61; 26;<br /></code></pre>



<a id="0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided derived_balance function type doesn&apos;t meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH">EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH</a>: u64 &#61; 27;<br /></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH"></a>

Fungible asset and store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH">EFUNGIBLE_ASSET_AND_STORE_MISMATCH</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH"></a>

Fungible asset do not match when merging.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH">EFUNGIBLE_ASSET_MISMATCH</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE"></a>

Fungible metadata does not exist on this account.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE">EFUNGIBLE_METADATA_EXISTENCE</a>: u64 &#61; 30;<br /></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE"></a>

Flag for the existence of fungible store.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>: u64 &#61; 23;<br /></code></pre>



<a id="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient balance to withdraw or transfer.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS"></a>

Invalid withdraw/deposit on dispatchable token. The specified token has a dispatchable function hook.
Need to invoke dispatchable_fungible_asset::withdraw/deposit to perform transfer.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS">EINVALID_DISPATCHABLE_OPERATIONS</a>: u64 &#61; 28;<br /></code></pre>



<a id="0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED"></a>

The fungible asset&apos;s supply has exceeded maximum.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_fungible_asset_EMINT_REF_AND_STORE_MISMATCH"></a>

The mint ref and the store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EMINT_REF_AND_STORE_MISMATCH">EMINT_REF_AND_STORE_MISMATCH</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_fungible_asset_ENAME_TOO_LONG"></a>

Name of the fungible asset metadata is too long


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENAME_TOO_LONG">ENAME_TOO_LONG</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x1_fungible_asset_ENOT_METADATA_OWNER"></a>

Account is not the owner of metadata object.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENOT_METADATA_OWNER">ENOT_METADATA_OWNER</a>: u64 &#61; 24;<br /></code></pre>



<a id="0x1_fungible_asset_ENOT_STORE_OWNER"></a>

Account is not the store&apos;s owner.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENOT_STORE_OWNER">ENOT_STORE_OWNER</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_fungible_asset_EOBJECT_IS_DELETABLE"></a>

Fungibility is only available for non&#45;deletable objects.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>: u64 &#61; 18;<br /></code></pre>



<a id="0x1_fungible_asset_ESTORE_IS_FROZEN"></a>

Store is disabled from sending and receiving this fungible asset.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_fungible_asset_ESUPPLY_NOT_FOUND"></a>

Supply resource is not found for a metadata object.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>: u64 &#61; 21;<br /></code></pre>



<a id="0x1_fungible_asset_ESUPPLY_UNDERFLOW"></a>

The fungible asset&apos;s supply will be negative which should be impossible.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>: u64 &#61; 20;<br /></code></pre>



<a id="0x1_fungible_asset_ESYMBOL_TOO_LONG"></a>

Symbol of the fungible asset metadata is too long


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESYMBOL_TOO_LONG">ESYMBOL_TOO_LONG</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

The transfer ref and the fungible asset do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH"></a>

Transfer ref and store do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH">ETRANSFER_REF_AND_STORE_MISMATCH</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_fungible_asset_EURI_TOO_LONG"></a>

URI for the icon of the fungible asset metadata is too long


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 &#61; 19;<br /></code></pre>



<a id="0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided withdraw function type doesn&apos;t meet the signature requirement.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH">EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH</a>: u64 &#61; 25;<br /></code></pre>



<a id="0x1_fungible_asset_MAX_DECIMALS"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_DECIMALS">MAX_DECIMALS</a>: u8 &#61; 32;<br /></code></pre>



<a id="0x1_fungible_asset_MAX_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_NAME_LENGTH">MAX_NAME_LENGTH</a>: u64 &#61; 32;<br /></code></pre>



<a id="0x1_fungible_asset_MAX_SYMBOL_LENGTH"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_SYMBOL_LENGTH">MAX_SYMBOL_LENGTH</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_fungible_asset_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 &#61; 512;<br /></code></pre>



<a id="0x1_fungible_asset_add_fungibility"></a>

## Function `add_fungibility`

Make an existing object fungible by adding the Metadata resource.
This returns the capabilities to mint, burn, and transfer.
maximum_supply defines the behavior of maximum supply when monitoring:
&#45; option::none(): Monitoring unlimited supply
(width of the field &#45; MAX_U128 is the implicit maximum supply)
if option::some(MAX_U128) is used, it is treated as unlimited supply.
&#45; option::some(max): Monitoring fixed supply with <code>max</code> as the maximum supply.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">add_fungibility</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, icon_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, project_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">add_fungibility</a>(<br />    constructor_ref: &amp;ConstructorRef,<br />    maximum_supply: Option&lt;u128&gt;,<br />    name: String,<br />    symbol: String,<br />    decimals: u8,<br />    icon_uri: String,<br />    project_uri: String,<br />): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; &#123;<br />    <b>assert</b>!(!<a href="object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(constructor_ref), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>));<br />    <b>let</b> metadata_object_signer &#61; &amp;<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="fungible_asset.md#0x1_fungible_asset_MAX_NAME_LENGTH">MAX_NAME_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENAME_TOO_LONG">ENAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;symbol) &lt;&#61; <a href="fungible_asset.md#0x1_fungible_asset_MAX_SYMBOL_LENGTH">MAX_SYMBOL_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESYMBOL_TOO_LONG">ESYMBOL_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a> &lt;&#61; <a href="fungible_asset.md#0x1_fungible_asset_MAX_DECIMALS">MAX_DECIMALS</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EDECIMALS_TOO_LARGE">EDECIMALS_TOO_LARGE</a>));<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;icon_uri) &lt;&#61; <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;project_uri) &lt;&#61; <a href="fungible_asset.md#0x1_fungible_asset_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>move_to</b>(metadata_object_signer,<br />        <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> &#123;<br />            name,<br />            symbol,<br />            decimals,<br />            icon_uri,<br />            project_uri,<br />        &#125;<br />    );<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_concurrent_fungible_assets_enabled">features::concurrent_fungible_assets_enabled</a>()) &#123;<br />        <b>let</b> unlimited &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&amp;maximum_supply);<br />        <b>move_to</b>(metadata_object_signer, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />            current: <b>if</b> (unlimited) &#123;<br />                <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>()<br />            &#125; <b>else</b> &#123;<br />                <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maximum_supply))<br />            &#125;,<br />        &#125;);<br />    &#125; <b>else</b> &#123;<br />        <b>move_to</b>(metadata_object_signer, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> &#123;<br />            current: 0,<br />            maximum: maximum_supply<br />        &#125;);<br />    &#125;;<br /><br />    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_set_untransferable"></a>

## Function `set_untransferable`

Set that only untransferable stores can be created for this fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_untransferable">set_untransferable</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_untransferable">set_untransferable</a>(constructor_ref: &amp;ConstructorRef) &#123;<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(metadata_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE">EFUNGIBLE_METADATA_EXISTENCE</a>));<br />    <b>let</b> metadata_signer &#61; &amp;<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);<br />    <b>move_to</b>(metadata_signer, <a href="fungible_asset.md#0x1_fungible_asset_Untransferable">Untransferable</a> &#123;&#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_is_untransferable"></a>

## Function `is_untransferable`

Returns true if the FA is untransferable.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_untransferable">is_untransferable</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_untransferable">is_untransferable</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): bool &#123;<br />    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Untransferable">Untransferable</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;metadata))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`

Create a fungible asset store whose transfer rule would be overloaded by the provided function.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, withdraw_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, deposit_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, derived_balance_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(<br />    constructor_ref: &amp;ConstructorRef,<br />    withdraw_function: Option&lt;FunctionInfo&gt;,<br />    deposit_function: Option&lt;FunctionInfo&gt;,<br />    derived_balance_function: Option&lt;FunctionInfo&gt;,<br />) &#123;<br />    // Verify that caller type matches callee type so wrongly typed function cannot be registered.<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&amp;withdraw_function, &#124;withdraw_function&#124; &#123;<br />        <b>let</b> dispatcher_withdraw_function_info &#61; <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(<br />            @aptos_framework,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>&quot;),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;dispatchable_withdraw&quot;),<br />        );<br /><br />        <b>assert</b>!(<br />            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(<br />                &amp;dispatcher_withdraw_function_info,<br />                withdraw_function<br />            ),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<br />                <a href="fungible_asset.md#0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH">EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH</a><br />            )<br />        );<br />    &#125;);<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&amp;deposit_function, &#124;deposit_function&#124; &#123;<br />        <b>let</b> dispatcher_deposit_function_info &#61; <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(<br />            @aptos_framework,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>&quot;),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;dispatchable_deposit&quot;),<br />        );<br />        // Verify that caller type matches callee type so wrongly typed function cannot be registered.<br />        <b>assert</b>!(<br />            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(<br />                &amp;dispatcher_deposit_function_info,<br />                deposit_function<br />            ),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<br />                <a href="fungible_asset.md#0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH">EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH</a><br />            )<br />        );<br />    &#125;);<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_for_each_ref">option::for_each_ref</a>(&amp;derived_balance_function, &#124;balance_function&#124; &#123;<br />        <b>let</b> dispatcher_derived_balance_function_info &#61; <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(<br />            @aptos_framework,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">dispatchable_fungible_asset</a>&quot;),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;dispatchable_derived_balance&quot;),<br />        );<br />        // Verify that caller type matches callee type so wrongly typed function cannot be registered.<br />        <b>assert</b>!(<br />            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(<br />                &amp;dispatcher_derived_balance_function_info,<br />                balance_function<br />            ),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<br />                <a href="fungible_asset.md#0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH">EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH</a><br />            )<br />        );<br />    &#125;);<br /><br />    // Cannot register hook for APT.<br />    <b>assert</b>!(<br />        <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref) !&#61; @aptos_fungible_asset,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAPT_NOT_DISPATCHABLE">EAPT_NOT_DISPATCHABLE</a>)<br />    );<br />    <b>assert</b>!(<br />        !<a href="object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(constructor_ref),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>)<br />    );<br />    <b>assert</b>!(<br />        !<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(<br />            <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref)<br />        ),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="fungible_asset.md#0x1_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>)<br />    );<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(<br />            <a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref)<br />        ),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE">EFUNGIBLE_METADATA_EXISTENCE</a>),<br />    );<br /><br />    <b>let</b> store_obj &#61; &amp;<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);<br /><br />    // Store the overload function hook.<br />    <b>move_to</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(<br />        store_obj,<br />        <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />            withdraw_function,<br />            deposit_function,<br />            derived_balance_function,<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_generate_mint_ref"></a>

## Function `generate_mint_ref`

Creates a mint ref that can be used to mint fungible assets from the given fungible object&apos;s constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_mint_ref">generate_mint_ref</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_mint_ref">generate_mint_ref</a>(constructor_ref: &amp;ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> &#123;<br />    <b>let</b> metadata &#61; <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);<br />    <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> &#123; metadata &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a burn ref that can be used to burn fungible assets from the given fungible object&apos;s constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_burn_ref">generate_burn_ref</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_burn_ref">generate_burn_ref</a>(constructor_ref: &amp;ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> &#123;<br />    <b>let</b> metadata &#61; <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);<br />    <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> &#123; metadata &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Creates a transfer ref that can be used to freeze/unfreeze/transfer fungible assets from the given fungible
object&apos;s constructor ref.
This can only be called at object creation time as constructor_ref is only available then.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">generate_transfer_ref</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">generate_transfer_ref</a>(constructor_ref: &amp;ConstructorRef): <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> &#123;<br />    <b>let</b> metadata &#61; <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(constructor_ref);<br />    <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> &#123; metadata &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_supply"></a>

## Function `supply`

Get the current supply from the <code>metadata</code> object.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> metadata_address &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;metadata);<br />    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&amp;supply.current))<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_maximum"></a>

## Function `maximum`

Get the maximum supply from the <code>metadata</code> object.
If supply is unlimited (or set explicitly to MAX_U128), none is returned


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> metadata_address &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;metadata);<br />    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);<br />        <b>let</b> max_value &#61; <a href="aggregator_v2.md#0x1_aggregator_v2_max_value">aggregator_v2::max_value</a>(&amp;supply.current);<br />        <b>if</b> (max_value &#61;&#61; <a href="fungible_asset.md#0x1_fungible_asset_MAX_U128">MAX_U128</a>) &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />        &#125; <b>else</b> &#123;<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(max_value)<br />        &#125;<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);<br />        supply.maximum<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_name"></a>

## Function `name`

Get the name of the fungible asset from the <code>metadata</code> object.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&amp;metadata).name<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_symbol"></a>

## Function `symbol`

Get the symbol of the fungible asset from the <code>metadata</code> object.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&amp;metadata).symbol<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_decimals"></a>

## Function `decimals`

Get the decimals from the <code>metadata</code> object.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): u8 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>(&amp;metadata).decimals<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_store_exists"></a>

## Function `store_exists`

Return whether the provided address has a store initialized.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(store: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(store: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_metadata_from_asset"></a>

## Function `metadata_from_asset`

Return the underlying metadata object


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa: &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa: &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; &#123;<br />    fa.metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_store_metadata"></a>

## Function `store_metadata`

Return the underlying metadata object.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store).metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_amount"></a>

## Function `amount`

Return the <code>amount</code> of a given fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): u64 &#123;<br />    fa.amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of a given store.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>&lt;T: key&gt;(store: Object&lt;T&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store))) &#123;<br />        <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store).balance<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_is_balance_at_least"></a>

## Function `is_balance_at_least`

Check whether the balance of a given store is &gt;&#61; <code>amount</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_balance_at_least">is_balance_at_least</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_balance_at_least">is_balance_at_least</a>&lt;T: key&gt;(store: Object&lt;T&gt;, amount: u64): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>let</b> store_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store);<br />    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(store_addr)) &#123;<br />        <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store).balance &gt;&#61; amount<br />    &#125; <b>else</b> &#123;<br />        amount &#61;&#61; 0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_is_frozen"></a>

## Function `is_frozen`

Return whether a store is frozen.

If the store has not been created, we default to returning false so deposits can be sent to it.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_frozen">is_frozen</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_frozen">is_frozen</a>&lt;T: key&gt;(store: Object&lt;T&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_store_exists">store_exists</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store)) &amp;&amp; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store).frozen<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_is_store_dispatchable"></a>

## Function `is_store_dispatchable`

Return whether a fungible asset type is dispatchable.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_store_dispatchable">is_store_dispatchable</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_is_store_dispatchable">is_store_dispatchable</a>&lt;T: key&gt;(store: Object&lt;T&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>let</b> fa_store &#61; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store);<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;fa_store.metadata);<br />    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_dispatch_function"></a>

## Function `deposit_dispatch_function`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">deposit_dispatch_function</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">deposit_dispatch_function</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> fa_store &#61; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store);<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;fa_store.metadata);<br />    <b>if</b>(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) &#123;<br />        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).deposit_function<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_has_deposit_dispatch_function"></a>

## Function `has_deposit_dispatch_function`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_deposit_dispatch_function">has_deposit_dispatch_function</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_deposit_dispatch_function">has_deposit_dispatch_function</a>(metadata: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;metadata);<br />    // Short circuit on APT for better perf<br />    <b>if</b>(metadata_addr !&#61; @aptos_fungible_asset &amp;&amp; <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).deposit_function)<br />    &#125; <b>else</b> &#123;<br />        <b>false</b><br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_dispatch_function"></a>

## Function `withdraw_dispatch_function`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">withdraw_dispatch_function</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">withdraw_dispatch_function</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> fa_store &#61; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store);<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;fa_store.metadata);<br />    <b>if</b>(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) &#123;<br />        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).withdraw_function<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_has_withdraw_dispatch_function"></a>

## Function `has_withdraw_dispatch_function`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_withdraw_dispatch_function">has_withdraw_dispatch_function</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_has_withdraw_dispatch_function">has_withdraw_dispatch_function</a>(metadata: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;metadata);<br />    // Short circuit on APT for better perf<br />    <b>if</b> (metadata_addr !&#61; @aptos_fungible_asset &amp;&amp; <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).withdraw_function)<br />    &#125; <b>else</b> &#123;<br />        <b>false</b><br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_derived_balance_dispatch_function"></a>

## Function `derived_balance_dispatch_function`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">derived_balance_dispatch_function</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">derived_balance_dispatch_function</a>&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> fa_store &#61; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store);<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;fa_store.metadata);<br />    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr)) &#123;<br />        <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a>&gt;(metadata_addr).derived_balance_function<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_asset_metadata"></a>

## Function `asset_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">asset_metadata</a>(fa: &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">asset_metadata</a>(fa: &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; &#123;<br />    fa.metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_mint_ref_metadata"></a>

## Function `mint_ref_metadata`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">mint_ref_metadata</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">mint_ref_metadata</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; &#123;<br />    ref.metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_transfer_ref_metadata"></a>

## Function `transfer_ref_metadata`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">transfer_ref_metadata</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">transfer_ref_metadata</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; &#123;<br />    ref.metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_burn_ref_metadata"></a>

## Function `burn_ref_metadata`

Get the underlying metadata object from the <code><a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">burn_ref_metadata</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">burn_ref_metadata</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt; &#123;<br />    ref.metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(sender: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(<br />    sender: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    from: Object&lt;T&gt;,<br />    <b>to</b>: Object&lt;T&gt;,<br />    amount: u64,<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> fa &#61; <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>(sender, from, amount);<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_create_store"></a>

## Function `create_store`

Allow an object to hold a store for fungible assets.
Applications can use this to create multiple stores for isolating fungible assets for different purposes.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_create_store">create_store</a>&lt;T: key&gt;(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_create_store">create_store</a>&lt;T: key&gt;(<br />    constructor_ref: &amp;ConstructorRef,<br />    metadata: Object&lt;T&gt;,<br />): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt; &#123;<br />    <b>let</b> store_obj &#61; &amp;<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);<br />    <b>move_to</b>(store_obj, <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />        metadata: <a href="object.md#0x1_object_convert">object::convert</a>(metadata),<br />        balance: 0,<br />        frozen: <b>false</b>,<br />    &#125;);<br />    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_is_untransferable">is_untransferable</a>(metadata)) &#123;<br />        <a href="object.md#0x1_object_set_untransferable">object::set_untransferable</a>(constructor_ref);<br />    &#125;;<br />    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(constructor_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_remove_store"></a>

## Function `remove_store`

Used to delete a store.  Requires the store to be completely empty prior to removing it


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_remove_store">remove_store</a>(delete_ref: &amp;<a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_remove_store">remove_store</a>(delete_ref: &amp;DeleteRef) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a> &#123;<br />    <b>let</b> store &#61; &amp;<a href="object.md#0x1_object_object_from_delete_ref">object::object_from_delete_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(delete_ref);<br />    <b>let</b> addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(store);<br />    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123; metadata: _, balance, frozen: _ &#125;<br />        &#61; <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(addr);<br />    <b>assert</b>!(balance &#61;&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBALANCE_IS_NOT_ZERO">EBALANCE_IS_NOT_ZERO</a>));<br />    // Cleanup deprecated <a href="event.md#0x1_event">event</a> handles <b>if</b> exist.<br />    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a>&gt;(addr)) &#123;<br />        <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a> &#123;<br />            deposit_events,<br />            withdraw_events,<br />            frozen_events,<br />        &#125; &#61; <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetEvents">FungibleAssetEvents</a>&gt;(addr);<br />        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(deposit_events);<br />        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(withdraw_events);<br />        <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(frozen_events);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    store: Object&lt;T&gt;,<br />    amount: u64,<br />): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">withdraw_sanity_check</a>(owner, store, <b>true</b>);<br />    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store), amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_sanity_check"></a>

## Function `withdraw_sanity_check`

Check the permission for withdraw operation.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">withdraw_sanity_check</a>&lt;T: key&gt;(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, abort_on_dispatch: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">withdraw_sanity_check</a>&lt;T: key&gt;(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    store: Object&lt;T&gt;,<br />    abort_on_dispatch: bool,<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>assert</b>!(<a href="object.md#0x1_object_owns">object::owns</a>(store, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENOT_STORE_OWNER">ENOT_STORE_OWNER</a>));<br />    <b>let</b> fa_store &#61; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store);<br />    <b>assert</b>!(<br />        !abort_on_dispatch &#124;&#124; !<a href="fungible_asset.md#0x1_fungible_asset_has_withdraw_dispatch_function">has_withdraw_dispatch_function</a>(fa_store.metadata),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS">EINVALID_DISPATCHABLE_OPERATIONS</a>)<br />    );<br />    <b>assert</b>!(!fa_store.frozen, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_sanity_check"></a>

## Function `deposit_sanity_check`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, abort_on_dispatch: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>&lt;T: key&gt;(<br />    store: Object&lt;T&gt;,<br />    abort_on_dispatch: bool<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <b>let</b> fa_store &#61; <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>(&amp;store);<br />    <b>assert</b>!(<br />        !abort_on_dispatch &#124;&#124; !<a href="fungible_asset.md#0x1_fungible_asset_has_deposit_dispatch_function">has_deposit_dispatch_function</a>(fa_store.metadata),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS">EINVALID_DISPATCHABLE_OPERATIONS</a>)<br />    );<br />    <b>assert</b>!(!fa_store.frozen, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESTORE_IS_FROZEN">ESTORE_IS_FROZEN</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: Object&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>(store, <b>true</b>);<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(store, fa);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint the specified <code>amount</code> of the fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> metadata &#61; ref.metadata;<br />    <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">mint_internal</a>(metadata, amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_mint_internal"></a>

## Function `mint_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">mint_internal</a>(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_internal">mint_internal</a>(<br />    metadata: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;,<br />    amount: u64<br />): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>(&amp;metadata, amount);<br />    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123;<br />        metadata,<br />        amount<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_mint_to"></a>

## Function `mint_to`

Mint the specified <code>amount</code> of the fungible asset to a destination store.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_to">mint_to</a>&lt;T: key&gt;(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_to">mint_to</a>&lt;T: key&gt;(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>, store: Object&lt;T&gt;, amount: u64)<br /><b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>, <a href="fungible_asset.md#0x1_fungible_asset_DispatchFunctionStore">DispatchFunctionStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">deposit_sanity_check</a>(store, <b>false</b>);<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(store, <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref, amount));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag"></a>

## Function `set_frozen_flag`

Enable/disable a store&apos;s ability to do direct transfers of the fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag">set_frozen_flag</a>&lt;T: key&gt;(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, frozen: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag">set_frozen_flag</a>&lt;T: key&gt;(<br />    ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,<br />    store: Object&lt;T&gt;,<br />    frozen: bool,<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>assert</b>!(<br />        ref.metadata &#61;&#61; <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>(store),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH">ETRANSFER_REF_AND_STORE_MISMATCH</a>),<br />    );<br />    <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">set_frozen_flag_internal</a>(store, frozen)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag_internal"></a>

## Function `set_frozen_flag_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">set_frozen_flag_internal</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, frozen: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag_internal">set_frozen_flag_internal</a>&lt;T: key&gt;(<br />    store: Object&lt;T&gt;,<br />    frozen: bool<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>let</b> store_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store);<br />    <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr).frozen &#61; frozen;<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="fungible_asset.md#0x1_fungible_asset_Frozen">Frozen</a> &#123; store: store_addr, frozen &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_burn"></a>

## Function `burn`

Burns a fungible asset


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>assert</b>!(<br />        ref.metadata &#61;&#61; <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(&amp;fa),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH">EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>)<br />    );<br />    <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">burn_internal</a>(fa);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_burn_internal"></a>

## Function `burn_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">burn_internal</a>(fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_internal">burn_internal</a>(<br />    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a><br />): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123;<br />        metadata,<br />        amount<br />    &#125; &#61; fa;<br />    <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>(&amp;metadata, amount);<br />    amount<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_burn_from"></a>

## Function `burn_from`

Burn the <code>amount</code> of the fungible asset from the given store.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_from">burn_from</a>&lt;T: key&gt;(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_from">burn_from</a>&lt;T: key&gt;(<br />    ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>,<br />    store: Object&lt;T&gt;,<br />    amount: u64<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>, <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> metadata &#61; ref.metadata;<br />    <b>assert</b>!(metadata &#61;&#61; <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>(store), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH">EBURN_REF_AND_STORE_MISMATCH</a>));<br />    <b>let</b> store_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store);<br />    <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref, <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(store_addr, amount));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw <code>amount</code> of the fungible asset from the <code>store</code> ignoring <code>frozen</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>&lt;T: key&gt;(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>&lt;T: key&gt;(<br />    ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,<br />    store: Object&lt;T&gt;,<br />    amount: u64<br />): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>assert</b>!(<br />        ref.metadata &#61;&#61; <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>(store),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH">ETRANSFER_REF_AND_STORE_MISMATCH</a>),<br />    );<br />    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store), amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit the fungible asset into the <code>store</code> ignoring <code>frozen</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>&lt;T: key&gt;(ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>&lt;T: key&gt;(<br />    ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,<br />    store: Object&lt;T&gt;,<br />    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a><br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>assert</b>!(<br />        ref.metadata &#61;&#61; fa.metadata,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>)<br />    );<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(store, fa);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>amount</code> of the fungible asset with <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code> even it is frozen.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>&lt;T: key&gt;(transfer_ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>&lt;T: key&gt;(<br />    transfer_ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,<br />    from: Object&lt;T&gt;,<br />    <b>to</b>: Object&lt;T&gt;,<br />    amount: u64,<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>let</b> fa &#61; <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>(transfer_ref, from, amount);<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>(transfer_ref, <b>to</b>, fa);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_zero"></a>

## Function `zero`

Create a fungible asset with zero amount.
This can be useful when starting a series of computations where the initial value is 0.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_zero">zero</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_zero">zero</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123;<br />        metadata: <a href="object.md#0x1_object_convert">object::convert</a>(metadata),<br />        amount: 0,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract a given amount from the given fungible asset and return a new one.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: &amp;<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: &amp;<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123;<br />    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount &gt;&#61; amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));<br />    <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount &#61; <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount &#45; amount;<br />    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123;<br />        metadata: <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.metadata,<br />        amount,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_merge"></a>

## Function `merge`

&quot;Merges&quot; the two given fungible assets. The fungible asset passed in as <code>dst_fungible_asset</code> will have a value
equal to the sum of the two (<code>dst_fungible_asset</code> and <code>src_fungible_asset</code>).


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(dst_fungible_asset: &amp;<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, src_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(dst_fungible_asset: &amp;<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>, src_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) &#123;<br />    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123; metadata, amount &#125; &#61; src_fungible_asset;<br />    <b>assert</b>!(metadata &#61;&#61; dst_fungible_asset.metadata, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH">EFUNGIBLE_ASSET_MISMATCH</a>));<br />    dst_fungible_asset.amount &#61; dst_fungible_asset.amount &#43; amount;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_destroy_zero"></a>

## Function `destroy_zero`

Destroy an empty fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destroy_zero">destroy_zero</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destroy_zero">destroy_zero</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) &#123;<br />    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123; amount, metadata: _ &#125; &#61; <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>;<br />    <b>assert</b>!(amount &#61;&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO">EAMOUNT_IS_NOT_ZERO</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_internal"></a>

## Function `deposit_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>&lt;T: key&gt;(store: Object&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123; metadata, amount &#125; &#61; fa;<br />    <b>if</b> (amount &#61;&#61; 0) <b>return</b>;<br /><br />    <b>let</b> store_metadata &#61; <a href="fungible_asset.md#0x1_fungible_asset_store_metadata">store_metadata</a>(store);<br />    <b>assert</b>!(metadata &#61;&#61; store_metadata, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH">EFUNGIBLE_ASSET_AND_STORE_MISMATCH</a>));<br />    <b>let</b> store_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store);<br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr);<br />    store.balance &#61; store.balance &#43; amount;<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="fungible_asset.md#0x1_fungible_asset_Deposit">Deposit</a> &#123; store: store_addr, amount &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_internal"></a>

## Function `withdraw_internal`

Extract <code>amount</code> of the fungible asset from <code>store</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(store_addr: <b>address</b>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<br />    store_addr: <b>address</b>,<br />    amount: u64,<br />): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>));<br />    <b>let</b> store &#61; <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr);<br />    <b>let</b> metadata &#61; store.metadata;<br />    <b>if</b> (amount !&#61; 0) &#123;<br />        <b>assert</b>!(store.balance &gt;&#61; amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));<br />        store.balance &#61; store.balance &#45; amount;<br />        <a href="event.md#0x1_event_emit">event::emit</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Withdraw">Withdraw</a>&gt;(<a href="fungible_asset.md#0x1_fungible_asset_Withdraw">Withdraw</a> &#123; store: store_addr, amount &#125;);<br />    &#125;;<br />    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> &#123; metadata, amount &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_increase_supply"></a>

## Function `increase_supply`

Increase the supply of a fungible asset by minting.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>&lt;T: key&gt;(metadata: &amp;<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>&lt;T: key&gt;(metadata: &amp;Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <b>return</b><br />    &#125;;<br />    <b>let</b> metadata_address &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);<br />        <b>assert</b>!(<br />            <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">aggregator_v2::try_add</a>(&amp;<b>mut</b> supply.current, (amount <b>as</b> u128)),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>)<br />        );<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);<br />        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;supply.maximum)) &#123;<br />            <b>let</b> max &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> supply.maximum);<br />            <b>assert</b>!(<br />                max &#45; supply.current &gt;&#61; (amount <b>as</b> u128),<br />                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>)<br />            )<br />        &#125;;<br />        supply.current &#61; supply.current &#43; (amount <b>as</b> u128);<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_decrease_supply"></a>

## Function `decrease_supply`

Decrease the supply of a fungible asset by burning.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>&lt;T: key&gt;(metadata: &amp;<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>&lt;T: key&gt;(metadata: &amp;Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>, <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>if</b> (amount &#61;&#61; 0) &#123;<br />        <b>return</b><br />    &#125;;<br />    <b>let</b> metadata_address &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a>&gt;(metadata_address);<br /><br />        <b>assert</b>!(<br />            <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">aggregator_v2::try_sub</a>(&amp;<b>mut</b> supply.current, (amount <b>as</b> u128)),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>)<br />        );<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address)) &#123;<br />        <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>));<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_address);<br />        <b>assert</b>!(<br />            supply.current &gt;&#61; (amount <b>as</b> u128),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>)<br />        );<br />        supply.current &#61; supply.current &#45; (amount <b>as</b> u128);<br />    &#125; <b>else</b> &#123;<br />        <b>assert</b>!(<b>false</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>));<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata"></a>

## Function `borrow_fungible_metadata`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>&lt;T: key&gt;(metadata: &amp;<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata">borrow_fungible_metadata</a>&lt;T: key&gt;(<br />    metadata: &amp;Object&lt;T&gt;<br />): &amp;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> &#123;<br />    <b>let</b> addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);<br />    <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata_mut"></a>

## Function `borrow_fungible_metadata_mut`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata_mut">borrow_fungible_metadata_mut</a>&lt;T: key&gt;(metadata: &amp;<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_fungible_metadata_mut">borrow_fungible_metadata_mut</a>&lt;T: key&gt;(<br />    metadata: &amp;Object&lt;T&gt;<br />): &amp;<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a> &#123;<br />    <b>let</b> addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);<br />    <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">Metadata</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_borrow_store_resource"></a>

## Function `borrow_store_resource`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>&lt;T: key&gt;(store: &amp;<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_borrow_store_resource">borrow_store_resource</a>&lt;T: key&gt;(store: &amp;Object&lt;T&gt;): &amp;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a> &#123;<br />    <b>let</b> store_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(store);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE">EFUNGIBLE_STORE_EXISTENCE</a>));<br />    <b>borrow_global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">FungibleStore</a>&gt;(store_addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fungible_asset_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_upgrade_to_concurrent">upgrade_to_concurrent</a>(ref: &amp;<a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_upgrade_to_concurrent">upgrade_to_concurrent</a>(<br />    ref: &amp;ExtendRef,<br />) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> &#123;<br />    <b>let</b> metadata_object_address &#61; <a href="object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(ref);<br />    <b>let</b> metadata_object_signer &#61; <a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(ref);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_concurrent_fungible_assets_enabled">features::concurrent_fungible_assets_enabled</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED">ECONCURRENT_SUPPLY_NOT_ENABLED</a>)<br />    );<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_object_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_NOT_FOUND">ESUPPLY_NOT_FOUND</a>));<br />    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a> &#123;<br />        current,<br />        maximum,<br />    &#125; &#61; <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Supply">Supply</a>&gt;(metadata_object_address);<br /><br />    <b>let</b> unlimited &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&amp;maximum);<br />    <b>let</b> supply &#61; <a href="fungible_asset.md#0x1_fungible_asset_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />        current: <b>if</b> (unlimited) &#123;<br />            <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>()<br />        &#125;<br />        <b>else</b> &#123;<br />            <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maximum))<br />        &#125;,<br />    &#125;;<br />    // <b>update</b> current state:<br />    <a href="aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&amp;<b>mut</b> supply.current, current);<br />    <b>move_to</b>(&amp;metadata_object_signer, supply);<br />&#125;<br /></code></pre>



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
<td>The withdraw function ensures that the store is not frozen before calling withdraw_internal which ensures that the withdrawing amount is greater than 0 and less than the total balance from the store. The withdraw_with_ref ensures that the reference&apos;s metadata matches the store metadata.</td>
<td>Audited that it aborts if the withdrawing store is frozen. Audited that it aborts if the store doesn&apos;t have sufficient balance. Audited that the balance of the withdrawing store is reduced by amount.</td>
</tr>

<tr>
<td>7</td>
<td>Only the same type of fungible assets should be deposited in a fungible asset store, if the store is not frozen, unless the deposit is performed with a reference, and afterwards the store balance should be increased.</td>
<td>High</td>
<td>The deposit function ensures that store is not frozen and proceeds to call the deposit_internal function which validates the store&apos;s metadata and the depositing asset&apos;s metadata followed by increasing the store balance by the given amount. The deposit_with_ref ensures that the reference&apos;s metadata matches the depositing asset&apos;s metadata.</td>
<td>Audited that it aborts if the store is frozen. Audited that it aborts if the asset and asset store are different. Audited that the store&apos;s balance is increased by the deposited amount.</td>
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
<td>The remove_store function validates the store&apos;s balance and removes the store under the object address.</td>
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
<td>The burn process ensures that the store has enough balance to burn, by asserting that the supply.current &gt;&#61; amount inside the decrease_supply function.</td>
<td>Audited that it aborts if the provided store doesn&apos;t have sufficient balance.</td>
</tr>

<tr>
<td>13</td>
<td>Enabling or disabling store&apos;s frozen status should only be done with a valid transfer reference.</td>
<td>High</td>
<td>The set_frozen_flag function ensures that the TransferRef is provided via function argument and that the store&apos;s metadata matches the metadata from the reference. It then proceeds to update the frozen flag of the store.</td>
<td>Audited that it aborts if the metadata doesn&apos;t match. Audited that the frozen flag is updated properly.</td>
</tr>

<tr>
<td>14</td>
<td>Extracting a specific amount from the fungible asset should be possible only if the total amount that it holds is greater or equal to the provided amount.</td>
<td>High</td>
<td>The extract function validates that the fungible asset has enough balance to extract and then updates it by subtracting the extracted amount.</td>
<td>Audited that it aborts if the asset didn&apos;t have sufficient balance. Audited that the balance of the asset is updated. Audited that the extract function returns the extracted asset.</td>
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


<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
