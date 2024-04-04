
<a id="0x1_dispatchable_fungible_asset"></a>

# Module `0x1::dispatchable_fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code>Metadata</code> object. The
metadata object can be any object that equipped with <code>Metadata</code> resource.

The dispatchable_fungible_asset wraps the existing fungible_asset module and adds the ability for token issuer
to customize the logic for withdraw and deposit operations. For example:

- Deflation token: a fixed percentage of token will be destructed upon transfer.
- Transfer allowlist: token can only be transfered to addresses in the allow list.
- Predicated transfer: transfer can only happen when some certain predicate has been met.
- Loyalty token: a fixed loyalty will be paid to a designated address when a fungible asset transfer happens

See AIP-73 for further discussion


-  [Resource `TransferRefStore`](#0x1_dispatchable_fungible_asset_TransferRefStore)
-  [Constants](#@Constants_0)
-  [Function `register_dispatch_functions`](#0x1_dispatchable_fungible_asset_register_dispatch_functions)
-  [Function `withdraw`](#0x1_dispatchable_fungible_asset_withdraw)
-  [Function `deposit`](#0x1_dispatchable_fungible_asset_deposit)
-  [Function `derived_value`](#0x1_dispatchable_fungible_asset_derived_value)
-  [Function `borrow_transfer_ref`](#0x1_dispatchable_fungible_asset_borrow_transfer_ref)
-  [Function `transfer_fixed_send`](#0x1_dispatchable_fungible_asset_transfer_fixed_send)
-  [Function `transfer_fixed_receive`](#0x1_dispatchable_fungible_asset_transfer_fixed_receive)
-  [Function `dispatchable_withdraw`](#0x1_dispatchable_fungible_asset_dispatchable_withdraw)
-  [Function `dispatchable_deposit`](#0x1_dispatchable_fungible_asset_dispatchable_deposit)
-  [Function `dispatchable_derived_value`](#0x1_dispatchable_fungible_asset_dispatchable_derived_value)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_TransferRefStore"></a>

## Resource `TransferRefStore`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>transfer_ref: <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_dispatchable_fungible_asset_ENOT_ACTIVATED"></a>

Feature is not activated yet on the network.


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>: u64 = 3;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH"></a>

Recipient is not getting the guaranteed value;


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>: u64 = 2;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND"></a>

TransferRefStore doesn't exist on the fungible asset type.


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND">ESTORE_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`



<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, withdraw_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, deposit_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, derived_value_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(
    constructor_ref: &ConstructorRef,
    withdraw_function: FunctionInfo,
    deposit_function: FunctionInfo,
    derived_value_function: FunctionInfo,
) {
    <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">fungible_asset::register_dispatch_functions</a>(
        constructor_ref,
        withdraw_function,
        deposit_function,
        derived_value_function,
    );
    <b>let</b> store_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>&lt;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a>&gt;(
        store_obj,
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
            transfer_ref: <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">fungible_asset::generate_transfer_ref</a>(constructor_ref),
        }
    );
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.

The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: Object&lt;T&gt;,
    amount: u64,
): FungibleAsset <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_is_dispatchable">fungible_asset::is_dispatchable</a>(store)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">fungible_asset::withdraw_dispatch_function</a>(store);
        <a href="function_info.md#0x1_function_info_load_function">function_info::load_function</a>(&func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>(
            <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), store,
            amount,
            <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>(store),
            &func,
        )
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_withdraw">fungible_asset::withdraw</a>(owner, store, amount)
    }
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.

The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_is_dispatchable">fungible_asset::is_dispatchable</a>(store)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">fungible_asset::deposit_dispatch_function</a>(store);
        <a href="function_info.md#0x1_function_info_load_function">function_info::load_function</a>(&func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>(
            store,
            fa,
            <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>(store),
            &func
        )
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(store, fa)
    }
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_derived_value"></a>

## Function `derived_value`

Get the derived value of store using the overloaded hook.

The semantics of value will be governed by the function specified in DispatchFunctionStore.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_value">derived_value</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_value">derived_value</a>&lt;T: key&gt;(store: Object&lt;T&gt;): u64 {
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_is_dispatchable">fungible_asset::is_dispatchable</a>(store)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="fungible_asset.md#0x1_fungible_asset_derived_value_dispatch_function">fungible_asset::derived_value_dispatch_function</a>(store);
        <a href="function_info.md#0x1_function_info_load_function">function_info::load_function</a>(&func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_value">dispatchable_derived_value</a>(store, &func)
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(store)
    }
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_borrow_transfer_ref"></a>

## Function `borrow_transfer_ref`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): &TransferRef <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(
        &<a href="fungible_asset.md#0x1_fungible_asset_store_metadata">fungible_asset::store_metadata</a>(metadata)
    );
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a>&gt;(metadata_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND">ESTORE_NOT_FOUND</a>)
    );
    &<b>borrow_global</b>&lt;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a>&gt;(metadata_addr).transfer_ref
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer_fixed_send"></a>

## Function `transfer_fixed_send`

A transfer with a fixed amount debited from the sender


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_fixed_send">transfer_fixed_send</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, send_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_fixed_send">transfer_fixed_send</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    send_amount: u64,
) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>let</b> balance_before = <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(from);
    <b>let</b> fa = <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>(sender, from, send_amount);
    <b>assert</b>!(
        balance_before == <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(from) + send_amount,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>)
    );
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer_fixed_receive"></a>

## Function `transfer_fixed_receive`

A transfer with a fixed amount credited to the recipient


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_fixed_receive">transfer_fixed_receive</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, receive_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_fixed_receive">transfer_fixed_receive</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    receive_amount: u64,
) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>let</b> fa = <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>(sender, from, receive_amount);
    <b>let</b> balance_before = <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<b>to</b>);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
    <b>assert</b>!(
        balance_before + receive_amount == <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<b>to</b>),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>)
    );
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_withdraw"></a>

## Function `dispatchable_withdraw`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(owner: <b>address</b>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(
    owner: <b>address</b>, store: Object&lt;T&gt;,
    amount: u64,
    transfer_ref: &TransferRef,
    function: &FunctionInfo,
): FungibleAsset;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_deposit"></a>

## Function `dispatchable_deposit`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    fa: FungibleAsset,
    transfer_ref: &TransferRef,
    function: &FunctionInfo,
);
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_derived_value"></a>

## Function `dispatchable_derived_value`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_value">dispatchable_derived_value</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_value">dispatchable_derived_value</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    function: &FunctionInfo,
): u64;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
