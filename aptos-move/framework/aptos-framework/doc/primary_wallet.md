
<a name="0x1_primary_wallet"></a>

# Module `0x1::primary_wallet`

This defines the module for interacting with primary wallets of accounts/objects, which have deterministic addresses


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


<pre><code><b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a name="0x1_primary_wallet_ensure_primary_wallet_exists"></a>

## Function `ensure_primary_wallet_exists`



<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleAssetWallet&gt; {
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


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet">create_primary_wallet</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_create_primary_wallet">create_primary_wallet</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleAssetWallet&gt; {
    <a href="fungible_asset.md#0x1_fungible_asset_create_deterministic_wallet">fungible_asset::create_deterministic_wallet</a>(owner, metadata)
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
    <a href="fungible_asset.md#0x1_fungible_asset_deterministic_wallet_address">fungible_asset::deterministic_wallet_address</a>(owner, metadata)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_primary_wallet"></a>

## Function `primary_wallet`



<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleAssetWallet&gt; {
    <b>let</b> wallet = <a href="primary_wallet.md#0x1_primary_wallet_primary_wallet_address">primary_wallet_address</a>(owner, metadata);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;FungibleAssetWallet&gt;(wallet)
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


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: Object&lt;T&gt;, amount: u64): FungibleAsset {
    <b>let</b> wallet = <a href="primary_wallet.md#0x1_primary_wallet">primary_wallet</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw">fungible_asset::withdraw</a>(owner, wallet, amount)
}
</code></pre>



</details>

<a name="0x1_primary_wallet_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of fungible asset to the given account's primary wallet.


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_deposit">deposit</a>(owner: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_deposit">deposit</a>(owner: <b>address</b>, fa: FungibleAsset) {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">fungible_asset::asset_metadata</a>(&fa);
    <b>let</b> wallet = <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>(owner, metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(wallet, fa);
}
</code></pre>



</details>

<a name="0x1_primary_wallet_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset from sender's primary wallet to receiver's primary wallet.


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64, recipient: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_wallet.md#0x1_primary_wallet_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: Object&lt;T&gt;, amount: u64, recipient: <b>address</b>) {
    <b>let</b> sender_wallet = <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), metadata);
    <b>let</b> recipient_wallet = <a href="primary_wallet.md#0x1_primary_wallet_ensure_primary_wallet_exists">ensure_primary_wallet_exists</a>(recipient, metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_transfer">fungible_asset::transfer</a>(sender, sender_wallet, amount, recipient_wallet);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
