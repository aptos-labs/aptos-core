
<a id="0x1_dispatchable_fungible_asset"></a>

# Module `0x1::dispatchable_fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code>Metadata</code> object. The
metadata object can be any object that equipped with <code>Metadata</code> resource.

The dispatchable_fungible_asset wraps the existing fungible_asset module and adds the ability for token issuer
to customize the logic for withdraw and deposit operations. For example:

&#45; Deflation token: a fixed percentage of token will be destructed upon transfer.
&#45; Transfer allowlist: token can only be transfered to addresses in the allow list.
&#45; Predicated transfer: transfer can only happen when some certain predicate has been met.
&#45; Loyalty token: a fixed loyalty will be paid to a designated address when a fungible asset transfer happens

The api listed here intended to be an in&#45;place replacement for defi applications that uses fungible_asset api directly
and is safe for non&#45;dispatchable (aka vanilla) fungible assets as well.

See AIP&#45;73 for further discussion


-  [Resource `TransferRefStore`](#0x1_dispatchable_fungible_asset_TransferRefStore)
-  [Constants](#@Constants_0)
-  [Function `register_dispatch_functions`](#0x1_dispatchable_fungible_asset_register_dispatch_functions)
-  [Function `withdraw`](#0x1_dispatchable_fungible_asset_withdraw)
-  [Function `deposit`](#0x1_dispatchable_fungible_asset_deposit)
-  [Function `transfer`](#0x1_dispatchable_fungible_asset_transfer)
-  [Function `transfer_assert_minimum_deposit`](#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit)
-  [Function `derived_balance`](#0x1_dispatchable_fungible_asset_derived_balance)
-  [Function `borrow_transfer_ref`](#0x1_dispatchable_fungible_asset_borrow_transfer_ref)
-  [Function `dispatchable_withdraw`](#0x1_dispatchable_fungible_asset_dispatchable_withdraw)
-  [Function `dispatchable_deposit`](#0x1_dispatchable_fungible_asset_dispatchable_deposit)
-  [Function `dispatchable_derived_balance`](#0x1_dispatchable_fungible_asset_dispatchable_derived_balance)
-  [Specification](#@Specification_1)
    -  [Function `dispatchable_withdraw`](#@Specification_1_dispatchable_withdraw)
    -  [Function `dispatchable_deposit`](#@Specification_1_dispatchable_deposit)
    -  [Function `dispatchable_derived_balance`](#@Specification_1_dispatchable_derived_balance)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;<br /><b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;<br /><b>use</b> <a href="object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /></code></pre>



<a id="0x1_dispatchable_fungible_asset_TransferRefStore"></a>

## Resource `TransferRefStore`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> <b>has</b> key<br /></code></pre>



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


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH"></a>

Recipient is not getting the guaranteed value;


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_dispatchable_fungible_asset_ENOT_LOADED"></a>

Dispatch target is not loaded.


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_LOADED">ENOT_LOADED</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND"></a>

TransferRefStore doesn&apos;t exist on the fungible asset type.


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND">ESTORE_NOT_FOUND</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_dispatchable_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`



<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(constructor_ref: &amp;<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, withdraw_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, deposit_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, derived_balance_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(<br />    constructor_ref: &amp;ConstructorRef,<br />    withdraw_function: Option&lt;FunctionInfo&gt;,<br />    deposit_function: Option&lt;FunctionInfo&gt;,<br />    derived_balance_function: Option&lt;FunctionInfo&gt;,<br />) &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">fungible_asset::register_dispatch_functions</a>(<br />        constructor_ref,<br />        withdraw_function,<br />        deposit_function,<br />        derived_balance_function,<br />    );<br />    <b>let</b> store_obj &#61; &amp;<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);<br />    <b>move_to</b>&lt;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a>&gt;(<br />        store_obj,<br />        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> &#123;<br />            transfer_ref: <a href="fungible_asset.md#0x1_fungible_asset_generate_transfer_ref">fungible_asset::generate_transfer_ref</a>(constructor_ref),<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.

The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(<br />    owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    store: Object&lt;T&gt;,<br />    amount: u64,<br />): FungibleAsset <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">fungible_asset::withdraw_sanity_check</a>(owner, store, <b>false</b>);<br />    <b>let</b> func_opt &#61; <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">fungible_asset::withdraw_dispatch_function</a>(store);<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;func_opt)) &#123;<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)<br />        );<br />        <b>let</b> start_balance &#61; <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(store);<br />        <b>let</b> func &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;func_opt);<br />        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);<br />        <b>let</b> fa &#61; <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>(<br />            store,<br />            amount,<br />            <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>(store),<br />            func,<br />        );<br />        <b>let</b> end_balance &#61; <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(store);<br />        <b>assert</b>!(amount &lt;&#61; start_balance &#45; end_balance, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>));<br />        fa<br />    &#125; <b>else</b> &#123;<br />        <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">fungible_asset::withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store), amount)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.

The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> &#123;<br />    <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">fungible_asset::deposit_sanity_check</a>(store, <b>false</b>);<br />    <b>let</b> func_opt &#61; <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">fungible_asset::deposit_dispatch_function</a>(store);<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;func_opt)) &#123;<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)<br />        );<br />        <b>let</b> func &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;func_opt);<br />        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);<br />        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>(<br />            store,<br />            fa,<br />            <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>(store),<br />            func<br />        )<br />    &#125; <b>else</b> &#123;<br />        <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">fungible_asset::deposit_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;store), fa)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(sender: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(<br />    sender: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    from: Object&lt;T&gt;,<br />    <b>to</b>: Object&lt;T&gt;,<br />    amount: u64,<br />) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> &#123;<br />    <b>let</b> fa &#61; <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>(sender, from, amount);<br />    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit"></a>

## Function `transfer_assert_minimum_deposit`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
The recipient is guranteed to receive asset greater than the expected amount.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit">transfer_assert_minimum_deposit</a>&lt;T: key&gt;(sender: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, expected: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit">transfer_assert_minimum_deposit</a>&lt;T: key&gt;(<br />    sender: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    from: Object&lt;T&gt;,<br />    <b>to</b>: Object&lt;T&gt;,<br />    amount: u64,<br />    expected: u64<br />) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> &#123;<br />    <b>let</b> start &#61; <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<b>to</b>);<br />    <b>let</b> fa &#61; <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>(sender, from, amount);<br />    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);<br />    <b>let</b> end &#61; <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<b>to</b>);<br />    <b>assert</b>!(end &#45; start &gt;&#61; expected, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_derived_balance"></a>

## Function `derived_balance`

Get the derived value of store using the overloaded hook.

The semantics of value will be governed by the function specified in DispatchFunctionStore.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_balance">derived_balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_balance">derived_balance</a>&lt;T: key&gt;(store: Object&lt;T&gt;): u64 &#123;<br />    <b>let</b> func_opt &#61; <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">fungible_asset::derived_balance_dispatch_function</a>(store);<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;func_opt)) &#123;<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)<br />        );<br />        <b>let</b> func &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;func_opt);<br />        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);<br />        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>(store, func)<br />    &#125; <b>else</b> &#123;<br />        <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(store)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_borrow_transfer_ref"></a>

## Function `borrow_transfer_ref`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): &amp;TransferRef <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> &#123;<br />    <b>let</b> metadata_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(<br />        &amp;<a href="fungible_asset.md#0x1_fungible_asset_store_metadata">fungible_asset::store_metadata</a>(metadata)<br />    );<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a>&gt;(metadata_addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND">ESTORE_NOT_FOUND</a>)<br />    );<br />    &amp;<b>borrow_global</b>&lt;<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a>&gt;(metadata_addr).transfer_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_withdraw"></a>

## Function `dispatchable_withdraw`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, transfer_ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(<br />    store: Object&lt;T&gt;,<br />    amount: u64,<br />    transfer_ref: &amp;TransferRef,<br />    function: &amp;FunctionInfo,<br />): FungibleAsset;<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_deposit"></a>

## Function `dispatchable_deposit`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, transfer_ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(<br />    store: Object&lt;T&gt;,<br />    fa: FungibleAsset,<br />    transfer_ref: &amp;TransferRef,<br />    function: &amp;FunctionInfo,<br />);<br /></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_derived_balance"></a>

## Function `dispatchable_derived_balance`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>&lt;T: key&gt;(<br />    store: Object&lt;T&gt;,<br />    function: &amp;FunctionInfo,<br />): u64;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_dispatchable_withdraw"></a>

### Function `dispatchable_withdraw`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, transfer_ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_1_dispatchable_deposit"></a>

### Function `dispatchable_deposit`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, transfer_ref: &amp;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_1_dispatchable_derived_balance"></a>

### Function `dispatchable_derived_balance`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
