
<a name="0x1_fungible_store"></a>

# Module `0x1::fungible_store`

This defines a store of <code>FungibleAssetWallet</code> under each account.


-  [Resource `FungibleAssetStore`](#0x1_fungible_store_FungibleAssetStore)
-  [Constants](#@Constants_0)
-  [Function `balance`](#0x1_fungible_store_balance)
-  [Function `ungated_transfer_allowed`](#0x1_fungible_store_ungated_transfer_allowed)
-  [Function `set_ungated_transfer`](#0x1_fungible_store_set_ungated_transfer)
-  [Function `withdraw`](#0x1_fungible_store_withdraw)
-  [Function `deposit`](#0x1_fungible_store_deposit)
-  [Function `transfer`](#0x1_fungible_store_transfer)
-  [Function `transfer_with_ref`](#0x1_fungible_store_transfer_with_ref)
-  [Function `withdraw_with_ref`](#0x1_fungible_store_withdraw_with_ref)
-  [Function `deposit_with_ref`](#0x1_fungible_store_deposit_with_ref)
-  [Function `burn`](#0x1_fungible_store_burn)
-  [Function `get_account_fungible_asset_object`](#0x1_fungible_store_get_account_fungible_asset_object)
-  [Function `create_account_fungible_asset_object`](#0x1_fungible_store_create_account_fungible_asset_object)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
</code></pre>



<a name="0x1_fungible_store_FungibleAssetStore"></a>

## Resource `FungibleAssetStore`

Represents all the fungible asset wallet objects of an onwer keyed by the base metadata objects.


<pre><code><b>struct</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;, <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_store_EFUNGIBLE_ASSET_WALLET_OBJECT"></a>

The fungible asset wallet object existence error.


<pre><code><b>const</b> <a href="fungible_store.md#0x1_fungible_store_EFUNGIBLE_ASSET_WALLET_OBJECT">EFUNGIBLE_ASSET_WALLET_OBJECT</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_store_balance"></a>

## Function `balance`

Check the balance of an account.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_balance">balance</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    metadata: &Object&lt;T&gt;
): u64 <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">fungible_asset::verify</a>(metadata);
    <b>let</b> afa_opt = <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(
        <a href="account.md#0x1_account">account</a>,
        &metadata,
        <b>false</b>
    );
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&afa_opt)) {
        <b>return</b> 0
    };
    <b>let</b> wallet = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(afa_opt);
    <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(&wallet)
}
</code></pre>



</details>

<a name="0x1_fungible_store_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Return true if <code><a href="account.md#0x1_account">account</a></code> allows ungated transfer.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    metadata: &Object&lt;T&gt;
): bool <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">fungible_asset::verify</a>(metadata);
    <b>let</b> afa_opt = <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(
        <a href="account.md#0x1_account">account</a>,
        &metadata,
        <b>false</b>
    );
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&afa_opt)) {
        <b>return</b> <b>true</b>
    };
    <b>let</b> wallet = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(afa_opt);
    <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">fungible_asset::ungated_transfer_allowed</a>(&wallet)
}
</code></pre>



</details>

<a name="0x1_fungible_store_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Enable/disable the direct transfer.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_set_ungated_transfer">set_ungated_transfer</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_set_ungated_transfer">set_ungated_transfer</a>(
    ref: &TransferRef,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    allow: bool
) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">fungible_asset::verify</a>(&<a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(ref));
    <b>let</b> afa_opt = <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>, &metadata, !allow);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&afa_opt)) {
        <b>return</b>
    };
    <b>let</b> wallet = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(afa_opt);
    <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">fungible_asset::set_ungated_transfer</a>(ref, &wallet, allow);
    maybe_delete(wallet);
}
</code></pre>



</details>

<a name="0x1_fungible_store_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible asset from <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    amount: u64
): FungibleAsset <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">fungible_asset::verify</a>(metadata);
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> wallet = ensure_fungible_asset_wallet(
        account_address,
        &metadata,
        <b>false</b>
    );

    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw">fungible_asset::withdraw</a>(<a href="account.md#0x1_account">account</a>, &wallet, amount);
    maybe_delete(wallet);
    fa
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
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&fa);
    <b>let</b> wallet = ensure_fungible_asset_wallet(
        <b>to</b>,
        &metadata,
        <b>true</b>
    );
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(&wallet, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_store_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset as the owner.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_transfer">transfer</a>&lt;T: key&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_transfer">transfer</a>&lt;T: key&gt;(
    from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    amount: u64,
    <b>to</b>: <b>address</b>
) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> fa = <a href="fungible_store.md#0x1_fungible_store_withdraw">withdraw</a>(from, metadata, amount);
    <a href="fungible_store.md#0x1_fungible_store_deposit">deposit</a>(fa, <b>to</b>);
}
</code></pre>



</details>

<a name="0x1_fungible_store_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>ammount</code> of fungible asset ignoring <code>allow_ungated_transfer</code> with <code>TransferRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_transfer_with_ref">transfer_with_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, from: <b>address</b>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_transfer_with_ref">transfer_with_ref</a>(
    ref: &TransferRef,
    from: <b>address</b>,
    <b>to</b>: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> sender_wallet = ensure_fungible_asset_wallet(
        from,
        &<a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(ref),
        <b>false</b>
    );
    <b>let</b> receiver_wallet = ensure_fungible_asset_wallet(
        <b>to</b>,
        &<a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(ref),
        <b>true</b>
    );
    <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">fungible_asset::transfer_with_ref</a>(ref, &sender_wallet, &receiver_wallet, amount);
}
</code></pre>



</details>

<a name="0x1_fungible_store_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw <code>ammount</code> of fungible asset ignoring <code>allow_ungated_transfer</code> with <code>TransferRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_withdraw_with_ref">withdraw_with_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_withdraw_with_ref">withdraw_with_ref</a>(
    ref: &TransferRef,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    amount: u64
): FungibleAsset <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> wallet = ensure_fungible_asset_wallet(
        <a href="account.md#0x1_account">account</a>,
        &<a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(ref),
        <b>false</b>
    );
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">fungible_asset::withdraw_with_ref</a>(ref, &wallet, amount)
}
</code></pre>



</details>

<a name="0x1_fungible_store_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit <code>ammount</code> of fungible asset ignoring <code>allow_ungated_transfer</code> with <code>TransferRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_deposit_with_ref">deposit_with_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_deposit_with_ref">deposit_with_ref</a>(ref: &TransferRef, <a href="account.md#0x1_account">account</a>: <b>address</b>, fa: FungibleAsset) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> wallet = ensure_fungible_asset_wallet(
        <a href="account.md#0x1_account">account</a>,
        &<a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(ref),
        <b>true</b>
    );
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">fungible_asset::deposit_with_ref</a>(ref, &wallet, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_store_burn"></a>

## Function `burn`

Burn the <code>amount</code> of fungible asset from <code><a href="account.md#0x1_account">account</a></code> with <code>BurnRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_burn">burn</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_store.md#0x1_fungible_store_burn">burn</a>(ref: &BurnRef, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64) <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    <b>let</b> wallet = ensure_fungible_asset_wallet(
        <a href="account.md#0x1_account">account</a>,
        &<a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">fungible_asset::burn_ref_metadata</a>(ref),
        <b>false</b>
    );
    <a href="fungible_asset.md#0x1_fungible_asset_burn">fungible_asset::burn</a>(ref, &wallet, amount);
    maybe_delete(wallet);
}
</code></pre>



</details>

<a name="0x1_fungible_store_get_account_fungible_asset_object"></a>

## Function `get_account_fungible_asset_object`

Get the <code>FungibleAssetWallet</code> object of <code>metadata</code> belonging to <code><a href="account.md#0x1_account">account</a></code>.
if <code>create_on_demand</code> is true, an default <code>FungibleAssetWallet</code> will be created if not exist; otherwise abort.


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;, create_on_demand: bool): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_get_account_fungible_asset_object">get_account_fungible_asset_object</a>(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    metadata: &Object&lt;FungibleAssetMetadata&gt;,
    create_on_demand: bool
): Option&lt;Object&lt;FungibleAssetWallet&gt;&gt; <b>acquires</b> <a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a> {
    ensure_fungible_asset_store(<a href="account.md#0x1_account">account</a>);
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">fungible_asset::verify</a>(metadata);
    <b>let</b> index_table = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="fungible_store.md#0x1_fungible_store_FungibleAssetStore">FungibleAssetStore</a>&gt;(<a href="account.md#0x1_account">account</a>).index;
    <b>if</b> (!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(index_table, <b>copy</b> metadata)) {
        <b>if</b> (create_on_demand) {
            <b>let</b> afa_obj = <a href="fungible_store.md#0x1_fungible_store_create_account_fungible_asset_object">create_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>, &metadata);
            <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(index_table, <b>copy</b> metadata, afa_obj);
        } <b>else</b> {
            <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        }
    };
    <b>let</b> wallet = *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(index_table, metadata);
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(wallet)
}
</code></pre>



</details>

<a name="0x1_fungible_store_create_account_fungible_asset_object"></a>

## Function `create_account_fungible_asset_object`

Create a default <code>FungibleAssetWallet</code> object with zero balance of <code>metadata</code>.


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_create_account_fungible_asset_object">create_account_fungible_asset_object</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_store.md#0x1_fungible_store_create_account_fungible_asset_object">create_account_fungible_asset_object</a>(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    metadata: &Object&lt;FungibleAssetMetadata&gt;
): Object&lt;FungibleAssetWallet&gt; {
    // Must review carefully here.
    <b>let</b> asset_signer = <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(metadata));
    <b>let</b> creator_ref = <a href="object.md#0x1_object_create_object_from_object">object::create_object_from_object</a>(&asset_signer);
    <b>let</b> wallet = <a href="fungible_asset.md#0x1_fungible_asset_new_fungible_asset_wallet_object">fungible_asset::new_fungible_asset_wallet_object</a>(&creator_ref, metadata);
    // Transfer the owner <b>to</b> `<a href="account.md#0x1_account">account</a>`.
    <a href="object.md#0x1_object_transfer">object::transfer</a>(&asset_signer, wallet, <a href="account.md#0x1_account">account</a>);
    // Disable transfer of <a href="coin.md#0x1_coin">coin</a> <a href="object.md#0x1_object">object</a> so the <a href="object.md#0x1_object">object</a> itself never gets transfered.
    <b>let</b> transfer_ref = <a href="object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&creator_ref);
    <a href="object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(&transfer_ref);
    wallet
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
