
<a id="0x1_fungible_asset_intent_hooks"></a>

# Module `0x1::fungible_asset_intent_hooks`



-  [Constants](#@Constants_0)
-  [Function `fa_to_fa_consumption`](#0x1_fungible_asset_intent_hooks_fa_to_fa_consumption)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent">0x1::fungible_asset_intent</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/hot_potato_any.md#0x1_hot_potato_any">0x1::hot_potato_any</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_fungible_asset_intent_hooks_EAMOUNT_NOT_MEET"></a>

The token offered does not meet amount requirement.


<pre><code><b>const</b> <a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks_EAMOUNT_NOT_MEET">EAMOUNT_NOT_MEET</a>: u64 = 1;
</code></pre>



<a id="0x1_fungible_asset_intent_hooks_ENOT_DESIRED_TOKEN"></a>

The token offered is not the desired fungible asset.


<pre><code><b>const</b> <a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks_ENOT_DESIRED_TOKEN">ENOT_DESIRED_TOKEN</a>: u64 = 0;
</code></pre>



<a id="0x1_fungible_asset_intent_hooks_fa_to_fa_consumption"></a>

## Function `fa_to_fa_consumption`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks_fa_to_fa_consumption">fa_to_fa_consumption</a>(target: <a href="../../aptos-stdlib/doc/hot_potato_any.md#0x1_hot_potato_any_Any">hot_potato_any::Any</a>, argument: <a href="../../aptos-stdlib/doc/hot_potato_any.md#0x1_hot_potato_any_Any">hot_potato_any::Any</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks_fa_to_fa_consumption">fa_to_fa_consumption</a>(target: Any, argument: Any) {
    <b>let</b> received_fa = <a href="../../aptos-stdlib/doc/hot_potato_any.md#0x1_hot_potato_any_unpack">hot_potato_any::unpack</a>&lt;FungibleAsset&gt;(target);
    <b>let</b> (desired_metadata, desired_amount, issuer) = <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_unpack_argument">fungible_asset_intent::unpack_argument</a>(
        <a href="../../aptos-stdlib/doc/hot_potato_any.md#0x1_hot_potato_any_unpack">hot_potato_any::unpack</a>&lt;FungibleAssetExchange&gt;(argument)
    );

    <b>assert</b>!(
        <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&received_fa) == desired_metadata,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks_ENOT_DESIRED_TOKEN">ENOT_DESIRED_TOKEN</a>)
    );
    <b>assert</b>!(
        <a href="fungible_asset.md#0x1_fungible_asset_amount">fungible_asset::amount</a>(&received_fa) &gt;= desired_amount,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks_EAMOUNT_NOT_MEET">EAMOUNT_NOT_MEET</a>),
    );

    <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">primary_fungible_store::deposit</a>(issuer, received_fa);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
