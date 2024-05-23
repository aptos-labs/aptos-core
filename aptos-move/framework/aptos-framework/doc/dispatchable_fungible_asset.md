
<a id="0x1_dispatchable_fungible_asset"></a>

# Module `0x1::dispatchable_fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code>Metadata</code> object. The<br/> metadata object can be any object that equipped with <code>Metadata</code> resource.<br/><br/> The dispatchable_fungible_asset wraps the existing fungible_asset module and adds the ability for token issuer<br/> to customize the logic for withdraw and deposit operations. For example:<br/><br/> &#45; Deflation token: a fixed percentage of token will be destructed upon transfer.<br/> &#45; Transfer allowlist: token can only be transfered to addresses in the allow list.<br/> &#45; Predicated transfer: transfer can only happen when some certain predicate has been met.<br/> &#45; Loyalty token: a fixed loyalty will be paid to a designated address when a fungible asset transfer happens<br/><br/> The api listed here intended to be an in&#45;place replacement for defi applications that uses fungible_asset api directly<br/> and is safe for non&#45;dispatchable (aka vanilla) fungible assets as well.<br/><br/> See AIP&#45;73 for further discussion<br/>


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


<pre><code>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::function_info;<br/>use 0x1::fungible_asset;<br/>use 0x1::object;<br/>use 0x1::option;<br/></code></pre>



<a id="0x1_dispatchable_fungible_asset_TransferRefStore"></a>

## Resource `TransferRefStore`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct TransferRefStore has key<br/></code></pre>



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


<pre><code>const ENOT_ACTIVATED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_dispatchable_fungible_asset_EAMOUNT_MISMATCH"></a>

Recipient is not getting the guaranteed value;


<pre><code>const EAMOUNT_MISMATCH: u64 &#61; 2;<br/></code></pre>



<a id="0x1_dispatchable_fungible_asset_ENOT_LOADED"></a>

Dispatch target is not loaded.


<pre><code>const ENOT_LOADED: u64 &#61; 4;<br/></code></pre>



<a id="0x1_dispatchable_fungible_asset_ESTORE_NOT_FOUND"></a>

TransferRefStore doesn&apos;t exist on the fungible asset type.


<pre><code>const ESTORE_NOT_FOUND: u64 &#61; 1;<br/></code></pre>



<a id="0x1_dispatchable_fungible_asset_register_dispatch_functions"></a>

## Function `register_dispatch_functions`



<pre><code>public fun register_dispatch_functions(constructor_ref: &amp;object::ConstructorRef, withdraw_function: option::Option&lt;function_info::FunctionInfo&gt;, deposit_function: option::Option&lt;function_info::FunctionInfo&gt;, derived_balance_function: option::Option&lt;function_info::FunctionInfo&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun register_dispatch_functions(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    withdraw_function: Option&lt;FunctionInfo&gt;,<br/>    deposit_function: Option&lt;FunctionInfo&gt;,<br/>    derived_balance_function: Option&lt;FunctionInfo&gt;,<br/>) &#123;<br/>    fungible_asset::register_dispatch_functions(<br/>        constructor_ref,<br/>        withdraw_function,<br/>        deposit_function,<br/>        derived_balance_function,<br/>    );<br/>    let store_obj &#61; &amp;object::generate_signer(constructor_ref);<br/>    move_to&lt;TransferRefStore&gt;(<br/>        store_obj,<br/>        TransferRefStore &#123;<br/>            transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of the fungible asset from <code>store</code> by the owner.<br/><br/> The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code>public fun withdraw&lt;T: key&gt;(owner: &amp;signer, store: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;T: key&gt;(<br/>    owner: &amp;signer,<br/>    store: Object&lt;T&gt;,<br/>    amount: u64,<br/>): FungibleAsset acquires TransferRefStore &#123;<br/>    fungible_asset::withdraw_sanity_check(owner, store, false);<br/>    let func_opt &#61; fungible_asset::withdraw_dispatch_function(store);<br/>    if (option::is_some(&amp;func_opt)) &#123;<br/>        assert!(<br/>            features::dispatchable_fungible_asset_enabled(),<br/>            error::aborted(ENOT_ACTIVATED)<br/>        );<br/>        let start_balance &#61; fungible_asset::balance(store);<br/>        let func &#61; option::borrow(&amp;func_opt);<br/>        function_info::load_module_from_function(func);<br/>        let fa &#61; dispatchable_withdraw(<br/>            store,<br/>            amount,<br/>            borrow_transfer_ref(store),<br/>            func,<br/>        );<br/>        let end_balance &#61; fungible_asset::balance(store);<br/>        assert!(amount &lt;&#61; start_balance &#45; end_balance, error::aborted(EAMOUNT_MISMATCH));<br/>        fa<br/>    &#125; else &#123;<br/>        fungible_asset::withdraw_internal(object::object_address(&amp;store), amount)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of the fungible asset to <code>store</code>.<br/><br/> The semantics of deposit will be governed by the function specified in DispatchFunctionStore.


<pre><code>public fun deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit&lt;T: key&gt;(store: Object&lt;T&gt;, fa: FungibleAsset) acquires TransferRefStore &#123;<br/>    fungible_asset::deposit_sanity_check(store, false);<br/>    let func_opt &#61; fungible_asset::deposit_dispatch_function(store);<br/>    if (option::is_some(&amp;func_opt)) &#123;<br/>        assert!(<br/>            features::dispatchable_fungible_asset_enabled(),<br/>            error::aborted(ENOT_ACTIVATED)<br/>        );<br/>        let func &#61; option::borrow(&amp;func_opt);<br/>        function_info::load_module_from_function(func);<br/>        dispatchable_deposit(<br/>            store,<br/>            fa,<br/>            borrow_transfer_ref(store),<br/>            func<br/>        )<br/>    &#125; else &#123;<br/>        fungible_asset::deposit_internal(store, fa)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer"></a>

## Function `transfer`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.<br/> Note: it does not move the underlying object.


<pre><code>public entry fun transfer&lt;T: key&gt;(sender: &amp;signer, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;T: key&gt;(<br/>    sender: &amp;signer,<br/>    from: Object&lt;T&gt;,<br/>    to: Object&lt;T&gt;,<br/>    amount: u64,<br/>) acquires TransferRefStore &#123;<br/>    let fa &#61; withdraw(sender, from, amount);<br/>    deposit(to, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit"></a>

## Function `transfer_assert_minimum_deposit`

Transfer an <code>amount</code> of fungible asset from <code>from_store</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.<br/> The recipient is guranteed to receive asset greater than the expected amount.<br/> Note: it does not move the underlying object.


<pre><code>public entry fun transfer_assert_minimum_deposit&lt;T: key&gt;(sender: &amp;signer, from: object::Object&lt;T&gt;, to: object::Object&lt;T&gt;, amount: u64, expected: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_assert_minimum_deposit&lt;T: key&gt;(<br/>    sender: &amp;signer,<br/>    from: Object&lt;T&gt;,<br/>    to: Object&lt;T&gt;,<br/>    amount: u64,<br/>    expected: u64<br/>) acquires TransferRefStore &#123;<br/>    let start &#61; fungible_asset::balance(to);<br/>    let fa &#61; withdraw(sender, from, amount);<br/>    deposit(to, fa);<br/>    let end &#61; fungible_asset::balance(to);<br/>    assert!(end &#45; start &gt;&#61; expected, error::aborted(EAMOUNT_MISMATCH));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_derived_balance"></a>

## Function `derived_balance`

Get the derived value of store using the overloaded hook.<br/><br/> The semantics of value will be governed by the function specified in DispatchFunctionStore.


<pre><code>&#35;[view]<br/>public fun derived_balance&lt;T: key&gt;(store: object::Object&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun derived_balance&lt;T: key&gt;(store: Object&lt;T&gt;): u64 &#123;<br/>    let func_opt &#61; fungible_asset::derived_balance_dispatch_function(store);<br/>    if (option::is_some(&amp;func_opt)) &#123;<br/>        assert!(<br/>            features::dispatchable_fungible_asset_enabled(),<br/>            error::aborted(ENOT_ACTIVATED)<br/>        );<br/>        let func &#61; option::borrow(&amp;func_opt);<br/>        function_info::load_module_from_function(func);<br/>        dispatchable_derived_balance(store, func)<br/>    &#125; else &#123;<br/>        fungible_asset::balance(store)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_borrow_transfer_ref"></a>

## Function `borrow_transfer_ref`



<pre><code>fun borrow_transfer_ref&lt;T: key&gt;(metadata: object::Object&lt;T&gt;): &amp;fungible_asset::TransferRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_transfer_ref&lt;T: key&gt;(metadata: Object&lt;T&gt;): &amp;TransferRef acquires TransferRefStore &#123;<br/>    let metadata_addr &#61; object::object_address(<br/>        &amp;fungible_asset::store_metadata(metadata)<br/>    );<br/>    assert!(<br/>        exists&lt;TransferRefStore&gt;(metadata_addr),<br/>        error::not_found(ESTORE_NOT_FOUND)<br/>    );<br/>    &amp;borrow_global&lt;TransferRefStore&gt;(metadata_addr).transfer_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_withdraw"></a>

## Function `dispatchable_withdraw`



<pre><code>fun dispatchable_withdraw&lt;T: key&gt;(store: object::Object&lt;T&gt;, amount: u64, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun dispatchable_withdraw&lt;T: key&gt;(<br/>    store: Object&lt;T&gt;,<br/>    amount: u64,<br/>    transfer_ref: &amp;TransferRef,<br/>    function: &amp;FunctionInfo,<br/>): FungibleAsset;<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_deposit"></a>

## Function `dispatchable_deposit`



<pre><code>fun dispatchable_deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun dispatchable_deposit&lt;T: key&gt;(<br/>    store: Object&lt;T&gt;,<br/>    fa: FungibleAsset,<br/>    transfer_ref: &amp;TransferRef,<br/>    function: &amp;FunctionInfo,<br/>);<br/></code></pre>



</details>

<a id="0x1_dispatchable_fungible_asset_dispatchable_derived_balance"></a>

## Function `dispatchable_derived_balance`



<pre><code>fun dispatchable_derived_balance&lt;T: key&gt;(store: object::Object&lt;T&gt;, function: &amp;function_info::FunctionInfo): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun dispatchable_derived_balance&lt;T: key&gt;(<br/>    store: Object&lt;T&gt;,<br/>    function: &amp;FunctionInfo,<br/>): u64;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_dispatchable_withdraw"></a>

### Function `dispatchable_withdraw`


<pre><code>fun dispatchable_withdraw&lt;T: key&gt;(store: object::Object&lt;T&gt;, amount: u64, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo): fungible_asset::FungibleAsset<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_dispatchable_deposit"></a>

### Function `dispatchable_deposit`


<pre><code>fun dispatchable_deposit&lt;T: key&gt;(store: object::Object&lt;T&gt;, fa: fungible_asset::FungibleAsset, transfer_ref: &amp;fungible_asset::TransferRef, function: &amp;function_info::FunctionInfo)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_dispatchable_derived_balance"></a>

### Function `dispatchable_derived_balance`


<pre><code>fun dispatchable_derived_balance&lt;T: key&gt;(store: object::Object&lt;T&gt;, function: &amp;function_info::FunctionInfo): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
