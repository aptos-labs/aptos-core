
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

The api listed here intended to be an in-place replacement for defi applications that uses fungible_asset api directly
and is safe for non-dispatchable (aka vanilla) fungible assets as well.

See AIP-73 for further discussion


-  [Resource `TransferRefStore`](#0x1_dispatchable_fungible_asset_TransferRefStore)
-  [Constants](#@Constants_0)
-  [Function `register_dispatch_functions`](#0x1_dispatchable_fungible_asset_register_dispatch_functions)
-  [Function `register_derive_supply_dispatch_function`](#0x1_dispatchable_fungible_asset_register_derive_supply_dispatch_function)
-  [Function `withdraw`](#0x1_dispatchable_fungible_asset_withdraw)
-  [Function `deposit`](#0x1_dispatchable_fungible_asset_deposit)
-  [Function `transfer`](#0x1_dispatchable_fungible_asset_transfer)
-  [Function `transfer_assert_minimum_deposit`](#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit)
-  [Function `derived_balance`](#0x1_dispatchable_fungible_asset_derived_balance)
-  [Function `is_derived_balance_at_least`](#0x1_dispatchable_fungible_asset_is_derived_balance_at_least)
-  [Function `derived_supply`](#0x1_dispatchable_fungible_asset_derived_supply)
-  [Function `borrow_transfer_ref`](#0x1_dispatchable_fungible_asset_borrow_transfer_ref)
-  [Function `dispatchable_withdraw`](#0x1_dispatchable_fungible_asset_dispatchable_withdraw)
-  [Function `dispatchable_deposit`](#0x1_dispatchable_fungible_asset_dispatchable_deposit)
-  [Function `dispatchable_derived_balance`](#0x1_dispatchable_fungible_asset_dispatchable_derived_balance)
-  [Function `dispatchable_derived_supply`](#0x1_dispatchable_fungible_asset_dispatchable_derived_supply)
-  [Specification](#@Specification_1)
    -  [Function `withdraw`](#@Specification_1_withdraw)
    -  [Function `deposit`](#@Specification_1_deposit)
    -  [Function `dispatchable_withdraw`](#@Specification_1_dispatchable_withdraw)
    -  [Function `dispatchable_deposit`](#@Specification_1_dispatchable_deposit)
    -  [Function `dispatchable_derived_balance`](#@Specification_1_dispatchable_derived_balance)
    -  [Function `dispatchable_derived_supply`](#@Specification_1_dispatchable_derived_supply)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
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



<a id="0x1_dispatchable_fungible_asset_ENOT_LOADED"></a>

Dispatch target is not loaded.


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_LOADED">ENOT_LOADED</a>: u64 = 4;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND"></a>

TransferRefStore doesn't exist on the fungible asset type.


<pre><code><b>const</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND">ESTORE_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`



<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, withdraw_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, deposit_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;, derived_balance_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_dispatch_functions">register_dispatch_functions</a>(
    constructor_ref: &ConstructorRef,
    withdraw_function: Option&lt;FunctionInfo&gt;,
    deposit_function: Option&lt;FunctionInfo&gt;,
    derived_balance_function: Option&lt;FunctionInfo&gt;,
) {
    <a href="fungible_asset.md#0x1_fungible_asset_register_dispatch_functions">fungible_asset::register_dispatch_functions</a>(
        constructor_ref,
        withdraw_function,
        deposit_function,
        derived_balance_function,
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

<a id="0x1_dispatchable_fungible_asset_register_derive_supply_dispatch_function"></a>

## Function `register_derive_supply_dispatch_function`



<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_derive_supply_dispatch_function">register_derive_supply_dispatch_function</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, dispatch_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_register_derive_supply_dispatch_function">register_derive_supply_dispatch_function</a>(
    constructor_ref: &ConstructorRef,
    dispatch_function: Option&lt;FunctionInfo&gt;
) {
    <a href="fungible_asset.md#0x1_fungible_asset_register_derive_supply_dispatch_function">fungible_asset::register_derive_supply_dispatch_function</a>(
        constructor_ref,
        dispatch_function
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
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    store: Object&lt;T&gt;,
    amount: u64,
): FungibleAsset <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_sanity_check">fungible_asset::withdraw_sanity_check</a>(owner, store, <b>false</b>);
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check">fungible_asset::withdraw_permission_check</a>(owner, store, amount);
    <b>let</b> func_opt = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_dispatch_function">fungible_asset::withdraw_dispatch_function</a>(store);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&func_opt)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&func_opt);
        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);
        <b>let</b> fa = <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>(
            store,
            amount,
            <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>(store),
            func,
        );
        fa
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_unchecked_withdraw">fungible_asset::unchecked_withdraw</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), amount)
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
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_sanity_check">fungible_asset::deposit_sanity_check</a>(store, <b>false</b>);
    <b>let</b> func_opt = <a href="fungible_asset.md#0x1_fungible_asset_deposit_dispatch_function">fungible_asset::deposit_dispatch_function</a>(store);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&func_opt)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&func_opt);
        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>(
            store,
            fa,
            <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_borrow_transfer_ref">borrow_transfer_ref</a>(store),
            func
        )
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_unchecked_deposit">fungible_asset::unchecked_deposit</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&store), fa)
    }
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>let</b> fa = <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>(sender, from, amount);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit"></a>

## Function `transfer_assert_minimum_deposit`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
The recipient is guranteed to receive asset greater than the expected amount.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit">transfer_assert_minimum_deposit</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, expected: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit">transfer_assert_minimum_deposit</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    amount: u64,
    expected: u64
) <b>acquires</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_TransferRefStore">TransferRefStore</a> {
    <b>let</b> start = <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<b>to</b>);
    <b>let</b> fa = <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>(sender, from, amount);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
    <b>let</b> end = <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<b>to</b>);
    <b>assert</b>!(end - start &gt;= expected, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH">EAMOUNT_MISMATCH</a>));
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_derived_balance"></a>

## Function `derived_balance`

Get the derived value of store using the overloaded hook.

The semantics of value will be governed by the function specified in DispatchFunctionStore.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_balance">derived_balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_balance">derived_balance</a>&lt;T: key&gt;(store: Object&lt;T&gt;): u64 {
    <b>let</b> func_opt = <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">fungible_asset::derived_balance_dispatch_function</a>(store);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&func_opt)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&func_opt);
        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>(store, func)
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(store)
    }
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_is_derived_balance_at_least"></a>

## Function `is_derived_balance_at_least`

Whether the derived value of store using the overloaded hook is at least <code>amount</code>

The semantics of value will be governed by the function specified in DispatchFunctionStore.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_is_derived_balance_at_least">is_derived_balance_at_least</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_is_derived_balance_at_least">is_derived_balance_at_least</a>&lt;T: key&gt;(store: Object&lt;T&gt;, amount: u64): bool {
    <b>let</b> func_opt = <a href="fungible_asset.md#0x1_fungible_asset_derived_balance_dispatch_function">fungible_asset::derived_balance_dispatch_function</a>(store);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&func_opt)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&func_opt);
        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>(store, func) &gt;= amount
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_is_balance_at_least">fungible_asset::is_balance_at_least</a>(store, amount)
    }
}
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_derived_supply"></a>

## Function `derived_supply`

Get the derived supply of the fungible asset using the overloaded hook.

The semantics of supply will be governed by the function specified in DeriveSupplyDispatch.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_supply">derived_supply</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_supply">derived_supply</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u128&gt; {
    <b>let</b> func_opt = <a href="fungible_asset.md#0x1_fungible_asset_derived_supply_dispatch_function">fungible_asset::derived_supply_dispatch_function</a>(metadata);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&func_opt)) {
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
        );
        <b>let</b> func = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&func_opt);
        <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(func);
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_supply">dispatchable_derived_supply</a>(metadata, func)
    } <b>else</b> {
        <a href="fungible_asset.md#0x1_fungible_asset_supply">fungible_asset::supply</a>(metadata)
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

<a id="0x1_dispatchable_fungible_asset_dispatchable_withdraw"></a>

## Function `dispatchable_withdraw`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
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

<a id="0x1_dispatchable_fungible_asset_dispatchable_derived_balance"></a>

## Function `dispatchable_derived_balance`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    function: &FunctionInfo,
): u64;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_derived_supply"></a>

## Function `dispatchable_derived_supply`



<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_supply">dispatchable_derived_supply</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_supply">dispatchable_derived_supply</a>&lt;T: key&gt;(
    metadata: Object&lt;T&gt;,
    function: &FunctionInfo,
): Option&lt;u128&gt;;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_withdraw"></a>

### Function `withdraw`


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">permissioned_signer::PermissionStorage</a>&gt;(<a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">permissioned_signer::spec_permission_address</a>(owner));
<b>modifies</b> <b>global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(store));
<b>modifies</b> <b>global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">fungible_asset::ConcurrentFungibleBalance</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(store));
</code></pre>



<a id="@Specification_1_deposit"></a>

### Function `deposit`


<pre><code><b>public</b> <b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(store));
<b>modifies</b> <b>global</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_ConcurrentFungibleBalance">fungible_asset::ConcurrentFungibleBalance</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(store));
</code></pre>



<a id="@Specification_1_dispatchable_withdraw"></a>

### Function `dispatchable_withdraw`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_withdraw">dispatchable_withdraw</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_dispatchable_deposit"></a>

### Function `dispatchable_deposit`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_deposit">dispatchable_deposit</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_dispatchable_derived_balance"></a>

### Function `dispatchable_derived_balance`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_balance">dispatchable_derived_balance</a>&lt;T: key&gt;(store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_dispatchable_derived_supply"></a>

### Function `dispatchable_derived_supply`


<pre><code><b>fun</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_dispatchable_derived_supply">dispatchable_derived_supply</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
