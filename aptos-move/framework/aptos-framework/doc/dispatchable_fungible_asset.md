
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


<pre><code>use 0x1::error;
use 0x1::features;
use 0x1::function_info;
use 0x1::fungible_asset;
use 0x1::object;
use 0x1::option;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_TransferRefStore"></a>

## Resource `TransferRefStore`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct TransferRefStore has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>transfer_ref: fungible_asset::TransferRef</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_dispatchable_fungible_asset_ENOT_ACTIVATED"></a>

Feature is not activated yet on the network.


<pre><code>const ENOT_ACTIVATED: u64 &#61; 3;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH"></a>

Recipient is not getting the guaranteed value;


<pre><code>const EAMOUNT_MISMATCH: u64 &#61; 2;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_ENOT_LOADED"></a>

Dispatch target is not loaded.


<pre><code>const ENOT_LOADED: u64 &#61; 4;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND"></a>

TransferRefStore doesn't exist on the fungible asset type.


<pre><code>const ESTORE_NOT_FOUND: u64 &#61; 1;
</code></pre>



<a id="0x1_dispatchable_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`



<pre><code>public fun register_dispatch_functions(constructor_ref: &amp;object::ConstructorRef, withdraw_function: option::Option&lt;function_info::FunctionInfo&gt;, deposit_function: option::Option&lt;function_info::FunctionInfo&gt;, derived_balance_function: option::Option&lt;function_info::FunctionInfo&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun register_dispatch_functions(
    constructor_ref: &amp;ConstructorRef,
    withdraw_function: Option&lt;FunctionInfo&gt;,
    deposit_function: Option&lt;FunctionInfo&gt;,
    derived_balance_function: Option&lt;FunctionInfo&gt;,
) &#123;
    fungible_asset::register_dispatch_functions(
        constructor_ref,
        withdraw_function,
        deposit_function,
        derived_balance_function,
    );
    let store_obj &#61; &amp;object::generate_signer(constructor_ref);
    move_to&lt;TransferRefStore&gt;(
        store_obj,
        TransferRefStore &#123;
            transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.

The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code>public fun withdraw&lt;T: key&gt;(owner: &amp;signer, store: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;T: key&gt;(
    owner: &amp;signer,
    store: Object&lt;T&gt;,
    amount: u64,
): FungibleAsset acquires TransferRefStore &#123;
    fungible_asset::withdraw_sanity_check(owner, store, false);
    let func_opt &#61; fungible_asset::withdraw_dispatch_function(store);
    if (option::is_some(&amp;func_opt)) &#123;
        assert!(
            features::dispatchable_fungible_asset_enabled(),
            error::aborted(ENOT_ACTIVATED)
        );
        let start_balance &#61; fungible_asset::balance(store);
        let func &#61; option::borrow(&amp;func_opt);
        function_info::load_module_from_function(func);
        let fa &#61; dispatchable_withdraw(
            store,
            amount,
            borrow_transfer_ref(store),
            func,
        );
        let end_balance &#61; fungible_asset::balance(store);
        assert!(amount &lt;&#61; start_balance &#45; end_balance, error::aborted(EAMOUNT_MISMATCH));
        fa
    &#125; else &#123;
        fungible_asset::withdraw_internal(object::object_address(&amp;store), amount)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.

The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code>public fun deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) acquires TransferRefStore &#123;
    fungible_asset::deposit_sanity_check(store, false);
    let func_opt &#61; fungible_asset::deposit_dispatch_function(store);
    if (option::is_some(&amp;func_opt)) &#123;
        assert!(
            features::dispatchable_fungible_asset_enabled(),
            error::aborted(ENOT_ACTIVATED)
        );
        let func &#61; option::borrow(&amp;func_opt);
        function_info::load_module_from_function(func);
        dispatchable_deposit(
            store,
            fa,
            borrow_transfer_ref(store),
            func
        )
    &#125; else &#123;
        fungible_asset::deposit_internal(store, fa)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer"></a>

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
) acquires TransferRefStore &#123;
    let fa &#61; withdraw(sender, from, amount);
    deposit(to, fa);
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit"></a>

## Function `transfer_assert_minimum_deposit`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
The recipient is guranteed to receive asset greater than the expected amount.
Note: it does not move the underlying object.


<pre><code>public entry fun transfer_assert_minimum_deposit&lt;T: key&gt;(sender: &amp;signer, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64, expected: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_assert_minimum_deposit&lt;T: key&gt;(
    sender: &amp;signer,
    from: Object&lt;T&gt;,
    to: Object&lt;T&gt;,
    amount: u64,
    expected: u64
) acquires TransferRefStore &#123;
    let start &#61; fungible_asset::balance(to);
    let fa &#61; withdraw(sender, from, amount);
    deposit(to, fa);
    let end &#61; fungible_asset::balance(to);
    assert!(end &#45; start &gt;&#61; expected, error::aborted(EAMOUNT_MISMATCH));
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_derived_balance"></a>

## Function `derived_balance`

Get the derived value of store using the overloaded hook.

The semantics of value will be governed by the function specified in DispatchFunctionStore.


<pre><code>&#35;[view]
public fun derived_balance&lt;T: key&gt;(store: object::Object&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun derived_balance&lt;T: key&gt;(store: Object&lt;T&gt;): u64 &#123;
    let func_opt &#61; fungible_asset::derived_balance_dispatch_function(store);
    if (option::is_some(&amp;func_opt)) &#123;
        assert!(
            features::dispatchable_fungible_asset_enabled(),
            error::aborted(ENOT_ACTIVATED)
        );
        let func &#61; option::borrow(&amp;func_opt);
        function_info::load_module_from_function(func);
        dispatchable_derived_balance(store, func)
    &#125; else &#123;
        fungible_asset::balance(store)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_borrow_transfer_ref"></a>

## Function `borrow_transfer_ref`



<pre><code>fun borrow_transfer_ref&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): &amp;fungible_asset::TransferRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_transfer_ref&lt;T: key&gt;(metadata: Object&lt;T&gt;): &amp;TransferRef acquires TransferRefStore &#123;
    let metadata_addr &#61; object::object_address(
        &amp;fungible_asset::store_metadata(metadata)
    );
    assert!(
        exists&lt;TransferRefStore&gt;(metadata_addr),
        error::not_found(ESTORE_NOT_FOUND)
    );
    &amp;borrow_global&lt;TransferRefStore&gt;(metadata_addr).transfer_ref
&#125;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_withdraw"></a>

## Function `dispatchable_withdraw`



<pre><code>fun dispatchable_withdraw&lt;T: key&gt;(store: object::Object&lt;T&gt;, amount: u64, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo): fungible_asset::FungibleAsset
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun dispatchable_withdraw&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    amount: u64,
    transfer_ref: &amp;TransferRef,
    function: &amp;FunctionInfo,
): FungibleAsset;
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_deposit"></a>

## Function `dispatchable_deposit`



<pre><code>fun dispatchable_deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun dispatchable_deposit&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    fa: FungibleAsset,
    transfer_ref: &amp;TransferRef,
    function: &amp;FunctionInfo,
);
</code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_derived_balance"></a>

## Function `dispatchable_derived_balance`



<pre><code>fun dispatchable_derived_balance&lt;T: key&gt;(store: object::Object&lt;T&gt;, function: &amp;function_info::FunctionInfo): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun dispatchable_derived_balance&lt;T: key&gt;(
    store: Object&lt;T&gt;,
    function: &amp;FunctionInfo,
): u64;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_dispatchable_withdraw"></a>

### Function `dispatchable_withdraw`


<pre><code>fun dispatchable_withdraw&lt;T: key&gt;(store: object::Object&lt;T&gt;, amount: u64, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo): fungible_asset::FungibleAsset
</code></pre>




<pre><code>pragma opaque;
</code></pre>



<a id="@Specification_1_dispatchable_deposit"></a>

### Function `dispatchable_deposit`


<pre><code>fun dispatchable_deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo)
</code></pre>




<pre><code>pragma opaque;
</code></pre>



<a id="@Specification_1_dispatchable_derived_balance"></a>

### Function `dispatchable_derived_balance`


<pre><code>fun dispatchable_derived_balance&lt;T: key&gt;(store: object::Object&lt;T&gt;, function: &amp;function_info::FunctionInfo): u64
</code></pre>




<pre><code>pragma opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
