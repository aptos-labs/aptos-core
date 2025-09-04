
<a id="0x1_overloadable_fungible_asset"></a>

# Module `0x1::overloadable_fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code>Metadata</code> object. The
metadata object can be any object that equipped with <code>Metadata</code> resource.

The overloadable_fungible_asset wraps the existing fungible_asset module and adds the ability for token issuer
to customize the logic for withdraw and deposit operations. For example:

- Deflation token: a fixed percentage of token will be destructed upon transfer.
- Transfer allowlist: token can only be transfered to addresses in the allow list.
- Predicated transfer: transfer can only happen when some certain predicate has been met.
- Loyalty token: a fixed loyalty will be paid to a designated address when a fungible asset transfer happens

See AIP-73 for further discussion


-  [Resource `OverloadFunctionStore`](#0x1_overloadable_fungible_asset_OverloadFunctionStore)
-  [Constants](#@Constants_0)
-  [Function `register_overload_functions`](#0x1_overloadable_fungible_asset_register_overload_functions)
-  [Function `withdraw`](#0x1_overloadable_fungible_asset_withdraw)
-  [Function `deposit`](#0x1_overloadable_fungible_asset_deposit)
-  [Function `transfer_fixed_send`](#0x1_overloadable_fungible_asset_transfer_fixed_send)
-  [Function `transfer_fixed_receive`](#0x1_overloadable_fungible_asset_transfer_fixed_receive)
-  [Function `dispatchable_withdraw`](#0x1_overloadable_fungible_asset_dispatchable_withdraw)
-  [Function `dispatchable_deposit`](#0x1_overloadable_fungible_asset_dispatchable_deposit)


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_overloadable_fungible_asset_OverloadFunctionStore"></a>

## Resource `OverloadFunctionStore`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>withdraw_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>transfer_ref: <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_overloadable_fungible_asset_EALREADY_REGISTERED"></a>

Trying to register overload functions to fungible asset that has already been initialized with custom transfer function.


<pre><code><b>const</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>: u64 = 5;
</code></pre>



<a id="0x1_overloadable_fungible_asset_EOBJECT_IS_DELETABLE"></a>

Fungibility is only available for non-deletable objects.


<pre><code><b>const</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>: u64 = 18;
</code></pre>



<a id="0x1_overloadable_fungible_asset_EAMOUNT_MISMATCH"></a>

Recipient is not getting the guaranteed value;


<pre><code><b>const</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>: u64 = 4;
</code></pre>



<a id="0x1_overloadable_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided deposit function type doesn't meet the signature requirement.


<pre><code><b>const</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH">EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 2;
</code></pre>



<a id="0x1_overloadable_fungible_asset_EFUNCTION_STORE_NOT_FOUND"></a>

Calling overloadable api on non-overloadable fungible asset store.


<pre><code><b>const</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EFUNCTION_STORE_NOT_FOUND">EFUNCTION_STORE_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x1_overloadable_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH"></a>

Provided withdraw function type doesn't meet the signature requirement.


<pre><code><b>const</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH">EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_overloadable_fungible_asset_register_overload_functions"></a>

## Function `register_overload_functions`

Create a fungible asset store whose transfer rule would be overloaded by the provided function.


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_register_overload_functions">register_overload_functions</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, withdraw_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, deposit_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_register_overload_functions">register_overload_functions</a>(
    constructor_ref: &ConstructorRef,
    withdraw_function: FunctionInfo,
		deposit_function: FunctionInfo,
) {
    <b>let</b> dispatcher_withdraw_function_info = <a href="function_info.md#0x1_function_info_new_function_info">function_info::new_function_info</a>(
	        @velor_framework,
        <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset">overloadable_fungible_asset</a>"),
        <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_withdraw"),
    );
    // Verify that caller type matches callee type so wrongly typed function cannot be registered.
    <b>assert</b>!(<a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(
        &dispatcher_withdraw_function_info,
        &withdraw_function
    ), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH">EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH</a>));

    <b>let</b> dispatcher_deposit_function_info = <a href="function_info.md#0x1_function_info_new_function_info">function_info::new_function_info</a>(
	        @velor_framework,
        <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset">overloadable_fungible_asset</a>"),
        <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_deposit"),
    );
    // Verify that caller type matches callee type so wrongly typed function cannot be registered.
    <b>assert</b>!(<a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(
        &dispatcher_deposit_function_info,
        &deposit_function
    ), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH">EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH</a>));

    <b>assert</b>!(!<a href="object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(constructor_ref), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EOBJECT_IS_DELETABLE">EOBJECT_IS_DELETABLE</a>));
    <b>assert</b>!(!<b>exists</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(<a href="object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(constructor_ref)), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EALREADY_REGISTERED">EALREADY_REGISTERED</a>));

    // Freeze the FungibleStore <b>to</b> force usign the new overloaded api.
    <b>let</b> extend_ref = <a href="object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(constructor_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_set_global_frozen_flag">fungible_asset::set_global_frozen_flag</a>(&extend_ref, <b>true</b>);

    <b>let</b> store_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);

    // Store the overload function hook.
    <b>move_to</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(store_obj, <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a> {
        withdraw_function,
		    deposit_function,
        transfer_ref: <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">fungible_asset::generate_transfer_ref</a>(constructor_ref),
    });
}
</code></pre>



</details>

<a id="0x1_overloadable_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.

The semantics of deposit will be governed by the function specified in OverloadFunctionStore.


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(
    owner: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    store: Object&lt;T&gt;,
    amount: u64,
): FungibleAsset <b>acquires</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_store_metadata">fungible_asset::store_metadata</a>(store));
    <b>let</b> owner_address = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>assert</b>!(<b>exists</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EFUNCTION_STORE_NOT_FOUND">EFUNCTION_STORE_NOT_FOUND</a>));
    <b>let</b> overloadable_store = <b>borrow_global</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr);
    <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>(
        owner_address,
        store,
        amount,
        &overloadable_store.transfer_ref,
        &overloadable_store.withdraw_function,
    )
}
</code></pre>



</details>

<a id="0x1_overloadable_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.

The semantics of deposit will be governed by the function specified in OverloadFunctionStore.


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    fa: FungibleAsset
) <b>acquires</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_store_metadata">fungible_asset::store_metadata</a>(store));
    <b>assert</b>!(<b>exists</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EFUNCTION_STORE_NOT_FOUND">EFUNCTION_STORE_NOT_FOUND</a>));
    <b>let</b> overloadable_store = <b>borrow_global</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr);
    <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>(
        store,
        fa,
        &overloadable_store.transfer_ref,
        &overloadable_store.deposit_function,
    );
}
</code></pre>



</details>

<a id="0x1_overloadable_fungible_asset_transfer_fixed_send"></a>

## Function `transfer_fixed_send`

A transfer with a fixed amount debited from the sender


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_transfer_fixed_send">transfer_fixed_send</a>&lt;T: key&gt;(_sender: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, send_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_transfer_fixed_send">transfer_fixed_send</a>&lt;T: key&gt;(
    _sender: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    send_amount: u64,
) <b>acquires</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_store_metadata">fungible_asset::store_metadata</a>(from));
    <b>assert</b>!(<b>exists</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EFUNCTION_STORE_NOT_FOUND">EFUNCTION_STORE_NOT_FOUND</a>));
    <b>let</b> overloadable_store = <b>borrow_global</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr);
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">fungible_asset::withdraw_with_ref</a>(&overloadable_store.transfer_ref, from, send_amount);
    <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
}
</code></pre>



</details>

<a id="0x1_overloadable_fungible_asset_transfer_fixed_receive"></a>

## Function `transfer_fixed_receive`

A transfer with a fixed amount credited to the recipient


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_transfer_fixed_receive">transfer_fixed_receive</a>&lt;T: key&gt;(sender: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, receive_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_transfer_fixed_receive">transfer_fixed_receive</a>&lt;T: key&gt;(
    sender: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    receive_amount: u64,
) <b>acquires</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a> {
    <b>let</b> fa = <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_withdraw">withdraw</a>(sender, from, receive_amount);
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="fungible_asset.md#0x1_fungible_asset_store_metadata">fungible_asset::store_metadata</a>(from));
    <b>let</b> overloadable_store = <b>borrow_global</b>&lt;<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_OverloadFunctionStore">OverloadFunctionStore</a>&gt;(metadata_addr);
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_amount">fungible_asset::amount</a>(&fa) == receive_amount, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>));
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">fungible_asset::deposit_with_ref</a>(&overloadable_store.transfer_ref, <b>to</b>, fa);
}
</code></pre>



</details>

<a id="0x1_overloadable_fungible_asset_dispatchable_withdraw"></a>

## Function `dispatchable_withdraw`



<pre><code><b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(owner: <b>address</b>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(
    owner: <b>address</b>,
    store: Object&lt;T&gt;,
    amount: u64,
    transfer_ref: &TransferRef,
    function: &FunctionInfo,
): FungibleAsset;
</code></pre>



</details>

<a id="0x1_overloadable_fungible_asset_dispatchable_deposit"></a>

## Function `dispatchable_deposit`



<pre><code><b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="overloadable_fungible_asset.md#0x1_overloadable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    fa: FungibleAsset,
    transfer_ref: &TransferRef,
    function: &FunctionInfo,
);
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
