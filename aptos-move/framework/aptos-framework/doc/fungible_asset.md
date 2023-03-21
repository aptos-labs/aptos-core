
<a name="0x1_fungible_asset"></a>

# Module `0x1::fungible_asset`

This defines the fungible asset module that can issue fungible assets of any <code>FungibleSource</code> object. The source
can be a token object or any object that equipped with <code>FungibleSource</code> resource.


-  [Resource `AccountFungibleAsset`](#0x1_fungible_asset_AccountFungibleAsset)
-  [Struct `FungibleAsset`](#0x1_fungible_asset_FungibleAsset)
-  [Constants](#@Constants_0)
-  [Function `fungible_asset_source`](#0x1_fungible_asset_fungible_asset_source)
-  [Function `fungible_asset_amount`](#0x1_fungible_asset_fungible_asset_amount)
-  [Function `new`](#0x1_fungible_asset_new)
-  [Function `mint`](#0x1_fungible_asset_mint)
-  [Function `destory_account_fungible_asset`](#0x1_fungible_asset_destory_account_fungible_asset)
-  [Function `burn`](#0x1_fungible_asset_burn)
-  [Function `extract`](#0x1_fungible_asset_extract)
-  [Function `merge`](#0x1_fungible_asset_merge)
-  [Function `balance`](#0x1_fungible_asset_balance)
-  [Function `account_fungible_asset_source`](#0x1_fungible_asset_account_fungible_asset_source)
-  [Function `ungated_transfer_allowed`](#0x1_fungible_asset_ungated_transfer_allowed)
-  [Function `set_ungated_transfer`](#0x1_fungible_asset_set_ungated_transfer)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_source.md#0x1_fungible_source">0x1::fungible_source</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
</code></pre>



<a name="0x1_fungible_asset_AccountFungibleAsset"></a>

## Resource `AccountFungibleAsset`

The resource of an object recording the properties of the fungible assets held of the object owner.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;</code>
</dt>
<dd>
 The address of the base asset object.
</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>
 The balance of the fungible asset.
</dd>
<dt>
<code>allow_ungated_transfer: bool</code>
</dt>
<dd>
 Fungible Assets transferring is a common operation, this allows for disabling and enabling
 transfers bypassing the use of a TransferCap.
</dd>
<dt>
<code>delete_ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a></code>
</dt>
<dd>
 The delete_ref of this object, used for cleanup.
</dd>
</dl>


</details>

<a name="0x1_fungible_asset_FungibleAsset"></a>

## Struct `FungibleAsset`

The transferable version of fungible asset.
Note: it does not have <code>store</code> ability so only used in hot potato pattern.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient amount.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 4;
</code></pre>



<a name="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_asset_EBALANCE_NOT_ZERO"></a>

The token account has positive amount so cannot be deleted.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBALANCE_NOT_ZERO">EBALANCE_NOT_ZERO</a>: u64 = 2;
</code></pre>



<a name="0x1_fungible_asset_EFUNGIBLE_ASSET_TYPE_MISMATCH"></a>

FungibleAsset type mismatch.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_TYPE_MISMATCH">EFUNGIBLE_ASSET_TYPE_MISMATCH</a>: u64 = 5;
</code></pre>



<a name="0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED"></a>

The token account is still allow_ungated_transfer so cannot be deleted.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>: u64 = 3;
</code></pre>



<a name="0x1_fungible_asset_fungible_asset_source"></a>

## Function `fungible_asset_source`

Self-explainatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_fungible_asset_source">fungible_asset_source</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_fungible_asset_source">fungible_asset_source</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;FungibleSource&gt; {
    fa.asset
}
</code></pre>



</details>

<a name="0x1_fungible_asset_fungible_asset_amount"></a>

## Function `fungible_asset_amount`

Self-explainatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_fungible_asset_amount">fungible_asset_amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_fungible_asset_amount">fungible_asset_amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): u64 {
    fa.amount
}
</code></pre>



</details>

<a name="0x1_fungible_asset_new"></a>

## Function `new`

Create a new <code>Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_new">new</a>&lt;T: key&gt;(creator_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_new">new</a>&lt;T: key&gt;(
    creator_ref: &ConstructorRef,
    asset: &Object&lt;T&gt;,
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt; {
    <b>let</b> pfa_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(creator_ref);
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);

    <b>move_to</b>(&pfa_signer, <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
        asset,
        balance: 0,
        allow_ungated_transfer: <b>true</b>,
        delete_ref: <a href="object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(creator_ref)
    });
    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;(creator_ref)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint fungible asset with <code>amount</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>&lt;T: key&gt;(
    asset: &Object&lt;T&gt;,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <a href="fungible_source.md#0x1_fungible_source_increase_supply">fungible_source::increase_supply</a>(&asset, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        asset,
        amount
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_destory_account_fungible_asset"></a>

## Function `destory_account_fungible_asset`

Burn fungible asset.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destory_account_fungible_asset">destory_account_fungible_asset</a>(afa: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destory_account_fungible_asset">destory_account_fungible_asset</a>(afa: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
        asset: _,
        balance: _,
        allow_ungated_transfer: _,
        delete_ref
    } = <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&afa));
    <a href="object.md#0x1_object_delete">object::delete</a>(delete_ref);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_burn"></a>

## Function `burn`

Burn fungible asset.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        asset,
        amount,
    } = <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>;
    <a href="fungible_source.md#0x1_fungible_source_decrease_supply">fungible_source::decrease_supply</a>(&asset, amount);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract <code>amount</code> of fungible asset from a <code><a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a></code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(afa: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(
    afa: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> afa = borrow_fungible_asset_mut(afa);
    <b>assert</b>!(afa.allow_ungated_transfer, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>));
    <b>assert</b>!(afa.balance &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    afa.balance = afa.balance - amount;
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        asset: afa.asset,
        amount
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_merge"></a>

## Function `merge`

Merge <code>amount</code> of fungible asset to <code><a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a></code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(afa: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(
    afa: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { asset, amount } = fa;
    // ensure merging the same <a href="coin.md#0x1_coin">coin</a>
    <b>let</b> afa = borrow_fungible_asset_mut(afa);
    <b>assert</b>!(afa.allow_ungated_transfer, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>));
    <b>assert</b>!(afa.asset == asset, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_TYPE_MISMATCH">EFUNGIBLE_ASSET_TYPE_MISMATCH</a>));
    afa.balance = afa.balance + amount;
}
</code></pre>



</details>

<a name="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of an <code>Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>(afa: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>(afa: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    borrow_fungible_asset(afa).balance
}
</code></pre>



</details>

<a name="0x1_fungible_asset_account_fungible_asset_source"></a>

## Function `account_fungible_asset_source`

Get the source object of an <code>Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_account_fungible_asset_source">account_fungible_asset_source</a>(afa: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_account_fungible_asset_source">account_fungible_asset_source</a>(
    afa: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;
): Object&lt;FungibleSource&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    borrow_fungible_asset(afa).asset
}
</code></pre>



</details>

<a name="0x1_fungible_asset_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Whether <code>ungated_transfer</code> is alllowed.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(afa: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(afa: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    borrow_fungible_asset(afa).allow_ungated_transfer
}
</code></pre>



</details>

<a name="0x1_fungible_asset_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Set <code>ungated_transfer</code> to the passed in <code>allow</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(afa: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">fungible_asset::AccountFungibleAsset</a>&gt;, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(
    afa: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a>&gt;,
    allow: bool
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_AccountFungibleAsset">AccountFungibleAsset</a> {
    borrow_fungible_asset_mut(afa).allow_ungated_transfer = allow;
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
