
<a id="0x1_fungible_asset"></a>

# Module `0x1::fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code>Metadata</code> object. The<br/> metadata object can be any object that equipped with <code>Metadata</code> resource.


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


<pre><code>use 0x1::aggregator_v2;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::function_info;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/></code></pre>



<a id="0x1_fungible_asset_Supply"></a>

## Resource `Supply`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Supply has key<br/></code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct ConcurrentSupply has key<br/></code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Metadata has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 Name of the fungible metadata, i.e., &quot;USDT&quot;.
</dd>
<dt>
<code>symbol: string::String</code>
</dt>
<dd>
 Symbol of the fungible metadata, usually a shorter version of the name.<br/> For example, Singapore Dollar is SGD.
</dd>
<dt>
<code>decimals: u8</code>
</dt>
<dd>
 Number of decimals used for display purposes.<br/> For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should<br/> be displayed to a user as <code>5.05</code> (<code>505 / 10 &#42;&#42; 2</code>).
</dd>
<dt>
<code>icon_uri: string::String</code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to an image that can be used as the icon for this fungible<br/> asset.
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

Defines a <code>FungibleAsset</code>, such that all <code>FungibleStore</code>s stores are untransferable at<br/> the object layer.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Untransferable has key<br/></code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct FungibleStore has key<br/></code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct DispatchFunctionStore has key<br/></code></pre>



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

FungibleAsset can be passed into function for type safety and to guarantee a specific amount.<br/> FungibleAsset is ephemeral and cannot be stored directly. It must be deposited back into a store.


<pre><code>struct FungibleAsset<br/></code></pre>



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

MintRef can be used to mint the fungible asset into an account&apos;s store.


<pre><code>struct MintRef has drop, store<br/></code></pre>



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

TransferRef can be used to allow or disallow the owner of fungible assets from transferring the asset<br/> and allow the holder of TransferRef to transfer fungible assets from any account.


<pre><code>struct TransferRef has drop, store<br/></code></pre>



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


<pre><code>struct BurnRef has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct Deposit has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct Withdraw has drop, store<br/></code></pre>



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

Emitted when a store&apos;s frozen status is updated.


<pre><code>&#35;[event]<br/>struct Frozen has drop, store<br/></code></pre>



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



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>&#35;[deprecated]<br/>struct FungibleAssetEvents has key<br/></code></pre>



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



<pre><code>&#35;[deprecated]<br/>struct DepositEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[deprecated]<br/>struct WithdrawEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[deprecated]<br/>struct FrozenEvent has drop, store<br/></code></pre>



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


<pre><code>const MAX_U128: u128 &#61; 340282366920938463463374607431768211455;<br/></code></pre>



<a id="0x1_fungible_asset_EALREADY_REGISTERED"></a>

Trying to re&#45;register dispatch hook on a fungible asset.


<pre><code>const EALREADY_REGISTERED: u64 &#61; 29;<br/></code></pre>



<a id="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code>const EAMOUNT_CANNOT_BE_ZERO: u64 &#61; 1;<br/></code></pre>



<a id="0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO"></a>

Cannot destroy non&#45;empty fungible assets.


<pre><code>const EAMOUNT_IS_NOT_ZERO: u64 &#61; 12;<br/></code></pre>



<a id="0x1_fungible_asset_EAPT_NOT_DISPATCHABLE"></a>

Cannot register dispatch hook for APT.


<pre><code>const EAPT_NOT_DISPATCHABLE: u64 &#61; 31;<br/></code></pre>



<a id="0x1_fungible_asset_EBALANCE_IS_NOT_ZERO"></a>

Cannot destroy fungible stores with a non&#45;zero balance.


<pre><code>const EBALANCE_IS_NOT_ZERO: u64 &#61; 14;<br/></code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

Burn ref and fungible asset do not match.


<pre><code>const EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH: u64 &#61; 13;<br/></code></pre>



<a id="0x1_fungible_asset_EBURN_REF_AND_STORE_MISMATCH"></a>

Burn ref and store do not match.


<pre><code>const EBURN_REF_AND_STORE_MISMATCH: u64 &#61; 10;<br/></code></pre>



<a id="0x1_fungible_asset_ECONCURRENT_SUPPLY_NOT_ENABLED"></a>

Flag for Concurrent Supply not enabled


<pre><code>const ECONCURRENT_SUPPLY_NOT_ENABLED: u64 &#61; 22;<br/></code></pre>



<a id="0x1_fungible_asset_EDECIMALS_TOO_LARGE"></a>

Decimals is over the maximum of 32


<pre><code>const EDECIMALS_TOO_LARGE: u64 &#61; 17;<br/></code></pre>



<a id="0x1_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided deposit function type doesn&apos;t meet the signature requirement.


<pre><code>const EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH: u64 &#61; 26;<br/></code></pre>



<a id="0x1_fungible_asset_EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided derived_balance function type doesn&apos;t meet the signature requirement.


<pre><code>const EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH: u64 &#61; 27;<br/></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_AND_STORE_MISMATCH"></a>

Fungible asset and store do not match.


<pre><code>const EFUNGIBLE_ASSET_AND_STORE_MISMATCH: u64 &#61; 11;<br/></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_ASSET_MISMATCH"></a>

Fungible asset do not match when merging.


<pre><code>const EFUNGIBLE_ASSET_MISMATCH: u64 &#61; 6;<br/></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_METADATA_EXISTENCE"></a>

Fungible metadata does not exist on this account.


<pre><code>const EFUNGIBLE_METADATA_EXISTENCE: u64 &#61; 30;<br/></code></pre>



<a id="0x1_fungible_asset_EFUNGIBLE_STORE_EXISTENCE"></a>

Flag for the existence of fungible store.


<pre><code>const EFUNGIBLE_STORE_EXISTENCE: u64 &#61; 23;<br/></code></pre>



<a id="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient balance to withdraw or transfer.


<pre><code>const EINSUFFICIENT_BALANCE: u64 &#61; 4;<br/></code></pre>



<a id="0x1_fungible_asset_EINVALID_DISPATCHABLE_OPERATIONS"></a>

Invalid withdraw/deposit on dispatchable token. The specified token has a dispatchable function hook.<br/> Need to invoke dispatchable_fungible_asset::withdraw/deposit to perform transfer.


<pre><code>const EINVALID_DISPATCHABLE_OPERATIONS: u64 &#61; 28;<br/></code></pre>



<a id="0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED"></a>

The fungible asset&apos;s supply has exceeded maximum.


<pre><code>const EMAX_SUPPLY_EXCEEDED: u64 &#61; 5;<br/></code></pre>



<a id="0x1_fungible_asset_EMINT_REF_AND_STORE_MISMATCH"></a>

The mint ref and the store do not match.


<pre><code>const EMINT_REF_AND_STORE_MISMATCH: u64 &#61; 7;<br/></code></pre>



<a id="0x1_fungible_asset_ENAME_TOO_LONG"></a>

Name of the fungible asset metadata is too long


<pre><code>const ENAME_TOO_LONG: u64 &#61; 15;<br/></code></pre>



<a id="0x1_fungible_asset_ENOT_METADATA_OWNER"></a>

Account is not the owner of metadata object.


<pre><code>const ENOT_METADATA_OWNER: u64 &#61; 24;<br/></code></pre>



<a id="0x1_fungible_asset_ENOT_STORE_OWNER"></a>

Account is not the store&apos;s owner.


<pre><code>const ENOT_STORE_OWNER: u64 &#61; 8;<br/></code></pre>



<a id="0x1_fungible_asset_EOBJECT_IS_DELETABLE"></a>

Fungibility is only available for non&#45;deletable objects.


<pre><code>const EOBJECT_IS_DELETABLE: u64 &#61; 18;<br/></code></pre>



<a id="0x1_fungible_asset_ESTORE_IS_FROZEN"></a>

Store is disabled from sending and receiving this fungible asset.


<pre><code>const ESTORE_IS_FROZEN: u64 &#61; 3;<br/></code></pre>



<a id="0x1_fungible_asset_ESUPPLY_NOT_FOUND"></a>

Supply resource is not found for a metadata object.


<pre><code>const ESUPPLY_NOT_FOUND: u64 &#61; 21;<br/></code></pre>



<a id="0x1_fungible_asset_ESUPPLY_UNDERFLOW"></a>

The fungible asset&apos;s supply will be negative which should be impossible.


<pre><code>const ESUPPLY_UNDERFLOW: u64 &#61; 20;<br/></code></pre>



<a id="0x1_fungible_asset_ESYMBOL_TOO_LONG"></a>

Symbol of the fungible asset metadata is too long


<pre><code>const ESYMBOL_TOO_LONG: u64 &#61; 16;<br/></code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

The transfer ref and the fungible asset do not match.


<pre><code>const ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH: u64 &#61; 2;<br/></code></pre>



<a id="0x1_fungible_asset_ETRANSFER_REF_AND_STORE_MISMATCH"></a>

Transfer ref and store do not match.


<pre><code>const ETRANSFER_REF_AND_STORE_MISMATCH: u64 &#61; 9;<br/></code></pre>



<a id="0x1_fungible_asset_EURI_TOO_LONG"></a>

URI for the icon of the fungible asset metadata is too long


<pre><code>const EURI_TOO_LONG: u64 &#61; 19;<br/></code></pre>



<a id="0x1_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided withdraw function type doesn&apos;t meet the signature requirement.


<pre><code>const EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH: u64 &#61; 25;<br/></code></pre>



<a id="0x1_fungible_asset_MAX_DECIMALS"></a>



<pre><code>const MAX_DECIMALS: u8 &#61; 32;<br/></code></pre>



<a id="0x1_fungible_asset_MAX_NAME_LENGTH"></a>



<pre><code>const MAX_NAME_LENGTH: u64 &#61; 32;<br/></code></pre>



<a id="0x1_fungible_asset_MAX_SYMBOL_LENGTH"></a>



<pre><code>const MAX_SYMBOL_LENGTH: u64 &#61; 10;<br/></code></pre>



<a id="0x1_fungible_asset_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;<br/></code></pre>



<a id="0x1_fungible_asset_add_fungibility"></a>

## Function `add_fungibility`

Make an existing object fungible by adding the Metadata resource.<br/> This returns the capabilities to mint, burn, and transfer.<br/> maximum_supply defines the behavior of maximum supply when monitoring:<br/>   &#45; option::none(): Monitoring unlimited supply
(width of the field &#45; MAX_U128 is the implicit maximum supply)<br/>     if option::some(MAX_U128) is used, it is treated as unlimited supply.<br/>   &#45; option::some(max): Monitoring fixed supply with <code>max</code> as the maximum supply.


<pre><code>public fun add_fungibility(constructor_ref: &amp;object::ConstructorRef, maximum_supply: option::Option&lt;u128&gt;, name: string::String, symbol: string::String, decimals: u8, icon_uri: string::String, project_uri: string::String): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_fungibility(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    maximum_supply: Option&lt;u128&gt;,<br/>    name: String,<br/>    symbol: String,<br/>    decimals: u8,<br/>    icon_uri: String,<br/>    project_uri: String,<br/>): Object&lt;Metadata&gt; &#123;<br/>    assert!(!object::can_generate_delete_ref(constructor_ref), error::invalid_argument(EOBJECT_IS_DELETABLE));<br/>    let metadata_object_signer &#61; &amp;object::generate_signer(constructor_ref);<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_NAME_LENGTH, error::out_of_range(ENAME_TOO_LONG));<br/>    assert!(string::length(&amp;symbol) &lt;&#61; MAX_SYMBOL_LENGTH, error::out_of_range(ESYMBOL_TOO_LONG));<br/>    assert!(decimals &lt;&#61; MAX_DECIMALS, error::out_of_range(EDECIMALS_TOO_LARGE));<br/>    assert!(string::length(&amp;icon_uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));<br/>    assert!(string::length(&amp;project_uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));<br/>    move_to(metadata_object_signer,<br/>        Metadata &#123;<br/>            name,<br/>            symbol,<br/>            decimals,<br/>            icon_uri,<br/>            project_uri,<br/>        &#125;<br/>    );<br/><br/>    if (features::concurrent_fungible_assets_enabled()) &#123;<br/>        let unlimited &#61; option::is_none(&amp;maximum_supply);<br/>        move_to(metadata_object_signer, ConcurrentSupply &#123;<br/>            current: if (unlimited) &#123;<br/>                aggregator_v2::create_unbounded_aggregator()<br/>            &#125; else &#123;<br/>                aggregator_v2::create_aggregator(option::extract(&amp;mut maximum_supply))<br/>            &#125;,<br/>        &#125;);<br/>    &#125; else &#123;<br/>        move_to(metadata_object_signer, Supply &#123;<br/>            current: 0,<br/>            maximum: maximum_supply<br/>        &#125;);<br/>    &#125;;<br/><br/>    object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_set_untransferable"></a>

## Function `set_untransferable`

Set that only untransferable stores can be created for this fungible asset.


<pre><code>public fun set_untransferable(constructor_ref: &amp;object::ConstructorRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_untransferable(constructor_ref: &amp;ConstructorRef) &#123;<br/>    let metadata_addr &#61; object::address_from_constructor_ref(constructor_ref);<br/>    assert!(exists&lt;Metadata&gt;(metadata_addr), error::not_found(EFUNGIBLE_METADATA_EXISTENCE));<br/>    let metadata_signer &#61; &amp;object::generate_signer(constructor_ref);<br/>    move_to(metadata_signer, Untransferable &#123;&#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_is_untransferable"></a>

## Function `is_untransferable`

Returns true if the FA is untransferable.


<pre><code>&#35;[view]<br/>public fun is_untransferable&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_untransferable&lt;T: key&gt;(metadata: Object&lt;T&gt;): bool &#123;<br/>    exists&lt;Untransferable&gt;(object::object_address(&amp;metadata))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`

Create a fungible asset store whose transfer rule would be overloaded by the provided function.


<pre><code>public(friend) fun register_dispatch_functions(constructor_ref: &amp;object::ConstructorRef, withdraw_function: option::Option&lt;function_info::FunctionInfo&gt;, deposit_function: option::Option&lt;function_info::FunctionInfo&gt;, derived_balance_function: option::Option&lt;function_info::FunctionInfo&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun register_dispatch_functions(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    withdraw_function: Option&lt;FunctionInfo&gt;,<br/>    deposit_function: Option&lt;FunctionInfo&gt;,<br/>    derived_balance_function: Option&lt;FunctionInfo&gt;,<br/>) &#123;<br/>    // Verify that caller type matches callee type so wrongly typed function cannot be registered.<br/>    option::for_each_ref(&amp;withdraw_function, &#124;withdraw_function&#124; &#123;<br/>        let dispatcher_withdraw_function_info &#61; function_info::new_function_info_from_address(<br/>            @aptos_framework,<br/>            string::utf8(b&quot;dispatchable_fungible_asset&quot;),<br/>            string::utf8(b&quot;dispatchable_withdraw&quot;),<br/>        );<br/><br/>        assert!(<br/>            function_info::check_dispatch_type_compatibility(<br/>                &amp;dispatcher_withdraw_function_info,<br/>                withdraw_function<br/>            ),<br/>            error::invalid_argument(<br/>                EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH<br/>            )<br/>        );<br/>    &#125;);<br/><br/>    option::for_each_ref(&amp;deposit_function, &#124;deposit_function&#124; &#123;<br/>        let dispatcher_deposit_function_info &#61; function_info::new_function_info_from_address(<br/>            @aptos_framework,<br/>            string::utf8(b&quot;dispatchable_fungible_asset&quot;),<br/>            string::utf8(b&quot;dispatchable_deposit&quot;),<br/>        );<br/>        // Verify that caller type matches callee type so wrongly typed function cannot be registered.<br/>        assert!(<br/>            function_info::check_dispatch_type_compatibility(<br/>                &amp;dispatcher_deposit_function_info,<br/>                deposit_function<br/>            ),<br/>            error::invalid_argument(<br/>                EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH<br/>            )<br/>        );<br/>    &#125;);<br/><br/>    option::for_each_ref(&amp;derived_balance_function, &#124;balance_function&#124; &#123;<br/>        let dispatcher_derived_balance_function_info &#61; function_info::new_function_info_from_address(<br/>            @aptos_framework,<br/>            string::utf8(b&quot;dispatchable_fungible_asset&quot;),<br/>            string::utf8(b&quot;dispatchable_derived_balance&quot;),<br/>        );<br/>        // Verify that caller type matches callee type so wrongly typed function cannot be registered.<br/>        assert!(<br/>            function_info::check_dispatch_type_compatibility(<br/>                &amp;dispatcher_derived_balance_function_info,<br/>                balance_function<br/>            ),<br/>            error::invalid_argument(<br/>                EDERIVED_BALANCE_FUNCTION_SIGNATURE_MISMATCH<br/>            )<br/>        );<br/>    &#125;);<br/><br/>    // Cannot register hook for APT.<br/>    assert!(<br/>        object::address_from_constructor_ref(constructor_ref) !&#61; @aptos_fungible_asset,<br/>        error::permission_denied(EAPT_NOT_DISPATCHABLE)<br/>    );<br/>    assert!(<br/>        !object::can_generate_delete_ref(constructor_ref),<br/>        error::invalid_argument(EOBJECT_IS_DELETABLE)<br/>    );<br/>    assert!(<br/>        !exists&lt;DispatchFunctionStore&gt;(<br/>            object::address_from_constructor_ref(constructor_ref)<br/>        ),<br/>        error::already_exists(EALREADY_REGISTERED)<br/>    );<br/>    assert!(<br/>        exists&lt;Metadata&gt;(<br/>            object::address_from_constructor_ref(constructor_ref)<br/>        ),<br/>        error::not_found(EFUNGIBLE_METADATA_EXISTENCE),<br/>    );<br/><br/>    let store_obj &#61; &amp;object::generate_signer(constructor_ref);<br/><br/>    // Store the overload function hook.<br/>    move_to&lt;DispatchFunctionStore&gt;(<br/>        store_obj,<br/>        DispatchFunctionStore &#123;<br/>            withdraw_function,<br/>            deposit_function,<br/>            derived_balance_function,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_generate_mint_ref"></a>

## Function `generate_mint_ref`

Creates a mint ref that can be used to mint fungible assets from the given fungible object&apos;s constructor ref.<br/> This can only be called at object creation time as constructor_ref is only available then.


<pre><code>public fun generate_mint_ref(constructor_ref: &amp;object::ConstructorRef): fungible_asset::MintRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mint_ref(constructor_ref: &amp;ConstructorRef): MintRef &#123;<br/>    let metadata &#61; object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref);<br/>    MintRef &#123; metadata &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a burn ref that can be used to burn fungible assets from the given fungible object&apos;s constructor ref.<br/> This can only be called at object creation time as constructor_ref is only available then.


<pre><code>public fun generate_burn_ref(constructor_ref: &amp;object::ConstructorRef): fungible_asset::BurnRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_burn_ref(constructor_ref: &amp;ConstructorRef): BurnRef &#123;<br/>    let metadata &#61; object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref);<br/>    BurnRef &#123; metadata &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Creates a transfer ref that can be used to freeze/unfreeze/transfer fungible assets from the given fungible<br/> object&apos;s constructor ref.<br/> This can only be called at object creation time as constructor_ref is only available then.


<pre><code>public fun generate_transfer_ref(constructor_ref: &amp;object::ConstructorRef): fungible_asset::TransferRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_transfer_ref(constructor_ref: &amp;ConstructorRef): TransferRef &#123;<br/>    let metadata &#61; object::object_from_constructor_ref&lt;Metadata&gt;(constructor_ref);<br/>    TransferRef &#123; metadata &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_supply"></a>

## Function `supply`

Get the current supply from the <code>metadata</code> object.


<pre><code>&#35;[view]<br/>public fun supply&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): option::Option&lt;u128&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun supply&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; acquires Supply, ConcurrentSupply &#123;<br/>    let metadata_address &#61; object::object_address(&amp;metadata);<br/>    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global&lt;ConcurrentSupply&gt;(metadata_address);<br/>        option::some(aggregator_v2::read(&amp;supply.current))<br/>    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global&lt;Supply&gt;(metadata_address);<br/>        option::some(supply.current)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_maximum"></a>

## Function `maximum`

Get the maximum supply from the <code>metadata</code> object.<br/> If supply is unlimited (or set explicitly to MAX_U128), none is returned


<pre><code>&#35;[view]<br/>public fun maximum&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): option::Option&lt;u128&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun maximum&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; acquires Supply, ConcurrentSupply &#123;<br/>    let metadata_address &#61; object::object_address(&amp;metadata);<br/>    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global&lt;ConcurrentSupply&gt;(metadata_address);<br/>        let max_value &#61; aggregator_v2::max_value(&amp;supply.current);<br/>        if (max_value &#61;&#61; MAX_U128) &#123;<br/>            option::none()<br/>        &#125; else &#123;<br/>            option::some(max_value)<br/>        &#125;<br/>    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global&lt;Supply&gt;(metadata_address);<br/>        supply.maximum<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_name"></a>

## Function `name`

Get the name of the fungible asset from the <code>metadata</code> object.


<pre><code>&#35;[view]<br/>public fun name&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;T: key&gt;(metadata: Object&lt;T&gt;): String acquires Metadata &#123;<br/>    borrow_fungible_metadata(&amp;metadata).name<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_symbol"></a>

## Function `symbol`

Get the symbol of the fungible asset from the <code>metadata</code> object.


<pre><code>&#35;[view]<br/>public fun symbol&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun symbol&lt;T: key&gt;(metadata: Object&lt;T&gt;): String acquires Metadata &#123;<br/>    borrow_fungible_metadata(&amp;metadata).symbol<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_decimals"></a>

## Function `decimals`

Get the decimals from the <code>metadata</code> object.


<pre><code>&#35;[view]<br/>public fun decimals&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun decimals&lt;T: key&gt;(metadata: Object&lt;T&gt;): u8 acquires Metadata &#123;<br/>    borrow_fungible_metadata(&amp;metadata).decimals<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_store_exists"></a>

## Function `store_exists`

Return whether the provided address has a store initialized.


<pre><code>&#35;[view]<br/>public fun store_exists(store: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun store_exists(store: address): bool &#123;<br/>    exists&lt;FungibleStore&gt;(store)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_metadata_from_asset"></a>

## Function `metadata_from_asset`

Return the underlying metadata object


<pre><code>public fun metadata_from_asset(fa: &amp;fungible_asset::FungibleAsset): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun metadata_from_asset(fa: &amp;FungibleAsset): Object&lt;Metadata&gt; &#123;<br/>    fa.metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_store_metadata"></a>

## Function `store_metadata`

Return the underlying metadata object.


<pre><code>&#35;[view]<br/>public fun store_metadata&lt;T: key&gt;(store: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun store_metadata&lt;T: key&gt;(store: Object&lt;T&gt;): Object&lt;Metadata&gt; acquires FungibleStore &#123;<br/>    borrow_store_resource(&amp;store).metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_amount"></a>

## Function `amount`

Return the <code>amount</code> of a given fungible asset.


<pre><code>public fun amount(fa: &amp;fungible_asset::FungibleAsset): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun amount(fa: &amp;FungibleAsset): u64 &#123;<br/>    fa.amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of a given store.


<pre><code>&#35;[view]<br/>public fun balance&lt;T: key&gt;(store: object::Object&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance&lt;T: key&gt;(store: Object&lt;T&gt;): u64 acquires FungibleStore &#123;<br/>    if (store_exists(object::object_address(&amp;store))) &#123;<br/>        borrow_store_resource(&amp;store).balance<br/>    &#125; else &#123;<br/>        0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_is_balance_at_least"></a>

## Function `is_balance_at_least`

Check whether the balance of a given store is &gt;&#61; <code>amount</code>.


<pre><code>&#35;[view]<br/>public fun is_balance_at_least&lt;T: key&gt;(store: object::Object&lt;T&gt;, amount: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_balance_at_least&lt;T: key&gt;(store: Object&lt;T&gt;, amount: u64): bool acquires FungibleStore &#123;<br/>    let store_addr &#61; object::object_address(&amp;store);<br/>    if (store_exists(store_addr)) &#123;<br/>        borrow_store_resource(&amp;store).balance &gt;&#61; amount<br/>    &#125; else &#123;<br/>        amount &#61;&#61; 0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_is_frozen"></a>

## Function `is_frozen`

Return whether a store is frozen.<br/><br/> If the store has not been created, we default to returning false so deposits can be sent to it.


<pre><code>&#35;[view]<br/>public fun is_frozen&lt;T: key&gt;(store: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_frozen&lt;T: key&gt;(store: Object&lt;T&gt;): bool acquires FungibleStore &#123;<br/>    store_exists(object::object_address(&amp;store)) &amp;&amp; borrow_store_resource(&amp;store).frozen<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_is_store_dispatchable"></a>

## Function `is_store_dispatchable`

Return whether a fungible asset type is dispatchable.


<pre><code>&#35;[view]<br/>public fun is_store_dispatchable&lt;T: key&gt;(store: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_store_dispatchable&lt;T: key&gt;(store: Object&lt;T&gt;): bool acquires FungibleStore &#123;<br/>    let fa_store &#61; borrow_store_resource(&amp;store);<br/>    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);<br/>    exists&lt;DispatchFunctionStore&gt;(metadata_addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_dispatch_function"></a>

## Function `deposit_dispatch_function`



<pre><code>public fun deposit_dispatch_function&lt;T: key&gt;(store: object::Object&lt;T&gt;): option::Option&lt;function_info::FunctionInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_dispatch_function&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; acquires FungibleStore, DispatchFunctionStore &#123;<br/>    let fa_store &#61; borrow_store_resource(&amp;store);<br/>    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);<br/>    if(exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;<br/>        borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).deposit_function<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_has_deposit_dispatch_function"></a>

## Function `has_deposit_dispatch_function`



<pre><code>fun has_deposit_dispatch_function(metadata: object::Object&lt;fungible_asset::Metadata&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun has_deposit_dispatch_function(metadata: Object&lt;Metadata&gt;): bool acquires DispatchFunctionStore &#123;<br/>    let metadata_addr &#61; object::object_address(&amp;metadata);<br/>    // Short circuit on APT for better perf<br/>    if(metadata_addr !&#61; @aptos_fungible_asset &amp;&amp; exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;<br/>        option::is_some(&amp;borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).deposit_function)<br/>    &#125; else &#123;<br/>        false<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_dispatch_function"></a>

## Function `withdraw_dispatch_function`



<pre><code>public fun withdraw_dispatch_function&lt;T: key&gt;(store: object::Object&lt;T&gt;): option::Option&lt;function_info::FunctionInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_dispatch_function&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; acquires FungibleStore, DispatchFunctionStore &#123;<br/>    let fa_store &#61; borrow_store_resource(&amp;store);<br/>    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);<br/>    if(exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;<br/>        borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).withdraw_function<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_has_withdraw_dispatch_function"></a>

## Function `has_withdraw_dispatch_function`



<pre><code>fun has_withdraw_dispatch_function(metadata: object::Object&lt;fungible_asset::Metadata&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun has_withdraw_dispatch_function(metadata: Object&lt;Metadata&gt;): bool acquires DispatchFunctionStore &#123;<br/>    let metadata_addr &#61; object::object_address(&amp;metadata);<br/>    // Short circuit on APT for better perf<br/>    if (metadata_addr !&#61; @aptos_fungible_asset &amp;&amp; exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;<br/>        option::is_some(&amp;borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).withdraw_function)<br/>    &#125; else &#123;<br/>        false<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_derived_balance_dispatch_function"></a>

## Function `derived_balance_dispatch_function`



<pre><code>public(friend) fun derived_balance_dispatch_function&lt;T: key&gt;(store: object::Object&lt;T&gt;): option::Option&lt;function_info::FunctionInfo&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun derived_balance_dispatch_function&lt;T: key&gt;(store: Object&lt;T&gt;): Option&lt;FunctionInfo&gt; acquires FungibleStore, DispatchFunctionStore &#123;<br/>    let fa_store &#61; borrow_store_resource(&amp;store);<br/>    let metadata_addr &#61; object::object_address(&amp;fa_store.metadata);<br/>    if (exists&lt;DispatchFunctionStore&gt;(metadata_addr)) &#123;<br/>        borrow_global&lt;DispatchFunctionStore&gt;(metadata_addr).derived_balance_function<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_asset_metadata"></a>

## Function `asset_metadata`



<pre><code>public fun asset_metadata(fa: &amp;fungible_asset::FungibleAsset): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun asset_metadata(fa: &amp;FungibleAsset): Object&lt;Metadata&gt; &#123;<br/>    fa.metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_mint_ref_metadata"></a>

## Function `mint_ref_metadata`

Get the underlying metadata object from the <code>MintRef</code>.


<pre><code>public fun mint_ref_metadata(ref: &amp;fungible_asset::MintRef): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_ref_metadata(ref: &amp;MintRef): Object&lt;Metadata&gt; &#123;<br/>    ref.metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_transfer_ref_metadata"></a>

## Function `transfer_ref_metadata`

Get the underlying metadata object from the <code>TransferRef</code>.


<pre><code>public fun transfer_ref_metadata(ref: &amp;fungible_asset::TransferRef): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_ref_metadata(ref: &amp;TransferRef): Object&lt;Metadata&gt; &#123;<br/>    ref.metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_burn_ref_metadata"></a>

## Function `burn_ref_metadata`

Get the underlying metadata object from the <code>BurnRef</code>.


<pre><code>public fun burn_ref_metadata(ref: &amp;fungible_asset::BurnRef): object::Object&lt;fungible_asset::Metadata&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn_ref_metadata(ref: &amp;BurnRef): Object&lt;Metadata&gt; &#123;<br/>    ref.metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.<br/> Note: it does not move the underlying object.


<pre><code>public entry fun transfer&lt;T: key&gt;(sender: &amp;signer, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;T: key&gt;(<br/>    sender: &amp;signer,<br/>    from: Object&lt;T&gt;,<br/>    to: Object&lt;T&gt;,<br/>    amount: u64,<br/>) acquires FungibleStore, DispatchFunctionStore &#123;<br/>    let fa &#61; withdraw(sender, from, amount);<br/>    deposit(to, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_create_store"></a>

## Function `create_store`

Allow an object to hold a store for fungible assets.<br/> Applications can use this to create multiple stores for isolating fungible assets for different purposes.


<pre><code>public fun create_store&lt;T: key&gt;(constructor_ref: &amp;object::ConstructorRef, metadata: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::FungibleStore&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_store&lt;T: key&gt;(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    metadata: Object&lt;T&gt;,<br/>): Object&lt;FungibleStore&gt; &#123;<br/>    let store_obj &#61; &amp;object::generate_signer(constructor_ref);<br/>    move_to(store_obj, FungibleStore &#123;<br/>        metadata: object::convert(metadata),<br/>        balance: 0,<br/>        frozen: false,<br/>    &#125;);<br/>    if (is_untransferable(metadata)) &#123;<br/>        object::set_untransferable(constructor_ref);<br/>    &#125;;<br/>    object::object_from_constructor_ref&lt;FungibleStore&gt;(constructor_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_remove_store"></a>

## Function `remove_store`

Used to delete a store.  Requires the store to be completely empty prior to removing it


<pre><code>public fun remove_store(delete_ref: &amp;object::DeleteRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_store(delete_ref: &amp;DeleteRef) acquires FungibleStore, FungibleAssetEvents &#123;<br/>    let store &#61; &amp;object::object_from_delete_ref&lt;FungibleStore&gt;(delete_ref);<br/>    let addr &#61; object::object_address(store);<br/>    let FungibleStore &#123; metadata: _, balance, frozen: _ &#125;<br/>        &#61; move_from&lt;FungibleStore&gt;(addr);<br/>    assert!(balance &#61;&#61; 0, error::permission_denied(EBALANCE_IS_NOT_ZERO));<br/>    // Cleanup deprecated event handles if exist.<br/>    if (exists&lt;FungibleAssetEvents&gt;(addr)) &#123;<br/>        let FungibleAssetEvents &#123;<br/>            deposit_events,<br/>            withdraw_events,<br/>            frozen_events,<br/>        &#125; &#61; move_from&lt;FungibleAssetEvents&gt;(addr);<br/>        event::destroy_handle(deposit_events);<br/>        event::destroy_handle(withdraw_events);<br/>        event::destroy_handle(frozen_events);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.


<pre><code>public fun withdraw&lt;T: key&gt;(owner: &amp;signer, store: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;T: key&gt;(<br/>    owner: &amp;signer,<br/>    store: Object&lt;T&gt;,<br/>    amount: u64,<br/>): FungibleAsset acquires FungibleStore, DispatchFunctionStore &#123;<br/>    withdraw_sanity_check(owner, store, true);<br/>    withdraw_internal(object::object_address(&amp;store), amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_sanity_check"></a>

## Function `withdraw_sanity_check`

Check the permission for withdraw operation.


<pre><code>public(friend) fun withdraw_sanity_check&lt;T: key&gt;(owner: &amp;signer, store: object::Object&lt;T&gt;, abort_on_dispatch: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun withdraw_sanity_check&lt;T: key&gt;(<br/>    owner: &amp;signer,<br/>    store: Object&lt;T&gt;,<br/>    abort_on_dispatch: bool,<br/>) acquires FungibleStore, DispatchFunctionStore &#123;<br/>    assert!(object::owns(store, signer::address_of(owner)), error::permission_denied(ENOT_STORE_OWNER));<br/>    let fa_store &#61; borrow_store_resource(&amp;store);<br/>    assert!(<br/>        !abort_on_dispatch &#124;&#124; !has_withdraw_dispatch_function(fa_store.metadata),<br/>        error::invalid_argument(EINVALID_DISPATCHABLE_OPERATIONS)<br/>    );<br/>    assert!(!fa_store.frozen, error::permission_denied(ESTORE_IS_FROZEN));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_sanity_check"></a>

## Function `deposit_sanity_check`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code>public fun deposit_sanity_check&lt;T: key&gt;(store: object::Object&lt;T&gt;, abort_on_dispatch: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_sanity_check&lt;T: key&gt;(<br/>    store: Object&lt;T&gt;,<br/>    abort_on_dispatch: bool<br/>) acquires FungibleStore, DispatchFunctionStore &#123;<br/>    let fa_store &#61; borrow_store_resource(&amp;store);<br/>    assert!(<br/>        !abort_on_dispatch &#124;&#124; !has_deposit_dispatch_function(fa_store.metadata),<br/>        error::invalid_argument(EINVALID_DISPATCHABLE_OPERATIONS)<br/>    );<br/>    assert!(!fa_store.frozen, error::permission_denied(ESTORE_IS_FROZEN));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.


<pre><code>public fun deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) acquires FungibleStore, DispatchFunctionStore &#123;<br/>    deposit_sanity_check(store, true);<br/>    deposit_internal(store, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint the specified <code>amount</code> of the fungible asset.


<pre><code>public fun mint(ref: &amp;fungible_asset::MintRef, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint(ref: &amp;MintRef, amount: u64): FungibleAsset acquires Supply, ConcurrentSupply &#123;<br/>    let metadata &#61; ref.metadata;<br/>    mint_internal(metadata, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_mint_internal"></a>

## Function `mint_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code>public(friend) fun mint_internal(metadata: object::Object&lt;fungible_asset::Metadata&gt;, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun mint_internal(<br/>    metadata: Object&lt;Metadata&gt;,<br/>    amount: u64<br/>): FungibleAsset acquires Supply, ConcurrentSupply &#123;<br/>    increase_supply(&amp;metadata, amount);<br/>    FungibleAsset &#123;<br/>        metadata,<br/>        amount<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_mint_to"></a>

## Function `mint_to`

Mint the specified <code>amount</code> of the fungible asset to a destination store.


<pre><code>public fun mint_to&lt;T: key&gt;(ref: &amp;fungible_asset::MintRef, store: object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_to&lt;T: key&gt;(ref: &amp;MintRef, store: Object&lt;T&gt;, amount: u64)<br/>acquires FungibleStore, Supply, ConcurrentSupply, DispatchFunctionStore &#123;<br/>    deposit_sanity_check(store, false);<br/>    deposit_internal(store, mint(ref, amount));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag"></a>

## Function `set_frozen_flag`

Enable/disable a store&apos;s ability to do direct transfers of the fungible asset.


<pre><code>public fun set_frozen_flag&lt;T: key&gt;(ref: &amp;fungible_asset::TransferRef, store: object::Object&lt;T&gt;, frozen: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_frozen_flag&lt;T: key&gt;(<br/>    ref: &amp;TransferRef,<br/>    store: Object&lt;T&gt;,<br/>    frozen: bool,<br/>) acquires FungibleStore &#123;<br/>    assert!(<br/>        ref.metadata &#61;&#61; store_metadata(store),<br/>        error::invalid_argument(ETRANSFER_REF_AND_STORE_MISMATCH),<br/>    );<br/>    set_frozen_flag_internal(store, frozen)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_set_frozen_flag_internal"></a>

## Function `set_frozen_flag_internal`



<pre><code>public(friend) fun set_frozen_flag_internal&lt;T: key&gt;(store: object::Object&lt;T&gt;, frozen: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun set_frozen_flag_internal&lt;T: key&gt;(<br/>    store: Object&lt;T&gt;,<br/>    frozen: bool<br/>) acquires FungibleStore &#123;<br/>    let store_addr &#61; object::object_address(&amp;store);<br/>    borrow_global_mut&lt;FungibleStore&gt;(store_addr).frozen &#61; frozen;<br/><br/>    event::emit(Frozen &#123; store: store_addr, frozen &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_burn"></a>

## Function `burn`

Burns a fungible asset


<pre><code>public fun burn(ref: &amp;fungible_asset::BurnRef, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn(ref: &amp;BurnRef, fa: FungibleAsset) acquires Supply, ConcurrentSupply &#123;<br/>    assert!(<br/>        ref.metadata &#61;&#61; metadata_from_asset(&amp;fa),<br/>        error::invalid_argument(EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH)<br/>    );<br/>    burn_internal(fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_burn_internal"></a>

## Function `burn_internal`

CAN ONLY BE CALLED BY coin.move for migration.


<pre><code>public(friend) fun burn_internal(fa: fungible_asset::FungibleAsset): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun burn_internal(<br/>    fa: FungibleAsset<br/>): u64 acquires Supply, ConcurrentSupply &#123;<br/>    let FungibleAsset &#123;<br/>        metadata,<br/>        amount<br/>    &#125; &#61; fa;<br/>    decrease_supply(&amp;metadata, amount);<br/>    amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_burn_from"></a>

## Function `burn_from`

Burn the <code>amount</code> of the fungible asset from the given store.


<pre><code>public fun burn_from&lt;T: key&gt;(ref: &amp;fungible_asset::BurnRef, store: object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn_from&lt;T: key&gt;(<br/>    ref: &amp;BurnRef,<br/>    store: Object&lt;T&gt;,<br/>    amount: u64<br/>) acquires FungibleStore, Supply, ConcurrentSupply &#123;<br/>    let metadata &#61; ref.metadata;<br/>    assert!(metadata &#61;&#61; store_metadata(store), error::invalid_argument(EBURN_REF_AND_STORE_MISMATCH));<br/>    let store_addr &#61; object::object_address(&amp;store);<br/>    burn(ref, withdraw_internal(store_addr, amount));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw <code>amount</code> of the fungible asset from the <code>store</code> ignoring <code>frozen</code>.


<pre><code>public fun withdraw_with_ref&lt;T: key&gt;(ref: &amp;fungible_asset::TransferRef, store: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_ref&lt;T: key&gt;(<br/>    ref: &amp;TransferRef,<br/>    store: Object&lt;T&gt;,<br/>    amount: u64<br/>): FungibleAsset acquires FungibleStore &#123;<br/>    assert!(<br/>        ref.metadata &#61;&#61; store_metadata(store),<br/>        error::invalid_argument(ETRANSFER_REF_AND_STORE_MISMATCH),<br/>    );<br/>    withdraw_internal(object::object_address(&amp;store), amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit the fungible asset into the <code>store</code> ignoring <code>frozen</code>.


<pre><code>public fun deposit_with_ref&lt;T: key&gt;(ref: &amp;fungible_asset::TransferRef, store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_with_ref&lt;T: key&gt;(<br/>    ref: &amp;TransferRef,<br/>    store: Object&lt;T&gt;,<br/>    fa: FungibleAsset<br/>) acquires FungibleStore &#123;<br/>    assert!(<br/>        ref.metadata &#61;&#61; fa.metadata,<br/>        error::invalid_argument(ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH)<br/>    );<br/>    deposit_internal(store, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>amount</code> of the fungible asset with <code>TransferRef</code> even it is frozen.


<pre><code>public fun transfer_with_ref&lt;T: key&gt;(transfer_ref: &amp;fungible_asset::TransferRef, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_with_ref&lt;T: key&gt;(<br/>    transfer_ref: &amp;TransferRef,<br/>    from: Object&lt;T&gt;,<br/>    to: Object&lt;T&gt;,<br/>    amount: u64,<br/>) acquires FungibleStore &#123;<br/>    let fa &#61; withdraw_with_ref(transfer_ref, from, amount);<br/>    deposit_with_ref(transfer_ref, to, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_zero"></a>

## Function `zero`

Create a fungible asset with zero amount.<br/> This can be useful when starting a series of computations where the initial value is 0.


<pre><code>public fun zero&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun zero&lt;T: key&gt;(metadata: Object&lt;T&gt;): FungibleAsset &#123;<br/>    FungibleAsset &#123;<br/>        metadata: object::convert(metadata),<br/>        amount: 0,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract a given amount from the given fungible asset and return a new one.


<pre><code>public fun extract(fungible_asset: &amp;mut fungible_asset::FungibleAsset, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract(fungible_asset: &amp;mut FungibleAsset, amount: u64): FungibleAsset &#123;<br/>    assert!(fungible_asset.amount &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));<br/>    fungible_asset.amount &#61; fungible_asset.amount &#45; amount;<br/>    FungibleAsset &#123;<br/>        metadata: fungible_asset.metadata,<br/>        amount,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_merge"></a>

## Function `merge`

&quot;Merges&quot; the two given fungible assets. The fungible asset passed in as <code>dst_fungible_asset</code> will have a value<br/> equal to the sum of the two (<code>dst_fungible_asset</code> and <code>src_fungible_asset</code>).


<pre><code>public fun merge(dst_fungible_asset: &amp;mut fungible_asset::FungibleAsset, src_fungible_asset: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun merge(dst_fungible_asset: &amp;mut FungibleAsset, src_fungible_asset: FungibleAsset) &#123;<br/>    let FungibleAsset &#123; metadata, amount &#125; &#61; src_fungible_asset;<br/>    assert!(metadata &#61;&#61; dst_fungible_asset.metadata, error::invalid_argument(EFUNGIBLE_ASSET_MISMATCH));<br/>    dst_fungible_asset.amount &#61; dst_fungible_asset.amount &#43; amount;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_destroy_zero"></a>

## Function `destroy_zero`

Destroy an empty fungible asset.


<pre><code>public fun destroy_zero(fungible_asset: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_zero(fungible_asset: FungibleAsset) &#123;<br/>    let FungibleAsset &#123; amount, metadata: _ &#125; &#61; fungible_asset;<br/>    assert!(amount &#61;&#61; 0, error::invalid_argument(EAMOUNT_IS_NOT_ZERO));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_deposit_internal"></a>

## Function `deposit_internal`



<pre><code>public(friend) fun deposit_internal&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun deposit_internal&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) acquires FungibleStore &#123;<br/>    let FungibleAsset &#123; metadata, amount &#125; &#61; fa;<br/>    if (amount &#61;&#61; 0) return;<br/><br/>    let store_metadata &#61; store_metadata(store);<br/>    assert!(metadata &#61;&#61; store_metadata, error::invalid_argument(EFUNGIBLE_ASSET_AND_STORE_MISMATCH));<br/>    let store_addr &#61; object::object_address(&amp;store);<br/>    let store &#61; borrow_global_mut&lt;FungibleStore&gt;(store_addr);<br/>    store.balance &#61; store.balance &#43; amount;<br/><br/>    event::emit(Deposit &#123; store: store_addr, amount &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_withdraw_internal"></a>

## Function `withdraw_internal`

Extract <code>amount</code> of the fungible asset from <code>store</code>.


<pre><code>public(friend) fun withdraw_internal(store_addr: address, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun withdraw_internal(<br/>    store_addr: address,<br/>    amount: u64,<br/>): FungibleAsset acquires FungibleStore &#123;<br/>    assert!(exists&lt;FungibleStore&gt;(store_addr), error::not_found(EFUNGIBLE_STORE_EXISTENCE));<br/>    let store &#61; borrow_global_mut&lt;FungibleStore&gt;(store_addr);<br/>    let metadata &#61; store.metadata;<br/>    if (amount !&#61; 0) &#123;<br/>        assert!(store.balance &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));<br/>        store.balance &#61; store.balance &#45; amount;<br/>        event::emit&lt;Withdraw&gt;(Withdraw &#123; store: store_addr, amount &#125;);<br/>    &#125;;<br/>    FungibleAsset &#123; metadata, amount &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_increase_supply"></a>

## Function `increase_supply`

Increase the supply of a fungible asset by minting.


<pre><code>fun increase_supply&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun increase_supply&lt;T: key&gt;(metadata: &amp;Object&lt;T&gt;, amount: u64) acquires Supply, ConcurrentSupply &#123;<br/>    if (amount &#61;&#61; 0) &#123;<br/>        return<br/>    &#125;;<br/>    let metadata_address &#61; object::object_address(metadata);<br/><br/>    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(metadata_address);<br/>        assert!(<br/>            aggregator_v2::try_add(&amp;mut supply.current, (amount as u128)),<br/>            error::out_of_range(EMAX_SUPPLY_EXCEEDED)<br/>        );<br/>    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;Supply&gt;(metadata_address);<br/>        if (option::is_some(&amp;supply.maximum)) &#123;<br/>            let max &#61; &#42;option::borrow_mut(&amp;mut supply.maximum);<br/>            assert!(<br/>                max &#45; supply.current &gt;&#61; (amount as u128),<br/>                error::out_of_range(EMAX_SUPPLY_EXCEEDED)<br/>            )<br/>        &#125;;<br/>        supply.current &#61; supply.current &#43; (amount as u128);<br/>    &#125; else &#123;<br/>        abort error::not_found(ESUPPLY_NOT_FOUND)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_decrease_supply"></a>

## Function `decrease_supply`

Decrease the supply of a fungible asset by burning.


<pre><code>fun decrease_supply&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun decrease_supply&lt;T: key&gt;(metadata: &amp;Object&lt;T&gt;, amount: u64) acquires Supply, ConcurrentSupply &#123;<br/>    if (amount &#61;&#61; 0) &#123;<br/>        return<br/>    &#125;;<br/>    let metadata_address &#61; object::object_address(metadata);<br/><br/>    if (exists&lt;ConcurrentSupply&gt;(metadata_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(metadata_address);<br/><br/>        assert!(<br/>            aggregator_v2::try_sub(&amp;mut supply.current, (amount as u128)),<br/>            error::out_of_range(ESUPPLY_UNDERFLOW)<br/>        );<br/>    &#125; else if (exists&lt;Supply&gt;(metadata_address)) &#123;<br/>        assert!(exists&lt;Supply&gt;(metadata_address), error::not_found(ESUPPLY_NOT_FOUND));<br/>        let supply &#61; borrow_global_mut&lt;Supply&gt;(metadata_address);<br/>        assert!(<br/>            supply.current &gt;&#61; (amount as u128),<br/>            error::invalid_state(ESUPPLY_UNDERFLOW)<br/>        );<br/>        supply.current &#61; supply.current &#45; (amount as u128);<br/>    &#125; else &#123;<br/>        assert!(false, error::not_found(ESUPPLY_NOT_FOUND));<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata"></a>

## Function `borrow_fungible_metadata`



<pre><code>fun borrow_fungible_metadata&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;): &amp;fungible_asset::Metadata<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_fungible_metadata&lt;T: key&gt;(<br/>    metadata: &amp;Object&lt;T&gt;<br/>): &amp;Metadata acquires Metadata &#123;<br/>    let addr &#61; object::object_address(metadata);<br/>    borrow_global&lt;Metadata&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_borrow_fungible_metadata_mut"></a>

## Function `borrow_fungible_metadata_mut`



<pre><code>fun borrow_fungible_metadata_mut&lt;T: key&gt;(metadata: &amp;object::Object&lt;T&gt;): &amp;mut fungible_asset::Metadata<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_fungible_metadata_mut&lt;T: key&gt;(<br/>    metadata: &amp;Object&lt;T&gt;<br/>): &amp;mut Metadata acquires Metadata &#123;<br/>    let addr &#61; object::object_address(metadata);<br/>    borrow_global_mut&lt;Metadata&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_borrow_store_resource"></a>

## Function `borrow_store_resource`



<pre><code>fun borrow_store_resource&lt;T: key&gt;(store: &amp;object::Object&lt;T&gt;): &amp;fungible_asset::FungibleStore<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_store_resource&lt;T: key&gt;(store: &amp;Object&lt;T&gt;): &amp;FungibleStore acquires FungibleStore &#123;<br/>    let store_addr &#61; object::object_address(store);<br/>    assert!(exists&lt;FungibleStore&gt;(store_addr), error::not_found(EFUNGIBLE_STORE_EXISTENCE));<br/>    borrow_global&lt;FungibleStore&gt;(store_addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fungible_asset_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code>public fun upgrade_to_concurrent(ref: &amp;object::ExtendRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_to_concurrent(<br/>    ref: &amp;ExtendRef,<br/>) acquires Supply &#123;<br/>    let metadata_object_address &#61; object::address_from_extend_ref(ref);<br/>    let metadata_object_signer &#61; object::generate_signer_for_extending(ref);<br/>    assert!(<br/>        features::concurrent_fungible_assets_enabled(),<br/>        error::invalid_argument(ECONCURRENT_SUPPLY_NOT_ENABLED)<br/>    );<br/>    assert!(exists&lt;Supply&gt;(metadata_object_address), error::not_found(ESUPPLY_NOT_FOUND));<br/>    let Supply &#123;<br/>        current,<br/>        maximum,<br/>    &#125; &#61; move_from&lt;Supply&gt;(metadata_object_address);<br/><br/>    let unlimited &#61; option::is_none(&amp;maximum);<br/>    let supply &#61; ConcurrentSupply &#123;<br/>        current: if (unlimited) &#123;<br/>            aggregator_v2::create_unbounded_aggregator()<br/>        &#125;<br/>        else &#123;<br/>            aggregator_v2::create_aggregator(option::extract(&amp;mut maximum))<br/>        &#125;,<br/>    &#125;;<br/>    // update current state:<br/>    aggregator_v2::add(&amp;mut supply.current, current);<br/>    move_to(&amp;metadata_object_signer, supply);<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The metadata associated with the fungible asset is subject to precise size constraints.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The add_fungibility function has size limitations for the name, symbol, number of decimals, icon_uri, and project_uri field of the Metadata resource.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;Adding fungibility to an existing object should initialize the metadata and supply resources and store them under the metadata object address.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The add_fungibility function initializes the Metadata and Supply resources and moves them under the metadata object.&lt;/td&gt;<br/>&lt;td&gt;Audited that the Metadata and Supply resources are initialized properly.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Generating mint, burn and transfer references can only be done at object creation time and if the object was added fungibility.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The following functions generate the related references of the Metadata object: 1. generate_mint_ref 2. generate_burn_ref 3. generate_transfer_ref&lt;/td&gt;<br/>&lt;td&gt;Audited that the Metadata object exists within the constructor ref.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;Only the owner of a store should be allowed to withdraw fungible assets from it.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The fungible_asset::withdraw function ensures that the signer owns the store by asserting that the object address matches the address of the signer.&lt;/td&gt;<br/>&lt;td&gt;Audited that the address of the signer owns the object.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;The transfer, withdrawal and deposit operation should never change the current supply of the fungible asset.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The transfer function withdraws the fungible assets from the store and deposits them to the receiver. The withdraw function extracts the fungible asset from the fungible asset store. The deposit function adds the balance to the fungible asset store.&lt;/td&gt;<br/>&lt;td&gt;Audited that the supply before and after the operation remains constant.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;The owner of the store should only be able to withdraw a certain amount if its store has sufficient balance and is not frozen, unless the withdrawal is performed with a reference, and afterwards the store balance should be decreased.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The withdraw function ensures that the store is not frozen before calling withdraw_internal which ensures that the withdrawing amount is greater than 0 and less than the total balance from the store. The withdraw_with_ref ensures that the reference&apos;s metadata matches the store metadata.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the withdrawing store is frozen. Audited that it aborts if the store doesn&apos;t have sufficient balance. Audited that the balance of the withdrawing store is reduced by amount.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;Only the same type of fungible assets should be deposited in a fungible asset store, if the store is not frozen, unless the deposit is performed with a reference, and afterwards the store balance should be increased.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The deposit function ensures that store is not frozen and proceeds to call the deposit_internal function which validates the store&apos;s metadata and the depositing asset&apos;s metadata followed by increasing the store balance by the given amount. The deposit_with_ref ensures that the reference&apos;s metadata matches the depositing asset&apos;s metadata.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the store is frozen. Audited that it aborts if the asset and asset store are different. Audited that the store&apos;s balance is increased by the deposited amount.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;An object should only be allowed to hold one store for fungible assets.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The create_store function initializes a new FungibleStore resource and moves it under the object address.&lt;/td&gt;<br/>&lt;td&gt;Audited that the resource was moved under the object.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;9&lt;/td&gt;<br/>&lt;td&gt;When a new store is created, the balance should be set by default to the value zero.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_store function initializes a new fungible asset store with zero balance and stores it under the given construtorRef object.&lt;/td&gt;<br/>&lt;td&gt;Audited that the store is properly initialized with zero balance.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;10&lt;/td&gt;<br/>&lt;td&gt;A store should only be deleted if its balance is zero.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The remove_store function validates the store&apos;s balance and removes the store under the object address.&lt;/td&gt;<br/>&lt;td&gt;Audited that aborts if the balance of the store is not zero. Audited that store is removed from the object address.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;11&lt;/td&gt;<br/>&lt;td&gt;Minting and burning should alter the total supply value, and the store balances.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The mint process increases the total supply by the amount minted using the increase_supply function. The burn process withdraws the burn amount from the given store and decreases the total supply by the amount burned using the decrease_supply function.&lt;/td&gt;<br/>&lt;td&gt;Audited the mint and burn functions that the supply was adjusted accordingly.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;12&lt;/td&gt;<br/>&lt;td&gt;It must not be possible to burn an amount of fungible assets larger than their current supply.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The burn process ensures that the store has enough balance to burn, by asserting that the supply.current &gt;&#61; amount inside the decrease_supply function.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the provided store doesn&apos;t have sufficient balance.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;13&lt;/td&gt;<br/>&lt;td&gt;Enabling or disabling store&apos;s frozen status should only be done with a valid transfer reference.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The set_frozen_flag function ensures that the TransferRef is provided via function argument and that the store&apos;s metadata matches the metadata from the reference. It then proceeds to update the frozen flag of the store.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the metadata doesn&apos;t match. Audited that the frozen flag is updated properly.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;14&lt;/td&gt;<br/>&lt;td&gt;Extracting a specific amount from the fungible asset should be possible only if the total amount that it holds is greater or equal to the provided amount.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The extract function validates that the fungible asset has enough balance to extract and then updates it by subtracting the extracted amount.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the asset didn&apos;t have sufficient balance. Audited that the balance of the asset is updated. Audited that the extract function returns the extracted asset.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;15&lt;/td&gt;<br/>&lt;td&gt;Merging two fungible assets should only be possible if both share the same metadata.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The merge function validates the metadata of the src and dst asset.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the metadata of the src and dst are not the same.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;16&lt;/td&gt;<br/>&lt;td&gt;Post merging two fungible assets, the source asset should have the amount value equal to the sum of the two.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The merge function increases dst_fungible_asset.amount by src_fungible_asset.amount.&lt;/td&gt;<br/>&lt;td&gt;Audited that the dst_fungible_asset balance is increased by amount.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;17&lt;/td&gt;<br/>&lt;td&gt;Fungible assets with zero balance should be destroyed when the amount reaches value 0.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The destroy_zero ensures that the balance of the asset has the value 0 and destroy the asset.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the balance of the asset is non zero.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify&#61;false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
