
<a name="0x1_managed_fungible_metadata"></a>

# Module `0x1::managed_fungible_metadata`

This module provides an addtional ready-to-use solution on top of <code>FungibleAssetMetadata</code> that manages the refs of
mint, burn and transfer for the creator in a straightforward scheme. It offers creators to destory any refs in an
on-demand manner too.


-  [Resource `ManagingRefs`](#0x1_managed_fungible_metadata_ManagingRefs)
-  [Constants](#@Constants_0)
-  [Function `init_managing_refs`](#0x1_managed_fungible_metadata_init_managing_refs)
-  [Function `mint`](#0x1_managed_fungible_metadata_mint)
-  [Function `withdraw`](#0x1_managed_fungible_metadata_withdraw)
-  [Function `deposit`](#0x1_managed_fungible_metadata_deposit)
-  [Function `transfer`](#0x1_managed_fungible_metadata_transfer)
-  [Function `burn`](#0x1_managed_fungible_metadata_burn)
-  [Function `set_ungated_transfer`](#0x1_managed_fungible_metadata_set_ungated_transfer)
-  [Function `can_mint`](#0x1_managed_fungible_metadata_can_mint)
-  [Function `can_transfer`](#0x1_managed_fungible_metadata_can_transfer)
-  [Function `can_burn`](#0x1_managed_fungible_metadata_can_burn)
-  [Function `waive_mint`](#0x1_managed_fungible_metadata_waive_mint)
-  [Function `waive_transfer`](#0x1_managed_fungible_metadata_waive_transfer)
-  [Function `waive_burn`](#0x1_managed_fungible_metadata_waive_burn)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="fungible_store.md#0x1_fungible_store">0x1::fungible_store</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_managed_fungible_metadata_ManagingRefs"></a>

## Resource `ManagingRefs`

Hold refs to control the minting, transfer and burning of fungible assets.


<pre><code><b>struct</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transfer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_managed_fungible_metadata_ENOT_OWNER"></a>

Not the owner.


<pre><code><b>const</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ENOT_OWNER">ENOT_OWNER</a>: u64 = 4;
</code></pre>



<a name="0x1_managed_fungible_metadata_EBURN_REF"></a>

BurnRef existence error.


<pre><code><b>const</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_EBURN_REF">EBURN_REF</a>: u64 = 3;
</code></pre>



<a name="0x1_managed_fungible_metadata_EMANAGED_FUNGIBLE_ASSET_REFS"></a>

Refs existence errors.


<pre><code><b>const</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_EMANAGED_FUNGIBLE_ASSET_REFS">EMANAGED_FUNGIBLE_ASSET_REFS</a>: u64 = 5;
</code></pre>



<a name="0x1_managed_fungible_metadata_EMINT_REF"></a>

MintRef existence error.


<pre><code><b>const</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_EMINT_REF">EMINT_REF</a>: u64 = 1;
</code></pre>



<a name="0x1_managed_fungible_metadata_ETRANSFER_REF"></a>

TransferRef existence error.


<pre><code><b>const</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ETRANSFER_REF">ETRANSFER_REF</a>: u64 = 2;
</code></pre>



<a name="0x1_managed_fungible_metadata_init_managing_refs"></a>

## Function `init_managing_refs`

Initialize metadata object and store the refs.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_init_managing_refs">init_managing_refs</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_init_managing_refs">init_managing_refs</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8
) {
    <b>let</b> (mint_ref, transfer_ref, burn_ref) = <a href="fungible_asset.md#0x1_fungible_asset_init_metadata">fungible_asset::init_metadata</a>(
        constructor_ref,
        maximum_supply,
        name,
        symbol,
        decimals
    );
    <b>let</b> metadata_object_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(
        &metadata_object_signer,
        <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
            mint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mint_ref), transfer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(transfer_ref), burn: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(burn_ref)
        }
    )
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_mint"></a>

## Function `mint`

Mint as the owner of metadata object.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_mint">mint</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_mint">mint</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    amount: u64,
    <b>to</b>: <b>address</b>
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    assert_owner(metadata_owner, metadata);
    <b>let</b> mint_ref = borrow_mint_from_refs(metadata);
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_mint">fungible_asset::mint</a>(mint_ref, amount);
    <a href="fungible_store.md#0x1_fungible_store_deposit">fungible_store::deposit</a>(fa, <b>to</b>);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_withdraw"></a>

## Function `withdraw`

Withdraw as the owner of metadata object ignoring <code>allow_ungated_transfer</code> field.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_withdraw">withdraw</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, from: <b>address</b>): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_withdraw">withdraw</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    amount: u64,
    from: <b>address</b>,
): FungibleAsset <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    assert_owner(metadata_owner, metadata);
    <b>let</b> transfer_ref = borrow_transfer_from_refs(metadata);
    <a href="fungible_store.md#0x1_fungible_store_withdraw_with_ref">fungible_store::withdraw_with_ref</a>(transfer_ref, from, amount)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_deposit"></a>

## Function `deposit`

Deposit as the owner of metadata object ignoring <code>allow_ungated_transfer</code> field.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_deposit">deposit</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_deposit">deposit</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    <b>to</b>: <b>address</b>,
    fa: FungibleAsset
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    assert_owner(metadata_owner, metadata);
    <b>let</b> transfer_ref = borrow_transfer_from_refs(metadata);
    <a href="fungible_store.md#0x1_fungible_store_deposit_with_ref">fungible_store::deposit_with_ref</a>(transfer_ref, <b>to</b>, fa);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_transfer"></a>

## Function `transfer`

Transfer as the owner of metadata object ignoring <code>allow_ungated_transfer</code> field.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_transfer">transfer</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, from: <b>address</b>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_transfer">transfer</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    from: <b>address</b>,
    <b>to</b>: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    assert_owner(metadata_owner, metadata);
    <b>let</b> transfer_ref = borrow_transfer_from_refs(metadata);
    <a href="fungible_store.md#0x1_fungible_store_transfer_with_ref">fungible_store::transfer_with_ref</a>(transfer_ref, from, <b>to</b>, amount);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_burn"></a>

## Function `burn`

Burn fungible assets as the owner of metadata object.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_burn">burn</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, from: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_burn">burn</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    from: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    assert_owner(metadata_owner, metadata);
    <b>let</b> burn_ref = borrow_burn_from_refs(metadata);
    <a href="fungible_store.md#0x1_fungible_store_burn">fungible_store::burn</a>(burn_ref, from, amount);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Set the <code>allow_ungated_transfer</code> field in <code>AccountFungibleAsset</code> associated with <code>metadata</code> of <code><a href="account.md#0x1_account">account</a></code> as the
owner of metadata object.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <a href="account.md#0x1_account">account</a>: <b>address</b>, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    allow: bool
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    assert_owner(metadata_owner, metadata);
    <b>let</b> transfer_ref = borrow_transfer_from_refs(metadata);
    <a href="fungible_store.md#0x1_fungible_store_set_ungated_transfer">fungible_store::set_ungated_transfer</a>(transfer_ref, <a href="account.md#0x1_account">account</a>, allow);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_can_mint"></a>

## Function `can_mint`

Return if the owner of <code>metadata</code> has access to <code>MintRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_can_mint">can_mint</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_can_mint">can_mint</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): bool <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&borrow_refs(metadata).mint)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_can_transfer"></a>

## Function `can_transfer`

Return if the owner of <code>metadata</code> has access to <code>TransferRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_can_transfer">can_transfer</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_can_transfer">can_transfer</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): bool <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&borrow_refs(metadata).transfer)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_can_burn"></a>

## Function `can_burn`

Return if the owner of <code>metadata</code> has access to <code>BurnRef</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_can_burn">can_burn</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_can_burn">can_burn</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): bool <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&borrow_refs(metadata).burn)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_waive_mint"></a>

## Function `waive_mint`

Let metadata owner to explicitly waive the mint capability.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_waive_mint">waive_mint</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_waive_mint">waive_mint</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    <b>let</b> mint_ref = &<b>mut</b> borrow_refs_mut(metadata_owner, metadata).mint;
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(mint_ref), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_EMINT_REF">EMINT_REF</a>));
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(mint_ref);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_waive_transfer"></a>

## Function `waive_transfer`

Let metadata owner to explicitly waive the transfer capability.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_waive_transfer">waive_transfer</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_waive_transfer">waive_transfer</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    <b>let</b> transfer_ref = &<b>mut</b> borrow_refs_mut(metadata_owner, metadata).transfer;
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(transfer_ref), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ETRANSFER_REF">ETRANSFER_REF</a>));
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(transfer_ref);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_metadata_waive_burn"></a>

## Function `waive_burn`

Let metadata owner to explicitly waive the burn capability.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_waive_burn">waive_burn</a>&lt;T: key&gt;(metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_waive_burn">waive_burn</a>&lt;T: key&gt;(
    metadata_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: &Object&lt;T&gt;
) <b>acquires</b> <a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ManagingRefs">ManagingRefs</a> {
    <b>let</b> burn_ref = &<b>mut</b> borrow_refs_mut(metadata_owner, metadata).burn;
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(burn_ref), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_fungible_metadata.md#0x1_managed_fungible_metadata_ETRANSFER_REF">ETRANSFER_REF</a>));
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(burn_ref);
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
