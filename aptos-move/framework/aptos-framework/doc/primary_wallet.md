
<a name="0x1_primary_wallet"></a>

# Module `0x1::primary_wallet`

This defines the module for interacting with primary wallets of accounts/objects, which have deterministic addresses


-  [Resource `PrimaryWalletSupport`](#0x1_primary_wallet_PrimaryWalletSupport)
-  [Function `create_primary_wallet_enabled_fungible_asset`](#0x1_primary_wallet_create_primary_wallet_enabled_fungible_asset)
-  [Function `ensure_primary_wallet_exists`](#0x1_primary_wallet_ensure_primary_wallet_exists)
-  [Function `create_primary_wallet`](#0x1_primary_wallet_create_primary_wallet)
-  [Function `primary_wallet_address`](#0x1_primary_wallet_primary_wallet_address)
-  [Function `primary_wallet`](#0x1_primary_wallet_primary_wallet)
-  [Function `primary_wallet_exists`](#0x1_primary_wallet_primary_wallet_exists)
-  [Function `balance`](#0x1_primary_wallet_balance)
-  [Function `ungated_transfer_allowed`](#0x1_primary_wallet_ungated_transfer_allowed)
-  [Function `withdraw`](#0x1_primary_wallet_withdraw)
-  [Function `deposit`](#0x1_primary_wallet_deposit)
-  [Function `transfer`](#0x1_primary_wallet_transfer)
-  [Specification](#@Specification_0)


<pre><code><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_primary_wallet_PrimaryWalletSupport"></a>

## Resource `PrimaryWalletSupport`

Resource stored on the fungible asset metadata object to allow creating primary wallets for it.


<pre><code><b>struct</b> <a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata_derive_ref: <a href="object.md#0x1_object_DeriveRef">object::DeriveRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_primary_wallet_create_primary_wallet_enabled_fungible_asset"></a>

## Function `create_primary_wallet_enabled_fungible_asset`

Creators of fungible assets can call this to enable support for creating primary (deterministic) wallets for
their users.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet_enabled_fungible_asset">create_primary_wallet_enabled_fungible_asset</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet_enabled_fungible_asset">create_primary_wallet_enabled_fungible_asset</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8,
) {
    <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">fungible_asset::add_fungibility</a>(constructor_ref, maximum_supply, name, symbol, decimals);
    <b>let</b> metadata_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(metadata_obj, <a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a> {
        metadata_derive_ref: <a href="object.md#0x1_object_generate_derive_ref">object::generate_derive_ref</a>(constructor_ref),
    });
}
</code></pre>



</details>

<a name="0x1_primary_wallet_ensure_primary_wallet_exists"></a>

## Function `ensure_primary_wallet_exists`



<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>&lt;T: key&gt;(
    owner: <b>address</b>,
    metadata: Object&lt;T&gt;,
): Object&lt;FungibleAsset&gt; <b>acquires</b> <a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a> {
    <b>if</b> (!<a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_exists">primary_wallet_exists</a>(owner, metadata)) {
        <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet">create_primary_wallet</a>(owner, metadata);
    };
    <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>(owner, metadata)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_create_primary_wallet"></a>

## Function `create_primary_wallet`

Create a primary wallet object to hold fungible asset for the given address.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet">create_primary_wallet</a>&lt;T: key&gt;(owner_addr: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet">create_primary_wallet</a>&lt;T: key&gt;(
    owner_addr: <b>address</b>,
    metadata: Object&lt;T&gt;,
): Object&lt;FungibleAsset&gt; <b>acquires</b> <a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a> {
    <b>let</b> owner = &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(owner_addr);
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <b>let</b> derive_ref = &<b>borrow_global</b>&lt;<a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a>&gt;(metadata_addr).metadata_derive_ref;
    <b>let</b> constructor_ref = &<a href="object.md#0x1_object_create_user_derived_object">object::create_user_derived_object</a>(owner, derive_ref);

    // Disable ungated transfer <b>as</b> deterministic wallets shouldn't be transferrable.
    <b>let</b> transfer_ref = &<a href="object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(constructor_ref);
    <a href="object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(transfer_ref);

    <a href="fungible_asset.md#0x1_fungible_asset_create_wallet">fungible_asset::create_wallet</a>(constructor_ref, metadata)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_primary_wallet_address"></a>

## Function `primary_wallet_address`



<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_address">primary_wallet_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_address">primary_wallet_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): <b>address</b> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(owner, metadata_addr)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_primary_wallet"></a>

## Function `primary_wallet`



<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleAsset&gt; {
    <b>let</b> wallet = <a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_address">primary_wallet_address</a>(owner, metadata);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;FungibleAsset&gt;(wallet)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_primary_wallet_exists"></a>

## Function `primary_wallet_exists`



<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_exists">primary_wallet_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_exists">primary_wallet_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
    <a href="fungible_asset.md#0x1_fungible_asset_wallet_exists">fungible_asset::wallet_exists</a>(<a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_address">primary_wallet_address</a>(<a href="account.md#0x1_account">account</a>, metadata))
}
</code></pre>



</details>

<a name="0x1_primary_wallet_balance"></a>

## Function `balance`

Get the balance of <code><a href="account.md#0x1_account">account</a></code>'s primary wallet.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): u64 {
    <b>if</b> (<a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_exists">primary_wallet_exists</a>(<a href="account.md#0x1_account">account</a>, metadata)) {
        <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>(<a href="account.md#0x1_account">account</a>, metadata))
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a name="0x1_primary_wallet_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Return whether the given account's primary wallet can do direct transfers.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
    <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">fungible_asset::ungated_transfer_allowed</a>(<a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>(<a href="account.md#0x1_account">account</a>, metadata))
}
</code></pre>



</details>

<a name="0x1_primary_wallet_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible asset from <code>wallet</code> by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_ExtractedAsset">fungible_asset::ExtractedAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: Object&lt;T&gt;, amount: u64): ExtractedAsset {
    <b>let</b> wallet = <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw">fungible_asset::withdraw</a>(owner, wallet, amount)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of fungible asset to the given account's primary wallet.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_deposit">deposit</a>(owner: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_ExtractedAsset">fungible_asset::ExtractedAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_deposit">deposit</a>(owner: <b>address</b>, fa: ExtractedAsset) <b>acquires</b> <a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">fungible_asset::asset_metadata</a>(&fa);
    <b>let</b> wallet = <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>(owner, metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(wallet, fa);
}
</code></pre>



</details>

<a name="0x1_primary_wallet_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset from sender's primary wallet to receiver's primary wallet.


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_transfer">transfer</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: Object&lt;T&gt;,
    recipient: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="primary_wallet.md#0x1_primary_wallet_PrimaryWalletSupport">PrimaryWalletSupport</a> {
    <b>let</b> sender_wallet = <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), metadata);
    <b>let</b> recipient_wallet = <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>(recipient, metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_transfer">fungible_asset::transfer</a>(sender, sender_wallet, recipient_wallet, amount);
}
</code></pre>



</details>

<a name="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
