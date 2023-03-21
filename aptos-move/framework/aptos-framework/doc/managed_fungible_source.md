
<a name="0x1_managed_fungible_source"></a>

# Module `0x1::managed_fungible_source`

This module provides an addtional abstraction on top of <code>FungibleSource</code> that manages the capabilities of mint, burn
and transfer for the creator in a simple way. It offers creators to destory any capabilities in an on-demand way too.
For more advanced goverance, please build your own module to manage capabilitys extending <code>FungibleSource</code>.


-  [Resource `ManagingCapabilities`](#0x1_managed_fungible_source_ManagingCapabilities)
-  [Constants](#@Constants_0)
-  [Function `init_managing_capabilities`](#0x1_managed_fungible_source_init_managing_capabilities)
-  [Function `mint`](#0x1_managed_fungible_source_mint)
-  [Function `transfer`](#0x1_managed_fungible_source_transfer)
-  [Function `burn`](#0x1_managed_fungible_source_burn)
-  [Function `set_ungated_transfer`](#0x1_managed_fungible_source_set_ungated_transfer)
-  [Function `owner_can_mint`](#0x1_managed_fungible_source_owner_can_mint)
-  [Function `owner_can_transfer`](#0x1_managed_fungible_source_owner_can_transfer)
-  [Function `owner_can_burn`](#0x1_managed_fungible_source_owner_can_burn)
-  [Function `waive_mint`](#0x1_managed_fungible_source_waive_mint)
-  [Function `waive_transfer`](#0x1_managed_fungible_source_waive_transfer)
-  [Function `waive_burn`](#0x1_managed_fungible_source_waive_burn)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_caps.md#0x1_fungible_caps">0x1::fungible_caps</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_managed_fungible_source_ManagingCapabilities"></a>

## Resource `ManagingCapabilities`

Used to hold capabilities to control the minting, transfer and burning of fungible assets.


<pre><code><b>struct</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_caps.md#0x1_fungible_caps_MintCap">fungible_caps::MintCap</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transfer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="fungible_caps.md#0x1_fungible_caps_BurnCap">fungible_caps::BurnCap</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_managed_fungible_source_EBURN_CAP"></a>

Burn capability exists or does not exist.


<pre><code><b>const</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_EBURN_CAP">EBURN_CAP</a>: u64 = 3;
</code></pre>



<a name="0x1_managed_fungible_source_EFREEZE_CAP"></a>

Transfer capability exists does not exist.


<pre><code><b>const</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_EFREEZE_CAP">EFREEZE_CAP</a>: u64 = 2;
</code></pre>



<a name="0x1_managed_fungible_source_EMANAGED_FUNGIBLE_ASSET_CAPS"></a>

Caps existence errors.


<pre><code><b>const</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_EMANAGED_FUNGIBLE_ASSET_CAPS">EMANAGED_FUNGIBLE_ASSET_CAPS</a>: u64 = 5;
</code></pre>



<a name="0x1_managed_fungible_source_EMINT_CAP"></a>

Mint capability exists or does not exist.


<pre><code><b>const</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_EMINT_CAP">EMINT_CAP</a>: u64 = 1;
</code></pre>



<a name="0x1_managed_fungible_source_ENOT_OWNER"></a>

Not the owner.


<pre><code><b>const</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ENOT_OWNER">ENOT_OWNER</a>: u64 = 4;
</code></pre>



<a name="0x1_managed_fungible_source_init_managing_capabilities"></a>

## Function `init_managing_capabilities`

Initialize capabilities of an asset object after initializing <code>FungibleSource</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_init_managing_capabilities">init_managing_capabilities</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_init_managing_capabilities">init_managing_capabilities</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8
) {
    <b>let</b> (mint_cap, transfer_cap, burn_cap) = <a href="fungible_caps.md#0x1_fungible_caps_init_fungible_source_with_caps">fungible_caps::init_fungible_source_with_caps</a>(
        constructor_ref,
        maximum_supply,
        name,
        symbol,
        decimals
    );
    <b>let</b> asset_object_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(
        &asset_object_signer,
        <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
            mint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(mint_cap), transfer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                transfer_cap
            ), burn: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(burn_cap)
        }
    )
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_mint"></a>

## Function `mint`

Mint fungible assets as the owner of the base asset.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_mint">mint</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_mint">mint</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;,
    amount: u64,
    <b>to</b>: <b>address</b>
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    assert_owner(asset_owner, asset);
    <b>let</b> mint_cap = borrow_mint_from_caps(asset);
    <a href="fungible_caps.md#0x1_fungible_caps_mint">fungible_caps::mint</a>(mint_cap, amount, <b>to</b>);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_transfer"></a>

## Function `transfer`

Transfer fungible assets as the owner of the base asset ignoring the <code>allow_ungated_transfer</code> field in
<code>AccountFungibleAsset</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_transfer">transfer</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, from: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_transfer">transfer</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;,
    amount: u64,
    from: <b>address</b>,
    <b>to</b>: <b>address</b>,
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    assert_owner(asset_owner, asset);
    <b>let</b> transfer_cap = borrow_transfer_from_caps(asset);
    <a href="fungible_caps.md#0x1_fungible_caps_transfer_with_cap">fungible_caps::transfer_with_cap</a>(transfer_cap, amount, from, <b>to</b>);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_burn"></a>

## Function `burn`

Burn fungible assets as the owner of the base asset.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_burn">burn</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, from: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_burn">burn</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;,
    amount: u64,
    from: <b>address</b>
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    assert_owner(asset_owner, asset);
    <b>let</b> burn_cap = borrow_burn_from_caps(asset);
    <a href="fungible_caps.md#0x1_fungible_caps_burn">fungible_caps::burn</a>(burn_cap, amount, from);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Set the <code>allow_ungated_transfer</code> field in <code>AccountFungibleAsset</code> of <code>asset</code> belonging to <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <a href="account.md#0x1_account">account</a>: <b>address</b>, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    allow: bool
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    assert_owner(asset_owner, asset);
    <b>let</b> transfer_cap = borrow_transfer_from_caps(asset);
    <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">fungible_caps::set_ungated_transfer</a>(transfer_cap, <a href="account.md#0x1_account">account</a>, allow);
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_owner_can_mint"></a>

## Function `owner_can_mint`

Return if the owner has access to <code>MintCap</code> of <code>asset</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_owner_can_mint">owner_can_mint</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_owner_can_mint">owner_can_mint</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): bool <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&borrow_caps(asset).mint)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_owner_can_transfer"></a>

## Function `owner_can_transfer`

Return if the owner has access to <code>TransferCap</code> of <code>asset</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_owner_can_transfer">owner_can_transfer</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_owner_can_transfer">owner_can_transfer</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): bool <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&borrow_caps(asset).transfer)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_owner_can_burn"></a>

## Function `owner_can_burn`

Return if the owner has access to <code>BurnCap</code> of <code>asset</code>.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_owner_can_burn">owner_can_burn</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_owner_can_burn">owner_can_burn</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): bool <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&borrow_caps(asset).burn)
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_waive_mint"></a>

## Function `waive_mint`

Let asset owner to explicitly waive the mint capability.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_waive_mint">waive_mint</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_waive_mint">waive_mint</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    <b>let</b> mint_cap = &<b>mut</b> borrow_caps_mut(asset_owner, asset).mint;
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(mint_cap), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_fungible_source.md#0x1_managed_fungible_source_EMINT_CAP">EMINT_CAP</a>));
    <a href="fungible_caps.md#0x1_fungible_caps_destroy_mint_cap">fungible_caps::destroy_mint_cap</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(mint_cap));
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_waive_transfer"></a>

## Function `waive_transfer`

Let asset owner to explicitly waive the transfer capability.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_waive_transfer">waive_transfer</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_waive_transfer">waive_transfer</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    <b>let</b> transfer_cap = &<b>mut</b> borrow_caps_mut(asset_owner, asset).transfer;
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(transfer_cap), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_fungible_source.md#0x1_managed_fungible_source_EFREEZE_CAP">EFREEZE_CAP</a>));
    <a href="fungible_caps.md#0x1_fungible_caps_destroy_transfer_cap">fungible_caps::destroy_transfer_cap</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(transfer_cap));
}
</code></pre>



</details>

<a name="0x1_managed_fungible_source_waive_burn"></a>

## Function `waive_burn`

Let asset owner to explicitly waive the burn capability.


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_waive_burn">waive_burn</a>&lt;T: key&gt;(asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_waive_burn">waive_burn</a>&lt;T: key&gt;(
    asset_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;
) <b>acquires</b> <a href="managed_fungible_source.md#0x1_managed_fungible_source_ManagingCapabilities">ManagingCapabilities</a> {
    <b>let</b> burn_cap = &<b>mut</b> borrow_caps_mut(asset_owner, asset).burn;
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(burn_cap), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_fungible_source.md#0x1_managed_fungible_source_EFREEZE_CAP">EFREEZE_CAP</a>));
    <a href="fungible_caps.md#0x1_fungible_caps_destroy_burn_cap">fungible_caps::destroy_burn_cap</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(burn_cap));
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
