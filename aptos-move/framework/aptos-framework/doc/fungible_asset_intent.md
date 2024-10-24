
<a id="0x1_fungible_asset_intent"></a>

# Module `0x1::fungible_asset_intent`



-  [Struct `FungibleStoreManager`](#0x1_fungible_asset_intent_FungibleStoreManager)
-  [Struct `FungibleAssetExchange`](#0x1_fungible_asset_intent_FungibleAssetExchange)
-  [Function `create_fa_to_fa_intent`](#0x1_fungible_asset_intent_create_fa_to_fa_intent)
-  [Function `start_fa_to_fa_session`](#0x1_fungible_asset_intent_start_fa_to_fa_session)
-  [Function `finish_fa_to_fa_session`](#0x1_fungible_asset_intent_finish_fa_to_fa_session)
-  [Function `unpack_argument`](#0x1_fungible_asset_intent_unpack_argument)


<pre><code><b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="intent.md#0x1_intent">0x1::intent</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_fungible_asset_intent_FungibleStoreManager"></a>

## Struct `FungibleStoreManager`



<pre><code><b>struct</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>extend_ref: <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>

</dd>
<dt>
<code>delete_ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_intent_FungibleAssetExchange"></a>

## Struct `FungibleAssetExchange`



<pre><code><b>struct</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>desired_metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>desired_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>issuer: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_fungible_asset_intent_create_fa_to_fa_intent"></a>

## Function `create_fa_to_fa_intent`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_create_fa_to_fa_intent">create_fa_to_fa_intent</a>(source_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, desired_metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, desired_amount: u64, expiry_time: u64, issuer: <b>address</b>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">fungible_asset_intent::FungibleStoreManager</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_create_fa_to_fa_intent">create_fa_to_fa_intent</a>(
    source_fungible_asset: FungibleAsset,
    desired_metadata: Object&lt;Metadata&gt;,
    desired_amount: u64,
    expiry_time: u64,
    issuer: <b>address</b>,
): Object&lt;TradeIntent&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a>, FungibleAsset, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;&gt; {
    <b>let</b> coin_store_ref = <a href="object.md#0x1_object_create_self_owned_object">object::create_self_owned_object</a>();
    <b>let</b> extend_ref = <a href="object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(&coin_store_ref);
    <b>let</b> delete_ref = <a href="object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(&coin_store_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_create_store">fungible_asset::create_store</a>(&coin_store_ref, <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&source_fungible_asset));
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(
        <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;FungibleStore&gt;(&coin_store_ref),
        source_fungible_asset
    );
    <b>let</b> dispatch_function_info = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
        @aptos_framework,
        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="fungible_asset_intent_hooks.md#0x1_fungible_asset_intent_hooks">fungible_asset_intent_hooks</a>"),
        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"fa_to_fa_consumption"),
    );
    <a href="intent.md#0x1_intent_create_intent">intent::create_intent</a>(
        <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a> { extend_ref, delete_ref},
        <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a> { desired_metadata, desired_amount, issuer },
        expiry_time,
        dispatch_function_info,
        issuer,
    )
}
</code></pre>



</details>

<a id="0x1_fungible_asset_intent_start_fa_to_fa_session"></a>

## Function `start_fa_to_fa_session`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_start_fa_to_fa_session">start_fa_to_fa_session</a>(<a href="intent.md#0x1_intent">intent</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">fungible_asset_intent::FungibleStoreManager</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;&gt;): (<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_start_fa_to_fa_session">start_fa_to_fa_session</a>(
    <a href="intent.md#0x1_intent">intent</a>: Object&lt;TradeIntent&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a>, FungibleAsset, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;&gt;
): (FungibleAsset, TradeSession&lt;FungibleAsset, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;) {
    <b>let</b> (store_manager, session) = <a href="intent.md#0x1_intent_start_intent_session">intent::start_intent_session</a>(<a href="intent.md#0x1_intent">intent</a>);
    <b>let</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a> { extend_ref, delete_ref } = store_manager;
    <b>let</b> store_signer = <a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&extend_ref);
    <b>let</b> fa_store = <a href="object.md#0x1_object_object_from_delete_ref">object::object_from_delete_ref</a>&lt;FungibleStore&gt;(&delete_ref);
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw">fungible_asset::withdraw</a>(&store_signer, fa_store, <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(fa_store));
    <a href="fungible_asset.md#0x1_fungible_asset_remove_store">fungible_asset::remove_store</a>(&delete_ref);
    <a href="object.md#0x1_object_delete">object::delete</a>(delete_ref);
    (fa, session)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_intent_finish_fa_to_fa_session"></a>

## Function `finish_fa_to_fa_session`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_finish_fa_to_fa_session">finish_fa_to_fa_session</a>(session: <a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;, desired_token: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_finish_fa_to_fa_session">finish_fa_to_fa_session</a>(
    session: TradeSession&lt;FungibleAsset, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;,
    desired_token: FungibleAsset,
) {
    <a href="intent.md#0x1_intent_finish_intent_session">intent::finish_intent_session</a>(session, desired_token)
}
</code></pre>



</details>

<a id="0x1_fungible_asset_intent_unpack_argument"></a>

## Function `unpack_argument`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_unpack_argument">unpack_argument</a>(info: <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>): (<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, u64, <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_unpack_argument">unpack_argument</a>(info: <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>): (Object&lt;Metadata&gt;, u64, <b>address</b>) {
    <b>let</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a> { desired_metadata, desired_amount, issuer } = info;
    (desired_metadata, desired_amount, issuer)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
