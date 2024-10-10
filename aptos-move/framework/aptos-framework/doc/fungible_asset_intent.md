
<a id="0x1_fungible_asset_intent"></a>

# Module `0x1::fungible_asset_intent`



-  [Struct `FungibleStoreManager`](#0x1_fungible_asset_intent_FungibleStoreManager)
-  [Struct `FungibleAssetExchange`](#0x1_fungible_asset_intent_FungibleAssetExchange)
-  [Struct `FungibleAssetRecipientWitness`](#0x1_fungible_asset_intent_FungibleAssetRecipientWitness)
-  [Constants](#@Constants_0)
-  [Function `create_fa_to_fa_intent`](#0x1_fungible_asset_intent_create_fa_to_fa_intent)
-  [Function `start_fa_to_fa_session`](#0x1_fungible_asset_intent_start_fa_to_fa_session)
-  [Function `finish_fa_to_fa_session`](#0x1_fungible_asset_intent_finish_fa_to_fa_session)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="intent.md#0x1_intent">0x1::intent</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
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

<a id="0x1_fungible_asset_intent_FungibleAssetRecipientWitness"></a>

## Struct `FungibleAssetRecipientWitness`



<pre><code><b>struct</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetRecipientWitness">FungibleAssetRecipientWitness</a> <b>has</b> drop
</code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_fungible_asset_intent_EAMOUNT_NOT_MEET"></a>

The token offered does not meet amount requirement.


<pre><code><b>const</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_EAMOUNT_NOT_MEET">EAMOUNT_NOT_MEET</a>: u64 = 1;
</code></pre>



<a id="0x1_fungible_asset_intent_ENOT_DESIRED_TOKEN"></a>

The token offered is not the desired fungible asset.


<pre><code><b>const</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_ENOT_DESIRED_TOKEN">ENOT_DESIRED_TOKEN</a>: u64 = 0;
</code></pre>



<a id="0x1_fungible_asset_intent_create_fa_to_fa_intent"></a>

## Function `create_fa_to_fa_intent`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_create_fa_to_fa_intent">create_fa_to_fa_intent</a>(source_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, desired_metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, desired_amount: u64, expiry_time: u64, issuer: <b>address</b>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">fungible_asset_intent::FungibleStoreManager</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_create_fa_to_fa_intent">create_fa_to_fa_intent</a>(
    source_fungible_asset: FungibleAsset,
    desired_metadata: Object&lt;Metadata&gt;,
    desired_amount: u64,
    expiry_time: u64,
    issuer: <b>address</b>,
): Object&lt;TradeIntent&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;&gt; {
    <b>let</b> coin_store_ref = <a href="object.md#0x1_object_create_self_owned_object">object::create_self_owned_object</a>();
    <b>let</b> extend_ref = <a href="object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(&coin_store_ref);
    <b>let</b> delete_ref = <a href="object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(&coin_store_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_create_store">fungible_asset::create_store</a>(&coin_store_ref, <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&source_fungible_asset));
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(
        <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;FungibleStore&gt;(&coin_store_ref),
        source_fungible_asset
    );
    <a href="intent.md#0x1_intent_create_intent">intent::create_intent</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetRecipientWitness">FungibleAssetRecipientWitness</a>&gt;(
        <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a> { extend_ref, delete_ref},
        <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a> { desired_metadata, desired_amount, issuer },
        expiry_time,
        issuer,
    )
}
</code></pre>



</details>

<a id="0x1_fungible_asset_intent_start_fa_to_fa_session"></a>

## Function `start_fa_to_fa_session`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_start_fa_to_fa_session">start_fa_to_fa_session</a>(<a href="intent.md#0x1_intent">intent</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">fungible_asset_intent::FungibleStoreManager</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;&gt;): (<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_start_fa_to_fa_session">start_fa_to_fa_session</a>(
    <a href="intent.md#0x1_intent">intent</a>: Object&lt;TradeIntent&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleStoreManager">FungibleStoreManager</a>, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;&gt;
): (FungibleAsset, TradeSession&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;) {
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



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_finish_fa_to_fa_session">finish_fa_to_fa_session</a>(session: <a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">fungible_asset_intent::FungibleAssetExchange</a>&gt;, received_fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_finish_fa_to_fa_session">finish_fa_to_fa_session</a>(
    session: TradeSession&lt;<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetExchange">FungibleAssetExchange</a>&gt;,
    received_fa: FungibleAsset,
) {
    <b>let</b> argument = <a href="intent.md#0x1_intent_get_argument">intent::get_argument</a>(&session);
    <b>assert</b>!(
        <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&received_fa) == argument.desired_metadata,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_ENOT_DESIRED_TOKEN">ENOT_DESIRED_TOKEN</a>)
    );
    <b>assert</b>!(
        <a href="fungible_asset.md#0x1_fungible_asset_amount">fungible_asset::amount</a>(&received_fa) &gt;= argument.desired_amount,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset_intent.md#0x1_fungible_asset_intent_EAMOUNT_NOT_MEET">EAMOUNT_NOT_MEET</a>),
    );

    <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">primary_fungible_store::deposit</a>(argument.issuer, received_fa);
    <a href="intent.md#0x1_intent_finish_intent_session">intent::finish_intent_session</a>(session, <a href="fungible_asset_intent.md#0x1_fungible_asset_intent_FungibleAssetRecipientWitness">FungibleAssetRecipientWitness</a> {})
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
