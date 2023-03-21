
<a name="0x1_fungible_caps"></a>

# Module `0x1::fungible_caps`



-  [Struct `MintCap`](#0x1_fungible_caps_MintCap)
-  [Struct `TransferCap`](#0x1_fungible_caps_TransferCap)
-  [Struct `BurnCap`](#0x1_fungible_caps_BurnCap)
-  [Constants](#@Constants_0)
-  [Function `init_fungible_source_with_caps`](#0x1_fungible_caps_init_fungible_source_with_caps)
-  [Function `mint`](#0x1_fungible_caps_mint)
-  [Function `set_ungated_transfer`](#0x1_fungible_caps_set_ungated_transfer)
-  [Function `burn`](#0x1_fungible_caps_burn)
-  [Function `withdraw`](#0x1_fungible_caps_withdraw)
-  [Function `withdraw_with_cap`](#0x1_fungible_caps_withdraw_with_cap)
-  [Function `deposit_with_cap`](#0x1_fungible_caps_deposit_with_cap)
-  [Function `transfer`](#0x1_fungible_caps_transfer)
-  [Function `transfer_with_cap`](#0x1_fungible_caps_transfer_with_cap)
-  [Function `destroy_mint_cap`](#0x1_fungible_caps_destroy_mint_cap)
-  [Function `destroy_transfer_cap`](#0x1_fungible_caps_destroy_transfer_cap)
-  [Function `destroy_burn_cap`](#0x1_fungible_caps_destroy_burn_cap)
-  [Function `asset_of_mint_cap`](#0x1_fungible_caps_asset_of_mint_cap)
-  [Function `asset_of_transfer_cap`](#0x1_fungible_caps_asset_of_transfer_cap)
-  [Function `asset_of_burn_cap`](#0x1_fungible_caps_asset_of_burn_cap)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="fungible_source.md#0x1_fungible_source">0x1::fungible_source</a>;
<b>use</b> <a href="fungible_store.md#0x1_fungible_store">0x1::fungible_store</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_fungible_caps_MintCap"></a>

## Struct `MintCap`

Capability to mint fungible asset.


<pre><code><b>struct</b> <a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_caps_TransferCap"></a>

## Struct `TransferCap`

Capability to control the transfer of fungible asset.


<pre><code><b>struct</b> <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_caps_BurnCap"></a>

## Struct `BurnCap`

Capability to burn fungible asset.


<pre><code><b>struct</b> <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_caps_ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH"></a>

The transfer cap and the the fungible asset do not match.


<pre><code><b>const</b> <a href="fungible_caps.md#0x1_fungible_caps_ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_caps_init_fungible_source_with_caps"></a>

## Function `init_fungible_source_with_caps`

The initialization of an object with <code>FungibleSource</code> with capabilities returned.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_init_fungible_source_with_caps">init_fungible_source_with_caps</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8): (<a href="fungible_caps.md#0x1_fungible_caps_MintCap">fungible_caps::MintCap</a>, <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>, <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">fungible_caps::BurnCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_init_fungible_source_with_caps">init_fungible_source_with_caps</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8,
): (<a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a>, <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>, <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a>) {
    <b>let</b> asset = init_fungible_source(constructor_ref, maximum_supply, name, symbol, decimals);
    (<a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a> { asset }, <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a> { asset }, <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a> { asset })
}
</code></pre>



</details>

<a name="0x1_fungible_caps_mint"></a>

## Function `mint`

Mint the <code>amount</code> of fungible asset with <code><a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_mint">mint</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_MintCap">fungible_caps::MintCap</a>, amount: u64, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_mint">mint</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a>, amount: u64, <b>to</b>: <b>address</b>) {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_mint">fungible_asset::mint</a>(&cap.asset, amount);
    <a href="fungible_store.md#0x1_fungible_store_deposit">fungible_store::deposit</a>(fa, <b>to</b>);
}
</code></pre>



</details>

<a name="0x1_fungible_caps_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Enable/disable the direct transfer of fungible asset with <code><a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">set_ungated_transfer</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">set_ungated_transfer</a>(
    cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    allow: bool,
) {
    <a href="fungible_store.md#0x1_fungible_store_set_ungated_transfer">fungible_store::set_ungated_transfer</a>(<a href="account.md#0x1_account">account</a>, &cap.asset, allow);
}
</code></pre>



</details>

<a name="0x1_fungible_caps_burn"></a>

## Function `burn`

Burn the <code>amount</code> of fungible asset from <code><a href="account.md#0x1_account">account</a></code> with a <code><a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_burn">burn</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_BurnCap">fungible_caps::BurnCap</a>, amount: u64, <a href="account.md#0x1_account">account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_burn">burn</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a>, amount: u64, <a href="account.md#0x1_account">account</a>: <b>address</b>) {
    <b>let</b> fa = <a href="fungible_store.md#0x1_fungible_store_withdraw">fungible_store::withdraw</a>(<a href="account.md#0x1_account">account</a>, &cap.asset, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_burn">fungible_asset::burn</a>(fa);
}
</code></pre>



</details>

<a name="0x1_fungible_caps_withdraw"></a>

## Function `withdraw`

Withdarw <code>amount</code> of fungible asset from <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_withdraw">withdraw</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_withdraw">withdraw</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &Object&lt;T&gt;, amount: u64): FungibleAsset {
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <a href="fungible_store.md#0x1_fungible_store_withdraw">fungible_store::withdraw</a>(account_address, &asset, amount)
}
</code></pre>



</details>

<a name="0x1_fungible_caps_withdraw_with_cap"></a>

## Function `withdraw_with_cap`

Withdarw <code>amount</code> of fungible asset from <code><a href="account.md#0x1_account">account</a></code> with <code><a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a></code> even ungated transfer is disabled.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_withdraw_with_cap">withdraw_with_cap</a>(transfer_cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_withdraw_with_cap">withdraw_with_cap</a>(transfer_cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): FungibleAsset {
    <b>let</b> ungated_transfer_allowed = <a href="fungible_store.md#0x1_fungible_store_ungated_transfer_allowed">fungible_store::ungated_transfer_allowed</a>(<a href="account.md#0x1_account">account</a>, &transfer_cap.asset);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">set_ungated_transfer</a>(transfer_cap, <a href="account.md#0x1_account">account</a>, <b>true</b>);
    };
    <b>let</b> fa = <a href="fungible_store.md#0x1_fungible_store_withdraw">fungible_store::withdraw</a>(<a href="account.md#0x1_account">account</a>, &transfer_cap.asset, amount);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">set_ungated_transfer</a>(transfer_cap, <a href="account.md#0x1_account">account</a>, <b>false</b>);
    };
    fa
}
</code></pre>



</details>

<a name="0x1_fungible_caps_deposit_with_cap"></a>

## Function `deposit_with_cap`

Deposit fungible asset into <code><a href="account.md#0x1_account">account</a></code> with <code><a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a></code> even ungated transfer is disabled.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_deposit_with_cap">deposit_with_cap</a>(transfer_cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_deposit_with_cap">deposit_with_cap</a>(transfer_cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>, fa: FungibleAsset, <a href="account.md#0x1_account">account</a>: <b>address</b>) {
    <b>assert</b>!(
        &transfer_cap.asset == &<a href="fungible_asset.md#0x1_fungible_asset_fungible_asset_source">fungible_asset::fungible_asset_source</a>(&fa),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_caps.md#0x1_fungible_caps_ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH</a>)
    );
    <b>let</b> ungated_transfer_allowed = <a href="fungible_store.md#0x1_fungible_store_ungated_transfer_allowed">fungible_store::ungated_transfer_allowed</a>(<a href="account.md#0x1_account">account</a>, &transfer_cap.asset);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">set_ungated_transfer</a>(transfer_cap, <a href="account.md#0x1_account">account</a>, <b>true</b>);
    };
    <a href="fungible_store.md#0x1_fungible_store_deposit">fungible_store::deposit</a>(fa, <a href="account.md#0x1_account">account</a>);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_caps.md#0x1_fungible_caps_set_ungated_transfer">set_ungated_transfer</a>(transfer_cap, <a href="account.md#0x1_account">account</a>, <b>false</b>);
    };
}
</code></pre>



</details>

<a name="0x1_fungible_caps_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset of <code>asset</code> to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_transfer">transfer</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, receiver: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_transfer">transfer</a>&lt;T: key&gt;(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset: &Object&lt;T&gt;,
    amount: u64,
    receiver: <b>address</b>
) {
    <b>let</b> asset = <a href="fungible_source.md#0x1_fungible_source_verify">fungible_source::verify</a>(asset);
    <b>let</b> fa = <a href="fungible_caps.md#0x1_fungible_caps_withdraw">withdraw</a>(<a href="account.md#0x1_account">account</a>, &asset, amount);
    <a href="fungible_store.md#0x1_fungible_store_deposit">fungible_store::deposit</a>(fa, receiver);
}
</code></pre>



</details>

<a name="0x1_fungible_caps_transfer_with_cap"></a>

## Function `transfer_with_cap`

Transfer <code>ammount</code> of  fungible asset with <code><a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a></code> even ungated transfer is disabled.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_transfer_with_cap">transfer_with_cap</a>(transfer_cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>, amount: u64, from: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_transfer_with_cap">transfer_with_cap</a>(
    transfer_cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>,
    amount: u64,
    from: <b>address</b>,
    <b>to</b>: <b>address</b>,
) {
    <b>let</b> fa = <a href="fungible_caps.md#0x1_fungible_caps_withdraw_with_cap">withdraw_with_cap</a>(transfer_cap, from, amount);
    <a href="fungible_caps.md#0x1_fungible_caps_deposit_with_cap">deposit_with_cap</a>(transfer_cap, fa, <b>to</b>);
}
</code></pre>



</details>

<a name="0x1_fungible_caps_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Explicitly destory <code><a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_destroy_mint_cap">destroy_mint_cap</a>(cap: <a href="fungible_caps.md#0x1_fungible_caps_MintCap">fungible_caps::MintCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_destroy_mint_cap">destroy_mint_cap</a>(cap: <a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a>) {
    <b>let</b> <a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a> { asset: _ } = cap;
}
</code></pre>



</details>

<a name="0x1_fungible_caps_destroy_transfer_cap"></a>

## Function `destroy_transfer_cap`

Explicitly destory <code><a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_destroy_transfer_cap">destroy_transfer_cap</a>(cap: <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_destroy_transfer_cap">destroy_transfer_cap</a>(cap: <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>) {
    <b>let</b> <a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a> { asset: _ } = cap;
}
</code></pre>



</details>

<a name="0x1_fungible_caps_destroy_burn_cap"></a>

## Function `destroy_burn_cap`

Explicitly destory <code><a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_destroy_burn_cap">destroy_burn_cap</a>(cap: <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">fungible_caps::BurnCap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_destroy_burn_cap">destroy_burn_cap</a>(cap: <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a>) {
    <b>let</b> <a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a> { asset: _ } = cap;
}
</code></pre>



</details>

<a name="0x1_fungible_caps_asset_of_mint_cap"></a>

## Function `asset_of_mint_cap`

Get the underlying asset object from <code><a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_asset_of_mint_cap">asset_of_mint_cap</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_MintCap">fungible_caps::MintCap</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_asset_of_mint_cap">asset_of_mint_cap</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_MintCap">MintCap</a>): Object&lt;FungibleSource&gt; {
    cap.asset
}
</code></pre>



</details>

<a name="0x1_fungible_caps_asset_of_transfer_cap"></a>

## Function `asset_of_transfer_cap`

Get the underlying asset object from <code><a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_asset_of_transfer_cap">asset_of_transfer_cap</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">fungible_caps::TransferCap</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_asset_of_transfer_cap">asset_of_transfer_cap</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_TransferCap">TransferCap</a>): Object&lt;FungibleSource&gt; {
    cap.asset
}
</code></pre>



</details>

<a name="0x1_fungible_caps_asset_of_burn_cap"></a>

## Function `asset_of_burn_cap`

Get the underlying asset object from <code><a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_asset_of_burn_cap">asset_of_burn_cap</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_BurnCap">fungible_caps::BurnCap</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_caps.md#0x1_fungible_caps_asset_of_burn_cap">asset_of_burn_cap</a>(cap: &<a href="fungible_caps.md#0x1_fungible_caps_BurnCap">BurnCap</a>): Object&lt;FungibleSource&gt; {
    cap.asset
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
