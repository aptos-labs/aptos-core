
<a name="0x1_fungible_store"></a>

# Module `0x1::fungible_store`



-  [Resource `FungibleAssetStore`](#0x1_fungible_store_FungibleAssetStore)
-  [Constants](#@Constants_0)
-  [Function `balance`](#0x1_fungible_store_balance)
-  [Function `ungated_transfer_allowed`](#0x1_fungible_store_ungated_transfer_allowed)
-  [Function `deposit`](#0x1_fungible_store_deposit)
-  [Function `set_ungated_transfer`](#0x1_fungible_store_set_ungated_transfer)
-  [Function `withdraw`](#0x1_fungible_store_withdraw)
-  [Function `get_account_fungible_asset_object`](#0x1_fungible_store_get_account_fungible_asset_object)
-  [Function `create_account_fungible_asset_object`](#0x1_fungible_store_create_account_fungible_asset_object)
-  [Function `delete_account_fungible_asset_object`](#0x1_fungible_store_delete_account_fungible_asset_object)


<pre><code><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="fungible_source.md#0x1_fungible_source">0x1::fungible_source</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
</code></pre>



<a name="0x1_fungible_store_FungibleAssetStore"></a>

## Resource `FungibleAssetStore`

Represents all the fungible asset objects of an onwer keyed by the address of the base asset object.


<pre><code><b>struct</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;, <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_store_EACCOUNT_FUNGIBLE_ASSET_OBJECT"></a>

The account fungible asset object existence error.


<pre><code><b>const</b> <a href="fungible_store.md#0x1_fungible_store_EACCOUNT_FUNGIBLE_ASSET_OBJECT">EACCOUNT_FUNGIBLE_ASSET_OBJECT</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_store_balance"></a>

## Function `balance`

Check the balance of an <code>AccountFungibleAsset</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_balance">balance</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;T&gt;
): u64 <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <b>let</b> afa_opt = <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(
        <a href="account.md#0x1_account">account</a>,
        &asset,
        <b>false</b>
    );
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&afa_opt)) {
        <b>return</b> 0
    };
    <b>let</b> afa = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(afa_opt);
    <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(&afa)
}
</code></pre>



</details>

<a name="0x1_fungible_store_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Check the <code>AccountFungibleAsset</code> of <code><a href="account.md#0x1_account">account</a></code> allows ungated transfer.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;T&gt;
): bool <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <b>let</b> afa_opt = <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(
        <a href="account.md#0x1_account">account</a>,
        &asset,
        <b>false</b>
    );
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&afa_opt)) {
        <b>return</b> <b>true</b>
    };
    <b>let</b> afa = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(afa_opt);
    <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">fungible_asset::ungated_transfer_allowed</a>(&afa)
}
</code></pre>



</details>

<a name="0x1_fungible_store_deposit"></a>

## Function `deposit`

Deposit fungible asset to <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_deposit">deposit</a>(fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_deposit">deposit</a>(
    fa: FungibleAsset,
    <b>to</b>: <b>address</b>
) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> asset = <a href="fungible_asset.md#0x1_fungible_asset_fungible_asset_source">fungible_asset::fungible_asset_source</a>(&fa);
    <b>let</b> afa = ensure_account_fungible_asset_object(
        <b>to</b>,
        &asset,
        <b>true</b>
    );
    <a href="fungible_asset.md#0x1_fungible_asset_merge">fungible_asset::merge</a>(&afa, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_store_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Enable/disable the direct transfer of fungible assets.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;T&gt;,
    allow: bool
) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <b>let</b> afa_opt = <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>, &asset, !allow);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&afa_opt)) {
        <b>return</b>
    };
    <b>let</b> afa = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(afa_opt);
    <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">fungible_asset::set_ungated_transfer</a>(&afa, allow);
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(&afa) == 0 && <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">fungible_asset::ungated_transfer_allowed</a>(&afa)) {
        <a href="fungible_store.md#0x1_fungible_store_delete_account_fungible_asset_object">delete_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>, &asset);
    };
}
</code></pre>



</details>

<a name="0x1_fungible_store_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible assets from <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;T&gt;,
    amount: u64
): FungibleAsset <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <b>let</b> afa = ensure_account_fungible_asset_object(
        <a href="account.md#0x1_account">account</a>,
        &asset,
        <b>false</b>
    );

    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_extract">fungible_asset::extract</a>(&afa, amount);
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(&afa) == 0 && <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">fungible_asset::ungated_transfer_allowed</a>(&afa)) {
        <a href="fungible_store.md#0x1_fungible_store_delete_account_fungible_asset_object">delete_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>, &asset);
    };
    fa
}
</code></pre>



</details>

<a name="0x1_fungible_store_get_account_fungible_asset_object"></a>

## Function `get_account_fungible_asset_object`

Get the <code>AccountFungibleAsset</code> object of <code>asset</code> belonging to <code><a href="account.md#0x1_account">account</a></code>.
if <code>create_on_demand</code> is true, an default <code>AccountFungibleAsset</code> will be created if not exist; otherwise, abort
with error.


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;, create_on_demand: bool): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;FungibleSource&gt;,
    create_on_demand: bool
): Option&lt;Object&lt;AccountFungibleAsset&gt;&gt; <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    ensure_fungible_asset_store(<a href="account.md#0x1_account">account</a>);
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <b>let</b> index_table = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a>&gt;(<a href="account.md#0x1_account">account</a>).index;
    <b>if</b> (!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(index_table, <b>copy</b> asset)) {
        <b>if</b> (create_on_demand) {
            <b>let</b> afa_obj = <a href="fungible_store.md#0x1_fungible_store_create_account_fungible_asset_object">create_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>, &asset);
            <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(index_table, <b>copy</b> asset, afa_obj);
        } <b>else</b> {
            <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        }
    };
    <b>let</b> afa = *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(index_table, asset);
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(afa)
}
</code></pre>



</details>

<a name="0x1_fungible_store_create_account_fungible_asset_object"></a>

## Function `create_account_fungible_asset_object`

Create a default <code>AccountFungibleAsset</code> object with zero balance of <code>asset</code>.


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_create_account_fungible_asset_object">create_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_create_account_fungible_asset_object">create_account_fungible_asset_object</a>(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;FungibleSource&gt;
): Object&lt;AccountFungibleAsset&gt; {
    // Must review carefully here.
    <b>let</b> asset_signer = <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(asset));
    <b>let</b> creator_ref = <a href="object.md#0x1_object_create_object_from_object">object::create_object_from_object</a>(&asset_signer);
    <b>let</b> afa = <a href="fungible_asset.md#0x1_fungible_asset_new">fungible_asset::new</a>(&creator_ref, asset);
    // Transfer the owner <b>to</b> `<a href="account.md#0x1_account">account</a>`.
    <a href="object.md#0x1_object_transfer">object::transfer</a>(&asset_signer, afa, <a href="account.md#0x1_account">account</a>);
    // Disable transfer of <a href="coin.md#0x1_coin">coin</a> <a href="object.md#0x1_object">object</a> so the <a href="object.md#0x1_object">object</a> itself never gets transfered.
    <b>let</b> transfer_ref = <a href="object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&creator_ref);
    <a href="object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(&transfer_ref);
    afa
}
</code></pre>



</details>

<a name="0x1_fungible_store_delete_account_fungible_asset_object"></a>

## Function `delete_account_fungible_asset_object`

Remove the <code>AccountFungibleAsset</code> object of <code>asset</code> from <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_delete_account_fungible_asset_object">delete_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_delete_account_fungible_asset_object">delete_account_fungible_asset_object</a>(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    asset: &Object&lt;FungibleSource&gt;
) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    // Delete <b>if</b> balance drops <b>to</b> 0 and ungated_transfer is allowed.
    ensure_fungible_asset_store(<a href="account.md#0x1_account">account</a>);
    <b>let</b> index_table = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a>&gt;(<a href="account.md#0x1_account">account</a>).index;
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(index_table, *asset), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="fungible_store.md#0x1_fungible_store_EACCOUNT_FUNGIBLE_ASSET_OBJECT">EACCOUNT_FUNGIBLE_ASSET_OBJECT</a>));
    <b>let</b> afa = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_remove">smart_table::remove</a>(index_table, *asset);
    <a href="fungible_asset.md#0x1_fungible_asset_destory_account_fungible_asset">fungible_asset::destory_account_fungible_asset</a>(afa);
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
